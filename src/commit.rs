use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Commit {
    /// The title of the commit, should be short one line
    pub title: Option<String>,
    /// If the title is not enough, this is a longer description, can be multi-line
    pub body: Option<String>,
    /// The timestamp of when the commit was created
    pub timestamp: Option<OffsetDateTime>,
    /// The snapshot which maps all files/directories to their hashes
    pub snapshot: BTreeMap<PathBuf, String>,
}

#[derive(Error, Debug)]
pub enum CommitError {
    #[error("Failed to parse commit from json")]
    ParseError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),
}

impl Commit {
    /// Creates a new commit with the current timestamp
    pub fn with_timestamp() -> Self {
        let mut commit = Commit::default();
        commit.set_timestamp_now();
        commit
    }

    /// Loads a commit from a file
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, CommitError> {
        let content = fs::read_to_string(&path)?;
        let commit = serde_json::from_str::<Commit>(&content)?;
        Ok(commit)
    }

    /// Save a commit to a file
    pub fn save_to(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, content)
    }

    /// Sets the timestamp to the current time
    pub fn set_timestamp_now(&mut self) {
        let now = OffsetDateTime::now_utc();
        self.timestamp = Some(now);
    }

    pub fn time_ago(&self) -> String {
        if let Some(ts) = self.timestamp {
            time_ago(ts)
        } else {
            "<no timestamp>".to_string()
        }
    }

    pub fn date(&self) -> String {
        if let Some(ts) = self.timestamp {
            ts.date().to_string()
        } else {
            "<no timestamp>".to_string()
        }
    }

    pub fn title(&self) -> String {
        self.title.clone().unwrap_or("<no title>".to_string())
    }

    pub fn body(&self) -> String {
        self.body.clone().unwrap_or("<no body>".to_string())
    }
}

fn time_ago(ts: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let delta = now - ts;

    if delta < Duration::seconds(10) {
        "just now".to_string()
    } else if delta < Duration::minutes(1) {
        let secs = delta.whole_seconds();
        format!("{} second{} ago", secs, if secs != 1 { "s" } else { "" })
    } else if delta < Duration::hours(1) {
        let mins = delta.whole_minutes();
        format!("{} minute{} ago", mins, if mins != 1 { "s" } else { "" })
    } else if delta < Duration::days(1) {
        let hours = delta.whole_hours();
        format!("{} hour{} ago", hours, if hours != 1 { "s" } else { "" })
    } else if delta < Duration::days(7) {
        let days = delta.whole_days();
        format!("{} day{} ago", days, if days != 1 { "s" } else { "" })
    } else if delta < Duration::weeks(5) {
        let weeks = delta.whole_weeks();
        format!("{} week{} ago", weeks, if weeks != 1 { "s" } else { "" })
    } else if delta < Duration::days(365) {
        let months = delta.whole_days() / 30;
        format!("{} month{} ago", months, if months != 1 { "s" } else { "" })
    } else {
        let years = delta.whole_days() / 365;
        format!("{} year{} ago", years, if years != 1 { "s" } else { "" })
    }
}
