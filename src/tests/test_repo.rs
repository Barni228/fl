use super::*;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

// ─── Repo: automatically find the current repo ────────────────────────────────

#[test]
fn test_repo_find() {
    assert_eq!(
        Some(PathBuf::from("test_repo")),
        FL::find_fl_path("test_repo".into())
    );
}

#[test]
fn test_repo_parent() {
    assert_eq!(
        Some(PathBuf::from("test_repo")),
        FL::find_fl_path("test_repo/subfolder".into())
    );
    assert_eq!(
        Some(PathBuf::from("test_repo")),
        FL::find_fl_path("test_repo/subfolder/sub-sub-folder".into())
    );
}

#[test]
fn test_repo_not_found() {
    // the root folder is probably not a fl repo, at least I hope so...
    assert_eq!(None, FL::find_fl_path("/".into()));
}

// ─── Update ───────────────────────────────────────────────────────────────────
#[test]
fn test_update() -> anyhow::Result<()> {
    // create fl repo
    let test_dir = tempfile::TempDir::new()?;
    let fl = FL::create_fl_repo(test_dir.path().to_path_buf())?;

    // create a file in the repo
    let file_path = test_dir.path().join("file.txt");
    fs::write(&file_path, "hello\n").unwrap();

    // Generate the STAGE snapshot
    fl.update()?;

    let content = fs::read_to_string(fl.stage_path())?;
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(
        json!({
            "title": null,
            "body": null,
            "timestamp": null,
            "snapshot": {
                ".": "7f39224e335994886c26ba8c241fcbe1d474aadaa2bd0a8e842983b098cea894",
                "file.txt": "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
            },
        }),
        parsed
    );
    Ok(())
}

// ─── Errors ───────────────────────────────────────────────────────────────────
#[test]
fn test_commit_no_exist() {
    let err = commit::Commit::from_path("no_exist").unwrap_err();
    assert_eq!(
        "I/O error: failed to open file `no_exist`: No such file or directory (os error 2)",
        err.to_string()
    )
}
