use filelist::FileList;
use std::{
    cmp::{max, min},
    collections::HashMap,
    path::{Path, PathBuf},
};

mod fs_helper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FL {
    /// path to directory containing `.fl` folder
    root: PathBuf,
    /// number of commits, last commit is `commits - 1`
    commits: i32,
}

// Constructors
impl FL {
    /// Creates a new `FL` instance for an existing repository root.
    ///
    /// This does not create any folders. It only initializes the struct
    /// and scans the `.fl/history` directory to determine the current
    /// number of commits.
    ///
    /// # Arguments
    /// * `root` - Path to the directory containing the `.fl` folder.
    ///
    /// # Panics
    /// May panic if filesystem helpers fail internally.
    pub fn new(root: PathBuf) -> Self {
        let mut fl = FL { root, commits: 0 };
        fl.update_commits();
        fl
    }

    /// Creates a new `FL` instance by locating a `.fl` repository
    /// starting from the current directory and walking up parent directories.
    ///
    /// This is similar to how tools like Git locate repositories.
    ///
    /// # Panics
    /// May panic if no `.fl` directory is found or filesystem helpers fail.
    pub fn in_current_dir() -> Self {
        FL::new(fs_helper::find_root_path())
    }

    /// Initializes a new `.fl` repository at the given path.
    ///
    /// This creates the required `.fl` and `.fl/history` directories,
    /// then returns an initialized `FL` instance.
    ///
    /// # Arguments
    /// * `root` - Directory where the repository should be created.
    pub fn create_fl_repo(root: PathBuf) -> Self {
        fs_helper::create_dir(root.join(".fl"));
        fs_helper::create_dir(root.join(".fl").join("history"));

        FL::new(root)
    }

    /// Initializes a new `.fl` repository in the current working directory.
    ///
    /// This is a convenience wrapper around [`FL::create_fl_repo`].
    pub fn init() -> Self {
        FL::create_fl_repo(fs_helper::current_dir())
    }
}

// public methods
impl FL {
    /// Recomputes the number of commits by scanning the history directory.
    ///
    /// This reads all files in `.fl/history`, parses their numeric names,
    /// and sets `self.commits` to the next available index.
    ///
    /// Invalid filenames are ignored with a warning.
    pub fn update_commits(&mut self) {
        let history_path = self.history_folder_path();

        let commits = fs_helper::read_dir(&history_path)
            .filter_map(Result::ok) // ignore read errors
            .filter_map(|e| e.file_name().into_string().ok()) // convert file name to string
            .filter_map(|s| {
                // convert string to number
                s.parse::<u32>()
                    .inspect_err(|_| {
                        eprintln!(
                            "WARNING: invalid history file name: '{}' ({})",
                            s,
                            history_path.join(&s).display()
                        )
                    })
                    .ok()
            })
            .max()
            .map_or(0, |n| n + 1);

        self.commits = commits as i32;
    }

    /// Creates a new snapshot of the current state of the repository.
    ///
    /// This generates a file list for the root directory and writes it
    /// to a new history file with the next commit index.
    ///
    /// # Panics
    /// Panics if file listing fails.
    pub fn update(&mut self) {
        let out_path = self.history_file_path(self.commits);
        let mut fl = self.get_filelist();
        // fl.set_output(Some(root.join(".fl/STAGE")));
        fl.set_output(Some(out_path));

        println!("Updating {}", self.root.display());
        fl.run(vec![self.root.clone()]).unwrap();
        self.commits += 1;
    }

    /// Compares two history snapshots and prints their differences.
    ///
    /// # Arguments
    /// * `first` - Index of the first history file.
    /// * `second` - Index of the second history file.
    ///
    /// Output format:
    /// * `A` - Added file
    /// * `D` - Deleted file
    /// * `M` - Modified file
    /// * `R` - Renamed/moved file
    pub fn diff_history(&self, a: i32, b: i32) {
        let valid_a = self.to_valid_history_index(a);
        let valid_b = self.to_valid_history_index(b);

        // always compare older commit to newer commit
        let first = min(valid_a, valid_b);
        let second = max(valid_a, valid_b);

        println!("Diffing {first} and {second}");
        FL::diff_paths(
            &self.history_file_path(first),
            &self.history_file_path(second),
        );
    }
}

// private methods
impl FL {
    fn diff_paths(first: &Path, second: &Path) {
        let content1 = fs_helper::read_to_string(first);
        let content2 = fs_helper::read_to_string(second);

        let old_by_path: HashMap<&str, &str> = fs_helper::parse_filelist(&content1);
        let new_by_path: HashMap<&str, &str> = fs_helper::parse_filelist(&content2);
        let mut deleted_by_hash: HashMap<&str, Vec<&str>> = HashMap::new();

        // detect deletions and modifications
        for (path, old_hash) in &old_by_path {
            match new_by_path.get(path) {
                None => deleted_by_hash.entry(old_hash).or_default().push(path),
                Some(new_hash) if new_hash != old_hash => println!("M  {path}"),
                _ => {}
            }
        }

        // detect additions
        for (path, new_hash) in &new_by_path {
            if old_by_path.contains_key(path) {
                continue;
            // if I "deleted" a file, but also "added" exactly the same file but with different path
            // then it is just a rename / move
            } else if let Some(deleted_paths) = deleted_by_hash.get_mut(new_hash)
                && let Some(deleted_path) = deleted_paths.pop()
            {
                println!(r#"R  "{deleted_path}" -> "{path}""#);
            } else {
                println!("A  {path}");
            }
        }
        for paths in deleted_by_hash.values() {
            for path in paths {
                println!("D  {path}");
            }
        }
    }

    fn history_folder_path(&self) -> PathBuf {
        self.root.join(".fl").join("history")
    }

    /// returns the path to a history file based on its index
    /// This does not check if the file exists, it just converts a number to a path
    /// To get a valid path, use `to_valid_history_index`
    fn history_file_path(&self, index: i32) -> PathBuf {
        self.root
            .join(".fl")
            .join("history")
            .join(format!("{index:08}"))
    }

    /// returns a valid commit index
    /// This will always return a positive number
    ///
    /// # Panics
    /// Panics if index is out of bounds
    fn to_valid_history_index(&self, index: i32) -> i32 {
        match index {
            n if 0 <= n && n < self.commits => index,
            n if -self.commits <= n && n < 0 => self.commits + n,
            _ => {
                eprintln!(
                    "fatal: Invalid commit index: {} (must be between {} and {})",
                    index,
                    -self.commits,
                    self.commits - 1
                );
                std::process::exit(2);
            }
        }
    }

    /// returns a pre-configured filelist for this repo
    fn get_filelist(&self) -> FileList {
        let mut fl = FileList::new();
        fl.set_hash_length(64); // show 64 chars of hash, to minimize chance of collision
        fl.set_sep('\t'); // use tab as separator
        fl.set_all(false); // don't track hidden files
        fl.set_hash_directory(true); // track directories
        fl.set_relative_to(&self.root); // output everything relative to the root, so that this works even if root folder is moved
        fl.set_use_progress_bar(true); // show progress bar, so that user knows how much to wait
        fl.set_force(true); // replace output file if it already exists
        fl
    }
}

// TODO: make this `FL` struct that stores needed info like root folder and history indexes
// TODO: so its going to work like this:
// update creates a .fl/STAGE file
// commit copies the .fl/STAGE file to .fl/history/
// something like "fl st" will compare the STAGE file to the latest history file
// "fl diff" will compare 2 history files together OR
// "fl diff" will compare given history file to the STAGE OR
// "fl diff" will compare most recent history file to the STAGE
// you can give history files as numbers like (1 instead of 00000001)
// you can also give history numbers as negative (-1 means last history file, -2 means second to last history file)
