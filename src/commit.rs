use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use time::{Duration, OffsetDateTime};

use crate::fs_helper;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Commit {
    pub title: Option<String>,
    pub body: Option<String>,
    pub timestamp: Option<OffsetDateTime>,
    pub snapshot: BTreeMap<String, String>,
}

impl Commit {
    /// Creates a new commit with the current timestamp
    pub fn with_timestamp() -> Self {
        let mut commit = Commit::default();
        commit.set_timestamp_now();
        commit
    }

    /// Loads a commit from a file
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let content = fs_helper::read_to_string(&path);
        serde_json::from_str(&content).unwrap()
    }

    /// Save a commit to a file
    pub fn save_to(&self, path: impl AsRef<Path>) {
        let content = serde_json::to_string_pretty(self).unwrap();
        fs_helper::write(&path, &content);
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
            "<missing timestamp>".to_string()
        }
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
    } else if delta < Duration::weeks(4) {
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
