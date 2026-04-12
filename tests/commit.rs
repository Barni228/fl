use fl::{FL, commit::Commit};
use std::{collections::BTreeMap, fs, path::PathBuf};

#[test]
fn test_no_commits() {
    let dir = tempfile::TempDir::new().unwrap();
    let fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    assert_eq!(0, fl.commits());
}

#[test]
fn test_commit_count() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();

    for i in 1..=10 {
        fl.commit_empty().unwrap();
        assert_eq!(i, fl.commits());
    }
}

#[test]
fn test_commit_no_stage() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();

    fl.commit_empty().unwrap();
    let commit = fl.get_commit(0).unwrap();
    assert_eq!(
        Commit {
            title: None,
            body: None,
            timestamp: commit.timestamp,
            snapshot: BTreeMap::new(),
        },
        commit
    );
}

#[test]
fn test_commit() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();

    fl.update().unwrap();
    fl.commit_empty().unwrap();
    let commit = fl.get_commit(0).unwrap();
    assert_eq!(
        Commit {
            title: None,
            body: None,
            timestamp: commit.timestamp,
            snapshot: BTreeMap::from([
                (
                    PathBuf::from("."),
                    "d7914fe546b684688bb95f4f888a92dfc680603a75f23eb823658031fff766d9".to_string()
                ),
                (
                    PathBuf::from("file.txt"),
                    "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".to_string()
                ),
            ])
        },
        commit
    );
}

#[test]
fn test_commit_title() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    fl.commit_message("title").unwrap();
    let commit = fl.get_commit(0).unwrap();

    assert_eq!(
        Commit {
            title: Some("title".to_string()),
            body: None,
            timestamp: commit.timestamp,
            snapshot: BTreeMap::new(),
        },
        commit
    );
}

#[test]
fn test_commit_message() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    let message = "\
        title\n\
        body\n\
    ";
    fl.commit_message(message).unwrap();
    let commit = fl.get_commit(0).unwrap();

    assert_eq!(
        Commit {
            title: Some("title".to_string()),
            body: Some("body".to_string()),
            timestamp: commit.timestamp,
            snapshot: BTreeMap::new(),
        },
        commit
    );
}

#[test]
fn test_commit_message_multi_line() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut fl = FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    let message = concat!(
        "\n",             // ignored
        " # comment!\n",  // comments are ignored
        " Title!   \n",   // title will be trimmed
        " \n",            // trimmed away
        " Body \n",       // both spaces will be trimmed
        "\n",             // NOT trimmed, it is a part of the body
        "More body \n",   // the last space will be trimmed
        " not trimmed\n", // the first space will not be trimmed, part of the body
        "\n"              // trimmed
    );

    fl.commit_message(message).unwrap();
    let commit = fl.get_commit(0).unwrap();

    assert_eq!(
        Commit {
            title: Some("Title!".to_string()),
            body: Some("Body\n\nMore body\n not trimmed".to_string()),
            timestamp: commit.timestamp,
            snapshot: BTreeMap::new(),
        },
        commit
    );
}
