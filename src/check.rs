use crate::{commit, config, find_fl_path, find_root_path};
use fs_err::{File, read_dir};
use miette::Diagnostic;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CheckWarning {
    #[error("Not inside an fl repository (or any of the parent directories)")]
    #[diagnostic(help("Run `fl init` to create a new fl repository"))]
    #[diagnostic(severity(Error))]
    RepoNotFound,

    #[error("Could not open the .fl directory")]
    #[diagnostic(help("Run `fl init` to create a new fl repository"))]
    #[diagnostic(severity(Error))]
    FlNotFound(#[source] std::io::Error),

    #[error("Could not open the STAGE file")]
    #[diagnostic(help("Run `fl update` to create valid STAGE file"))]
    #[diagnostic(severity(Warning))]
    StageNotFound(#[source] std::io::Error),

    #[error("Could not open the config file")]
    #[diagnostic(help(
        "You can create an empty config file there to fix this\n\
         You can also run `fl config default` to see the default config"
    ))]
    #[diagnostic(severity(Warning))]
    ConfigNotFound(#[source] std::io::Error),

    #[error("Could not open the history directory")]
    #[diagnostic(severity(Warning))]
    HistoryNotFound(#[source] std::io::Error),

    #[error("Invalid commit index: {index} (expected {expected}): {path}")]
    #[diagnostic(severity(Warning))]
    InvalidCommitIndex {
        path: PathBuf,
        index: u32,
        expected: u32,
    },

    #[error("Invalid stage file")]
    #[diagnostic(help("Run `fl update` to create valid STAGE file"))]
    #[diagnostic(severity(Warning))]
    InvalidStage(#[source] commit::CommitError),

    #[error("Failed to parse config")]
    #[diagnostic(help("Fix the syntax error in there, or just make it an empty file"))]
    #[diagnostic(severity(Warning))]
    InvalidConfig(#[from] conf::ConfigError),

    #[error("Invalid commit at index {index} ({path})")]
    #[diagnostic(severity(Warning))]
    InvalidCommit {
        source: commit::CommitError,
        path: PathBuf,
        index: u32,
    },

    #[error("Unrecognized {type}: `{0}`", type = if .0.is_dir() { "directory" } else { "file" })]
    #[diagnostic(help("You can safely delete this"))]
    #[diagnostic(severity(Advice))]
    UnrecognizedEntry(PathBuf),

    #[error("Error hashing {file}: {bad_hash} (full path: {full_path})")]
    #[diagnostic(help("Maybe try checking the permissions of this file?"))]
    #[diagnostic(severity(Advice))]
    BadHash {
        file: PathBuf,
        bad_hash: String,
        full_path: PathBuf,
    },

    #[error("Could not open an entry")]
    #[diagnostic(help(
        "This can happen if the filesystem changed during iteration. Maybe try again"
    ))]
    #[diagnostic(severity(Advice))]
    BadEntry(#[source] std::io::Error),
}

pub fn print_warnings(mut warnings: Vec<CheckWarning>) {
    warnings.sort_by_key(|w| w.severity().unwrap_or_default());

    if warnings.is_empty() {
        println!("No issues found");
    }

    for warning in warnings {
        println!("{:?}", miette::Report::new(warning));
    }
}

pub fn check_current_dir() -> Vec<CheckWarning> {
    let repo_path = if let Ok(repo_path) = find_root_path() {
        repo_path
    } else {
        return vec![CheckWarning::RepoNotFound];
    };

    check(repo_path)
}

pub fn check(dir: PathBuf) -> Vec<CheckWarning> {
    let mut warnings = vec![];

    let recognized = ["STAGE.json", "config.toml", "history", "FL_COMMIT_MESSAGE"];
    let repo = if let Some(repo) = find_fl_path(dir) {
        repo
    } else {
        return vec![CheckWarning::RepoNotFound];
    };
    let fl = repo.join(".fl");

    if let Err(e) = File::open(fl.join("STAGE.json")) {
        warnings.push(CheckWarning::StageNotFound(e));
    } else {
        match commit::Commit::load_from(fl.join("STAGE.json")) {
            Err(e) => warnings.push(CheckWarning::InvalidStage(e)),
            Ok(stage) => {
                for (path, hash) in stage.snapshot {
                    if hash.starts_with("ERROR:") {
                        warnings.push(CheckWarning::BadHash {
                            full_path: repo.join(&path),
                            file: path,
                            bad_hash: hash,
                        });
                    }
                }
            }
        }
    }

    if let Err(e) = File::open(fl.join("config.toml")) {
        warnings.push(CheckWarning::ConfigNotFound(e));
    } else {
        if let Err(e) = config::Config::load(&fl.join("config.toml"), false) {
            warnings.push(CheckWarning::InvalidConfig(e))
        }
    }

    warnings.extend(check_history(&fl));

    match read_dir(&fl) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        // if this entry is not in the list of recognized entries, add a warning
                        if !entry
                            .file_name()
                            .to_str()
                            .is_some_and(|n| recognized.contains(&n))
                        {
                            warnings.push(CheckWarning::UnrecognizedEntry(entry.path()));
                        }
                    }
                    Err(e) => {
                        warnings.push(CheckWarning::BadEntry(e));
                    }
                };
            }
        }
        Err(e) => warnings.push(CheckWarning::FlNotFound(e)),
    }

    warnings
}

fn check_history(fl_path: &Path) -> Vec<CheckWarning> {
    let mut warnings = vec![];
    let mut expected = 0;

    let entries = match read_dir(fl_path.join("history")) {
        Err(e) => return vec![CheckWarning::HistoryNotFound(e)],
        Ok(entries) => entries,
    };

    let mut sorted: Vec<_> = entries
        .filter_map(|e| match e {
            Ok(e) => Some(e),
            Err(e) => {
                warnings.push(CheckWarning::BadEntry(e));
                None
            }
        })
        .collect();

    sorted.sort_by_key(|e| e.file_name());

    for entry in sorted {
        let commit_index = entry
            .file_name()
            .into_string()
            .ok()
            .and_then(|s| s.strip_suffix(".json").map(|s| s.to_string()))
            // if this isn't 8 digits, it's not a valid commit index
            .and_then(|s| if s.len() == 8 { Some(s) } else { None })
            .and_then(|s| s.parse::<u32>().ok());

        match commit_index {
            Some(index) => {
                if index == expected {
                    expected += 1;
                    if let Err(e) = commit::Commit::load_from(entry.path()) {
                        warnings.push(CheckWarning::InvalidCommit {
                            source: e,
                            path: entry.path(),
                            index,
                        });
                    }
                } else {
                    warnings.push(CheckWarning::InvalidCommitIndex {
                        path: entry.path(),
                        index,
                        expected,
                    });
                }
            }
            None => {
                warnings.push(CheckWarning::UnrecognizedEntry(entry.path()));
            }
        }
    }

    warnings
}
