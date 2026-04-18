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

#[test]
fn test_repo_structure() -> Result<(), Box<dyn std::error::Error>> {
    fn list_dir(path: &Path) -> Vec<PathBuf> {
        fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .map(|p| p.strip_prefix(path).unwrap().to_path_buf())
            .collect()
    }

    // create fl repo
    let dir = tempfile::TempDir::new()?;
    // initially the repo is empty
    assert!(fs::read_dir(dir.path())?.next().is_none());

    let fl = FL::create_fl_repo(dir.path().to_path_buf())?;
    assert_eq!(vec![PathBuf::from(".fl")], list_dir(dir.path()));
    assert_eq!(
        vec![
            PathBuf::from("history"),
            PathBuf::from("config.toml"),
            PathBuf::from("STAGE.json")
        ],
        list_dir(&dir.path().join(".fl"))
    );
    assert_eq!(
        Vec::<PathBuf>::new(),
        list_dir(&dir.path().join(".fl").join("history"))
    );
    assert_eq!(
        "# TIP: run `fl config default` to see the default config\n",
        fs::read_to_string(dir.path().join(".fl").join("config.toml"))?
    );
    assert_eq!(
        json!({
            "title": null,
            "body": null,
            "timestamp": null,
            "snapshot": {},
        }),
        serde_json::from_str::<serde_json::Value>(&fs::read_to_string(fl.stage_path())?)?
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
