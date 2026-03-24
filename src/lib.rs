use colored::Colorize;
use filelist::FileList;
use std::{
    cmp::{max, min},
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

use crate::fs_helper::FILELIST_MESSAGE_SEP;

mod fs_helper;
mod rename_detection;

const RENAME_GROUP_SIZE_LIMIT: usize = 8;

/// Represents a change detected between two file snapshots.
///
/// Each variant describes a type of file change:
/// * `Add` — the file was added
/// * `Remove` — the file was deleted
/// * `Modify` — the file content changed
/// * `Rename` — the file was moved or renamed
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action<'a> {
    Add(&'a str),
    Remove(&'a str),
    Rename(&'a str, &'a str),
    Modify(&'a str),
}

impl<'a> Action<'a> {
    fn colored(&self) -> String {
        match self {
            Action::Add(path) => format!("{}  {path}", "A".green()),
            Action::Remove(path) => format!("{}  {path}", "D".red()),
            Action::Modify(path) => format!("{}  {path}", "M".yellow()),
            Action::Rename(from, to) => {
                format!(r#"{}  "{}" -> "{}""#, "R".magenta(), from.red(), to)
            }
        }
    }

    fn sort_key(&self) -> (&str, u8) {
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
            Action::Add(path) => write!(f, "A  {path}"),
            Action::Remove(path) => write!(f, "D  {path}"),
            Action::Modify(path) => write!(f, "M  {path}"),
            Action::Rename(from, to) => {
                write!(f, r#"R  "{from}" -> "{to}""#)
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
        fs_helper::create_file(root.join(".fl").join("STAGE"));

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
    /// Creates a new snapshot of the current state of the repository.
    ///
    /// This generates a file list for the root directory and writes it
    /// to a new history file with the next commit index.
    ///
    /// # Panics
    /// Panics if file listing fails.
    pub fn update(&self) {
        let mut fl = self.get_filelist();
        fl.set_output(Some(self.root.join(".fl").join("STAGE")));

        println!("Updating {}", self.root.display());
        fl.run(vec![self.root.clone()]).unwrap();
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

    pub fn diff_stage(&self, commit: i32) {
        // if there are no commits, I if user gave -1 or 0, diff against empty file
        if self.commits == 0 && [-1, 0].contains(&commit) {
            println!("Diffing EMPTY and STAGE");
            FL::diff_content(
                "",
                &fs_helper::read_to_string(self.root.join(".fl").join("STAGE")),
            );
            return;
        }
        let valid_commit = self.to_valid_history_index(commit);
        println!("Diffing {valid_commit} and STAGE");
        FL::diff_paths(
            &self.history_file_path(valid_commit),
            &self.root.join(".fl").join("STAGE"),
        );
    }

    pub fn commit(&mut self) {
        let stage_file = self.root.join(".fl").join("STAGE");
        let out_path = self.history_file_path(self.commits);

        // if there is a previous commit, diff against it
        let changes = if self.commits > 0 {
            let stage_content = fs_helper::read_to_string(&stage_file);
            let stage_snapshot = fs_helper::parse_commit(&stage_content).snapshot;
            let history_content =
                fs_helper::read_to_string(self.history_file_path(self.commits - 1));
            let history_snapshot = fs_helper::parse_commit(&history_content).snapshot;
            FL::diff_map(&history_snapshot, &stage_snapshot).len()
        // if this is the first commit, diff against an empty snapshot
        } else {
            let stage_content = fs_helper::read_to_string(&stage_file);
            let stage_snapshot = fs_helper::parse_commit(&stage_content).snapshot;
            FL::diff_map(&HashMap::new(), &stage_snapshot).len()
        };
        println!("Committing {} changes", changes);

        fs_helper::copy(self.root.join(".fl").join("STAGE"), out_path);
        self.commits += 1;
    }

    pub fn commit_message(&mut self, message: &str) {
        let commit_path = self.history_file_path(self.commits);
        self.commit();
        let content = fs_helper::read_to_string(&commit_path);
        // prepend the commit message to the file
        fs_helper::write(
            &commit_path,
            format!("{message}\n{FILELIST_MESSAGE_SEP}{content}",),
        );
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
    fn update_commits(&mut self) {
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

    fn diff_paths(old: &Path, new: &Path) {
        let content1 = fs_helper::read_to_string(old);
        let content2 = fs_helper::read_to_string(new);
        FL::diff_content(&content1, &content2);
    }

    fn diff_content(content1: &str, content2: &str) {
        let old_by_path: HashMap<&str, &str> = fs_helper::parse_commit(content1).snapshot;
        let new_by_path: HashMap<&str, &str> = fs_helper::parse_commit(content2).snapshot;
        let actions = FL::diff_map(&old_by_path, &new_by_path);
        if actions.is_empty() {
            println!("No changes");
            return;
        }
        for action in actions {
            println!("{}", action.colored());
        }
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
        old: &HashMap<&'a str, &'a str>,
        new: &HashMap<&'a str, &'a str>,
    ) -> Vec<Action<'a>> {
        // Collect paths that disappeared (keyed by hash, for rename detection).
        // A path goes here only if it is absent from new_by_path entirely —
        // modifications are handled separately and never enter this map.
        let mut deleted_by_hash: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut actions: Vec<Action> = Vec::new();

        for (path, old_hash) in old {
            match new.get(path) {
                None => deleted_by_hash.entry(old_hash).or_default().push(path),
                Some(new_hash) if new_hash != old_hash => actions.push(Action::Modify(path)),
                _ => {}
            }
        }

        // Separate newly added paths into true additions vs rename candidates.
        // A rename candidate is an added path whose hash matches at least one
        // deleted path — same content, different location.
        let mut rename_candidates: HashMap<&str, Vec<&str>> = HashMap::new();

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

        // Match each group of rename candidates against the pool of deleted paths
        // that share the same hash. Within a group we want the globally optimal
        // pairing (minimum total path-distance), not the greedy local optimum.
        // Because real-world rename groups are tiny (almost always 1-to-1, rarely
        // more than a handful), we use brute-force permutation search which is
        // exact and fast enough in practice.
        for (hash, mut new_paths) in rename_candidates {
            // unwrap: key is guaranteed to exist because we only inserted into
            // rename_candidates when deleted_by_hash contained the same hash.
            let deleted_paths = deleted_by_hash.get_mut(hash).unwrap();

            // if there are a lot of renames, randomly choose who gets renamed, because the search is too slow
            if new_paths.len() > RENAME_GROUP_SIZE_LIMIT
                || deleted_paths.len() > RENAME_GROUP_SIZE_LIMIT
            {
                while !deleted_paths.is_empty() && !new_paths.is_empty() {
                    actions.push(Action::Rename(
                        deleted_paths.pop().unwrap(),
                        new_paths.pop().unwrap(),
                    ));
                }
            // if there are few renames, do a full search
            } else {
                let pairings = rename_detection::optimal_pairings(&new_paths, deleted_paths);

                // Collect the deleted-path indices that were consumed by a rename so
                // we can remove them from the pool afterwards.
                let mut used_deleted: Vec<usize> = Vec::new();

                for (new_path, deleted_idx) in pairings {
                    actions.push(Action::Rename(deleted_paths[deleted_idx], new_path));
                    used_deleted.push(deleted_idx);
                    // remove new_path from `new_paths`, because it is now claimed by rename
                    new_paths.remove(new_paths.iter().position(|p| p == &new_path).unwrap());
                }

                // remove every index in `used_deleted` from `deleted_paths`, by removing last index first so earlier indexes remain valid
                used_deleted.sort_unstable_by(|a, b| b.cmp(a));
                for idx in used_deleted {
                    deleted_paths.remove(idx);
                }
            }

            // Any new path that was not claimed by a rename is a true addition
            for path in new_paths {
                actions.push(Action::Add(path));
            }
            if deleted_paths.is_empty() {
                deleted_by_hash.remove(hash);
            }
        }

        // Any deleted path that was not claimed by a rename is a true deletion.
        for paths in deleted_by_hash.values() {
            for path in paths {
                actions.push(Action::Remove(path));
            }
        }

        // sort actions by path
        actions.sort();
        actions
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

#[cfg(test)]
mod tests;
