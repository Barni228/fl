use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

use crate::fs_helper;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Commit {
    pub title: Option<String>,
    pub body: Option<String>,
    pub snapshot: BTreeMap<String, String>,
}

impl Commit {
    pub fn new(
        title: Option<String>,
        body: Option<String>,
        snapshot: BTreeMap<String, String>,
    ) -> Self {
        Self {
            title,
            body,
            snapshot,
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Self {
        // let file = fs_helper::open(&path);
        // let reader = BufReader::new(file);
        // serde_json::from_reader(reader).unwrap()

        // serde says that this is more efficient
        let content = fs_helper::read_to_string(&path);
        serde_json::from_str(&content).unwrap()
    }

    pub fn save_to(&self, path: impl AsRef<Path>) {
        let content = serde_json::to_string_pretty(self).unwrap();
        fs_helper::write(&path, &content);
    }
}
