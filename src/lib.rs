use crate::commit::Commit;
use colored::Colorize;
use filelist::FileList;
use fs_err as fs;
use std::{
    cmp::{max, min},
    collections::HashMap,
    env, fmt,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
};

pub mod commit;
mod rename_detection;

/// Alias for a `Result` with the error type [`crate::Error`]
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),

    #[error("fatal: not inside an fl repository (or any of the parent directories): `{0}`")]
    RepoNotFound(PathBuf),

    #[error("fatal: Invalid commit index: {index} (must be between {min} and {max})", min = -max-1)]
    CommitNotFound { index: i32, max: i32 },

    #[error("Failed to parse commit from json")]
    CommitError(#[from] commit::CommitError),

    #[error("Command not found: `{0}`")]
    CommandNotFound(String),

    #[error("Commit Editor exited with non-zero status: {0:?}")]
    CommitEditorFailed(Option<i32>),
}

/// Represents a change detected between two file snapshots.
///
/// Each variant describes a type of file change:
/// * `Add` — the file was added
/// * `Remove` — the file was deleted
/// * `Modify` — the file content changed
/// * `Rename` — the file was moved or renamed
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action<'a> {
    Add(&'a Path),
    Remove(&'a Path),
    Rename(&'a Path, &'a Path),
    Modify(&'a Path),
}

impl<'a> Action<'a> {
    fn colored(&self) -> String {
        match self {
            Action::Add(path) => format!("{A}  {path}", A = "A".green(), path = path.display()),
            Action::Remove(path) => format!("{D}  {path}", D = "D".red(), path = path.display()),
            Action::Modify(path) => format!("{M}  {path}", M = "M".yellow(), path = path.display()),
            Action::Rename(from, to) => {
                format!(
                    r#"{R}  "{from}" -> "{to}""#,
                    R = "R".blue(),
                    from = from.display(),
                    to = to.display()
                )
            }
        }
    }

    fn sort_key(&self) -> (&Path, u8) {
        match self {
            Action::Add(p) => (p, 0),
            Action::Remove(p) => (p, 1),
            Action::Modify(p) => (p, 2),
            Action::Rename(_, to) => (to, 3),
        }
    }
}

impl<'a> fmt::Display for Action<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Add(path) => write!(f, "A  {}", path.display()),
            Action::Remove(path) => write!(f, "D  {}", path.display()),
            Action::Modify(path) => write!(f, "M  {}", path.display()),
            Action::Rename(from, to) => {
                write!(f, r#"R  "{}" -> "{}""#, from.display(), to.display())
            }
        }
    }
}

impl<'a> PartialOrd for Action<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Action<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FL {
    /// path to directory containing `.fl` folder
    root: PathBuf,
    /// number of commits, last commit is `commits - 1`
    commits: i32,
    // TODO: make this part of `config` field, when i will implement config fields
    /// if true, add `diff` commands will ignore modifications on directories
    pub ignore_dir_modifications: bool,
}

// Constructors
impl FL {
    /// Creates a new [`FL`] instance for an existing repository root.
    ///
    /// This does not create any folders. It only initializes the struct
    /// and scans the `.fl/history` directory to determine the current
    /// number of commits.
    ///
    /// # Arguments
    /// * `root` - Path to the directory containing the `.fl` folder.
    pub fn new(root: PathBuf) -> io::Result<FL> {
        let mut fl = FL {
            root,
            commits: 0,
            ignore_dir_modifications: false,
        };
        fl.update_commits()?;
        Ok(fl)
    }

    /// Creates a new `FL` instance by locating a `.fl` repository
    /// starting from the current directory and walking up parent directories.
    pub fn in_current_dir() -> Result<FL> {
        let fl = FL::new(FL::find_root_path()?)?;
        Ok(fl)
    }

    /// Initializes a new `.fl` repository at the given path.
    ///
    /// This creates the required `.fl` and `.fl/history` directories,
    /// then returns an initialized [`FL`] instance.
    ///
    /// # Arguments
    /// * `root` - Directory where the repository should be created.
    pub fn create_fl_repo(root: PathBuf) -> io::Result<FL> {
        fs::create_dir(root.join(".fl"))?;
        fs::create_dir(root.join(".fl").join("history"))?;
        Commit::default().save_to(root.join(".fl").join("STAGE.json"))?;

        FL::new(root)
    }

    /// Initializes a new `.fl` repository in the current working directory.
    ///
    /// This is a convenience wrapper around [`FL::create_fl_repo`].
    pub fn init() -> io::Result<FL> {
        FL::create_fl_repo(env::current_dir()?)
    }
}

// getters
impl FL {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn commits(&self) -> i32 {
        self.commits
    }
}

// public methods
impl FL {
    /// Creates a new snapshot of the current state of the repository.
    ///
    /// This generates a file list for the root directory and writes it
    /// to a new history file with the next commit index.
    pub fn update(&self) -> io::Result<()> {
        let mut fl = self.get_filelist();
        let mut commit = commit::Commit::default();
        let output = self.stage_path();
        println!("Updating {}", self.root.display());

        commit.snapshot = fl.hash_paths(&[&self.root]);

        commit.save_to(output)
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
    pub fn diff_history(&self, a: i32, b: i32) -> Result<()> {
        let valid_a = self.to_valid_history_index(a)?;
        let valid_b = self.to_valid_history_index(b)?;

        // always compare older commit to newer commit
        let first = min(valid_a, valid_b);
        let second = max(valid_a, valid_b);

        println!("Diffing {first} and {second}");
        self.diff_paths(
            &self.history_file_path(first),
            &self.history_file_path(second),
        )?;
        Ok(())
    }

    /// Compares the current staged snapshot (`STAGE.json`) with a commit.
    ///
    /// # Arguments
    /// * `commit` - Index of the commit to compare against.
    ///
    /// Special cases:
    /// * If there are no commits and `commit` is `-1` or `0`, the stage is diffed against an empty snapshot.
    ///
    /// Output format:
    /// * `A` - Added file
    /// * `D` - Deleted file
    /// * `M` - Modified file
    /// * `R` - Renamed/moved file
    pub fn diff_stage(&self, commit: i32) -> Result<()> {
        let stage = Commit::from_path(self.stage_path())?;
        // if there are no commits, if user gave -1 or 0, diff against empty commit
        let target_commit = if self.commits == 0 && [-1, 0].contains(&commit) {
            println!("Diffing EMPTY and STAGE");
            Commit::default()
        } else {
            let valid_commit = self.to_valid_history_index(commit)?;
            println!("Diffing {valid_commit} and STAGE");
            Commit::from_path(self.history_file_path(valid_commit))?
        };
        let actions = FL::diff_commit(&target_commit, &stage);
        self.print_actions(&actions);
        Ok(())
    }

    /// Commit the STAGE file, without a commit message
    pub fn commit_empty(&mut self) -> Result<()> {
        let mut stage = Commit::from_path(self.stage_path())?;
        stage.set_timestamp_now();
        self.commit_commit(&stage)
    }

    /// Commit the STAGE file, with a commit message
    /// First line of the message will be used as the title, the rest as the body
    pub fn commit_message(&mut self, message: &str) -> Result<()> {
        let (title, body) = message
            .split_once('\n')
            .map(|(t, b)| (t.trim_end(), b.trim()))
            .unwrap_or((message, ""));

        self.commit_title_body(title.to_string(), body.to_string())
    }

    /// Commit the STAGE file, with a title and body
    pub fn commit_title_body(&mut self, title: String, body: String) -> Result<()> {
        let mut stage = Commit::from_path(self.stage_path())?;
        stage.title = Some(title);
        stage.body = Some(body);
        stage.set_timestamp_now();
        self.commit_commit(&stage)
    }

    /// Commit the STAGE file, but open an editor to write a commit message
    pub fn commit_interactive(&mut self) -> Result<()> {
        // these should also be config options
        let ask_confirmation = true;
        let editor = env::var("EDITOR").unwrap_or("vim".to_string());

        let mut path = env::temp_dir();
        path.push("FL_COMMIT_MESSAGE");

        fs::write(
            &path,
            "# Write your commit message. Lines starting with '#' will be ignored.",
        )?;

        let status = process::Command::new(&editor)
            .arg(&path)
            .status()
            .map_err(|_| Error::CommandNotFound(editor.to_string()))?;

        if !status.success() {
            return Err(Error::CommitEditorFailed(status.code()));
        }

        if ask_confirmation {
            print!("Press enter to commit: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut String::new())?;
        }

        let content = fs::read_to_string(&path)?;

        let _ = fs::remove_file(&path);

        let cleaned = content
            .lines()
            // filter out comments
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if cleaned.is_empty() {
            self.commit_empty()
        } else {
            self.commit_message(&cleaned)
        }
    }

    pub fn print_short_log(&self) -> Result<()> {
        // TODO: these should be config options
        let print_title = true;
        let print_title_quotes = false;
        let print_number_of_changes = false;
        let print_time_ago = true;

        for i in 0..self.commits {
            let path = self.history_file_path(i);
            let commit = Commit::from_path(&path)?;

            let title = match print_title {
                true => commit.title.as_deref().unwrap_or("No commit message"),
                false => "",
            };

            let title_quotes = match print_title_quotes {
                true => "\"",
                false => "",
            };

            let changes = match print_number_of_changes {
                true => {
                    let prev_commit = if i > 0 {
                        Commit::from_path(self.history_file_path(i - 1))?
                    } else {
                        Commit::default()
                    };
                    let num_changes = FL::diff_commit(&prev_commit, &commit).len();
                    format!(" ({num_changes} Changes)")
                }
                false => "".to_string(),
            };

            let time_ago = match print_time_ago {
                true => format!(" ({})", commit.time_ago()),
                false => "".to_string(),
            };

            println!("{i}: {title_quotes}{title}{title_quotes}{changes}{time_ago}");
        }

        Ok(())
    }
}

// private methods
impl FL {
    /// Recomputes the number of commits by scanning the history directory.
    ///
    /// This reads all files in `.fl/history`, parses their numeric names,
    /// and sets `self.commits` to the next available index.
    ///
    /// Invalid filenames are ignored with a warning.
    fn update_commits(&mut self) -> io::Result<()> {
        let history_path = self.history_path();

        let commits = fs::read_dir(&history_path)?
            .filter_map(Result::ok) // ignore read errors
            // convert file name to string
            .filter_map(|e| {
                e.file_name()
                    .into_string()
                    .inspect_err(|os_str| self.warn_invalid_history(os_str.display()))
                    .ok()
            })
            // remove ".json" extension
            .filter_map(|s| {
                let no_extension = s.strip_suffix(".json");
                match no_extension {
                    Some(s) => Some(s.to_string()),
                    None => {
                        self.warn_invalid_history(s);
                        None
                    }
                }
            })
            // convert string to number
            .filter_map(|s| match s.parse::<u32>() {
                Ok(n) => Some(n),
                Err(_) => {
                    self.warn_invalid_history(s);
                    None
                }
            })
            .max()
            .map_or(0, |n| n + 1);

        self.commits = commits as i32;

        Ok(())
    }

    fn warn_invalid_history(&self, path: impl fmt::Display) {
        eprintln!(
            "{}: invalid history file name: '{}' ({})",
            "WARNING".yellow(),
            path,
            self.history_path().join(path.to_string()).display()
        );
    }

    /// This will commit a [`commit::Commit`] object
    fn commit_commit(&mut self, commit: &Commit) -> Result<()> {
        let out_path = self.history_file_path(self.commits);

        // if there is a previous commit, diff against it
        let prev_commit = if self.commits > 0 {
            Commit::from_path(self.history_file_path(self.commits - 1))?
        } else {
            Commit::default()
        };

        let changes = FL::diff_commit(&prev_commit, commit).len();
        println!("Committing {} changes", changes);

        commit.save_to(out_path)?;
        self.commits += 1;
        Ok(())
    }

    fn diff_paths(&self, old: &Path, new: &Path) -> Result<(), commit::CommitError> {
        let old = Commit::from_path(old)?;
        let new = Commit::from_path(new)?;
        let actions = FL::diff_commit(&old, &new);
        self.print_actions(&actions);
        Ok(())
    }

    fn print_actions(&self, actions: &[Action]) {
        if actions.is_empty() {
            println!("No changes");
            return;
        }

        for action in actions {
            if self.ignore_dir_modifications
                && let Action::Modify(path) = action
                && PathBuf::from(path).is_dir()
            {
                continue;
            }
            println!("{}", action.colored());
        }
    }

    fn diff_commit<'a>(old: &'a Commit, new: &'a Commit) -> Vec<Action<'a>> {
        // convert `BTreeMap<PathBuf, String>` to `HashMap<&Path, &str>`
        let old_by_path: HashMap<&Path, &str> =
            HashMap::from_iter(old.snapshot.iter().map(|(k, v)| (k.as_path(), v.as_str())));
        let new_by_path: HashMap<&Path, &str> =
            HashMap::from_iter(new.snapshot.iter().map(|(k, v)| (k.as_path(), v.as_str())));

        FL::diff_map(&old_by_path, &new_by_path)
    }

    /// Computes the differences between two snapshots of files and returns a list of actions.
    ///
    /// # Arguments
    /// * `old` — previous snapshot, maps paths (keys) to hashes (values)
    /// * `new` — current snapshot, maps paths (keys) to hashes (values)
    ///
    /// # Returns
    /// A sorted list of [`Action`] describing the differences.
    fn diff_map<'a>(
        old: &HashMap<&'a Path, &'a str>,
        new: &HashMap<&'a Path, &'a str>,
    ) -> Vec<Action<'a>> {
        // Paths that were deleted are stored here by hash, for rename detection.
        // only path that are missing come here, modifications never enter this map
        let mut deleted_by_hash: HashMap<&str, Vec<&Path>> = HashMap::new();
        let mut actions: Vec<Action> = Vec::new();

        for (path, old_hash) in old {
            match new.get(path) {
                None => deleted_by_hash.entry(old_hash).or_default().push(path),
                Some(new_hash) if new_hash != old_hash => actions.push(Action::Modify(path)),
                _ => {}
            }
        }

        // A hash from `deleted_by_hash` as key, a list of new paths with that hash as value.
        let mut rename_candidates: HashMap<&str, Vec<&Path>> = HashMap::new();

        for (path, new_hash) in new {
            if old.contains_key(path) {
                // Path existed before and is still present — already handled above.
                continue;
            }
            // if file was "deleted" but then "added" with a different name, it is a rename
            if deleted_by_hash.contains_key(new_hash) {
                rename_candidates.entry(new_hash).or_default().push(path);
            } else {
                actions.push(Action::Add(path));
            }
        }

        // Match each group of rename candidates against the pool of deleted paths that share the same hash.
        for (hash, mut new_paths) in rename_candidates {
            // unwrap: key is guaranteed to exist because we only inserted into
            // rename_candidates when deleted_by_hash contained the same hash.
            let deleted_paths = deleted_by_hash.get_mut(hash).unwrap();

            let pairings = rename_detection::optimal_pairings(deleted_paths, &new_paths);

            // for (new_path, deleted_path) in pairings {
            for (deleted_path, new_path) in pairings {
                actions.push(Action::Rename(deleted_path, new_path));
                // remove `new_path` from `new_paths`, because it is now claimed by rename
                // the order doesn't matter, since i don't index new_paths, so use swap_remove (very fast)
                new_paths.swap_remove(new_paths.iter().position(|p| p == &new_path).unwrap());
                // same as above
                deleted_paths.swap_remove(
                    deleted_paths
                        .iter()
                        .position(|p| p == &deleted_path)
                        .unwrap(),
                );
            }
            // Any new path that was not claimed by a rename is a true addition
            for path in new_paths {
                actions.push(Action::Add(path));
            }
            // if i removed all paths, might as well remove the hash from deleted_by_hash
            // since I know I will never do deleted_paths[hash] again, I can remove it (not required though)
            if deleted_paths.is_empty() {
                deleted_by_hash.remove(hash);
            }
        }

        // Any deleted path that was not claimed by a rename is a true deletion.
        for deleted_paths in deleted_by_hash.values() {
            for deleted_path in deleted_paths {
                actions.push(Action::Remove(deleted_path));
            }
        }

        // sort actions by path
        actions.sort();
        actions
    }

    fn fl_path(&self) -> PathBuf {
        self.root.join(".fl")
    }

    fn history_path(&self) -> PathBuf {
        self.fl_path().join("history")
    }

    fn stage_path(&self) -> PathBuf {
        self.fl_path().join("STAGE.json")
    }

    /// returns the path to a history file based on its index
    /// This does not check if the file exists, it just converts a number to a path
    /// To get a valid path, use `to_valid_history_index`
    fn history_file_path(&self, index: i32) -> PathBuf {
        self.history_path().join(format!("{index:08}.json"))
    }

    /// returns a valid commit index
    /// This will always return a positive number
    ///
    /// # Panics
    /// Panics if index is out of bounds
    fn to_valid_history_index(&self, index: i32) -> Result<i32> {
        match index {
            n if 0 <= n && n < self.commits => Ok(index),
            n if -self.commits <= n && n < 0 => Ok(self.commits + n),
            _ => Err(Error::CommitNotFound {
                index,
                max: self.commits - 1,
            }),
        }
    }

    fn find_root_path() -> Result<PathBuf> {
        // let dir = env::current_dir().context("Failed to get current directory")?;
        let dir = env::current_dir()?;

        FL::find_fl_path(dir.clone()).ok_or(Error::RepoNotFound(dir))
        // .context("fatal: not inside an fl repository (or any of the parent directories)")
    }

    fn find_fl_path(mut dir: PathBuf) -> Option<PathBuf> {
        loop {
            // if dir contains `.fl` folder, return it
            if dir.join(".fl").is_dir() {
                return Some(dir);
            }

            // go one level up, or if there are no more parents then return None
            if !dir.pop() {
                return None;
            }
        }
    }

    /// returns a pre-configured filelist for this repo
    fn get_filelist(&self) -> FileList {
        let mut fl = FileList::new();
        fl.set_hash_length(64); // show 64 chars of hash, to minimize chance of collision
        fl.hasher_mut().set_all(false); // don't track hidden files
        fl.hasher_mut().set_hash_directory(true); // track directories
        fl.set_relative_to(&self.root); // output everything relative to the root, so that this works even if root folder is moved
        fl.set_use_progress_bar(true); // show progress bar, so that user knows how much to wait
        fl
    }
}

#[cfg(test)]
mod tests;
