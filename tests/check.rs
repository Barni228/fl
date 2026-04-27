use fl::check::{CheckWarning, check};
use std::io::Write;
use std::{
    fs::{self, OpenOptions},
    os::unix::fs::PermissionsExt,
    path::Path,
};

#[must_use]
fn new_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fl::FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    dir
}

#[test]
fn test_check_no_repo() {
    let dir = new_repo();
    fs::remove_dir_all(dir.path().join(".fl")).unwrap();
    let warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], CheckWarning::RepoNotFound));
}

#[test]
fn test_check_no_stage() {
    let dir = new_repo();
    fs::remove_file(dir.path().join(".fl/STAGE.json")).unwrap();
    let warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], CheckWarning::StageNotFound(_)));
}

#[test]
fn test_check_no_config() {
    let dir = new_repo();
    fs::remove_file(dir.path().join(".fl/config.toml")).unwrap();
    let warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], CheckWarning::ConfigNotFound(_)));
}

#[test]
fn test_check_no_history() {
    let dir = new_repo();
    fs::remove_dir(dir.path().join(".fl/history")).unwrap();
    let warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], CheckWarning::HistoryNotFound(_)));
}

#[test]
fn test_check_unrecognized_entry() {
    let dir = new_repo();
    fs::write(dir.path().join(".fl/foo"), "bar").unwrap();
    let mut warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings.pop().unwrap(),
        CheckWarning::UnrecognizedEntry(entry) if entry.file_name().is_some_and(|n| n == "foo")));
}

#[test]
fn test_check_unrecognized_entry_history() {
    let dir = new_repo();
    fs::write(dir.path().join(".fl/history/foo"), "bar").unwrap();
    let mut warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings.pop().unwrap(),
        CheckWarning::UnrecognizedEntry(entry) if entry.file_name().is_some_and(|n| n == "foo")));
}

#[test]
fn test_check_weird_history_index() {
    let dir = new_repo();
    fs::write(dir.path().join(".fl/history/000.json"), "bar").unwrap();
    let mut warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings.pop().unwrap(),
        CheckWarning::UnrecognizedEntry(entry) if entry.file_name().is_some_and(|n| n == "000.json")));
}

#[test]
fn test_check_invalid_commit_index() {
    let dir = new_repo();
    fs::write(dir.path().join(".fl/history/12345678.json"), "bar").unwrap();
    let mut warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);

    let warning = warnings.pop().unwrap();

    // assert!(matches!(warning, CheckWarning::InvalidCommitIndex { .. }));
    let CheckWarning::InvalidCommitIndex {
        path,
        index,
        expected,
    } = warning
    else {
        panic!("expected InvalidCommitIndex, got {:?}", warning);
    };

    assert_eq!(path, dir.path().join(".fl/history/12345678.json"));
    assert_eq!(index, 12345678);
    assert_eq!(expected, 0);
}

#[test]
fn test_check_invalid_stage() {
    let dir = new_repo();
    fs::write(dir.path().join(".fl/STAGE.json"), "bar").unwrap();
    let warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], CheckWarning::InvalidStage(_)));
}

#[test]
fn test_check_no_read() {
    let dir = new_repo();
    fs::write(dir.path().join("bob"), "bar").unwrap();
    fs::set_permissions(dir.path().join("bob"), fs::Permissions::from_mode(0o200)).unwrap();

    fl::FL::new(dir.path().to_path_buf(), false)
        .unwrap()
        .update()
        .unwrap();

    let mut warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    let CheckWarning::BadHash {
        file,
        bad_hash,
        full_path,
    } = warnings.pop().unwrap()
    else {
        panic!("expected BadHash, got: {warnings:?}");
    };

    assert_eq!(file, Path::new("bob"));
    assert_eq!(full_path, dir.path().join("bob"));
    assert_eq!(bad_hash, "ERROR: Permission denied (os error 13)");
}

#[test]
fn test_check_invalid_config() {
    let dir = new_repo();
    fs::write(dir.path().join(".fl/config.toml"), "bar").unwrap();
    let warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], CheckWarning::InvalidConfig(_)));
}

#[test]
fn test_check_invalid_commit() {
    let dir = new_repo();
    fl::FL::new(dir.path().to_path_buf(), false)
        .unwrap()
        .commit_empty()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(dir.path().join(".fl/history/00000000.json"))
        .unwrap();

    writeln!(file, "hello").unwrap();
    let mut warnings = check(dir.path().to_path_buf());
    assert_eq!(warnings.len(), 1);
    let CheckWarning::InvalidCommit { index, path, .. } = warnings.pop().unwrap() else {
        panic!("expected InvalidCommit, got: {warnings:?}");
    };

    assert_eq!(index, 0);
    assert_eq!(path, dir.path().join(".fl/history/00000000.json"));
}

#[test]
fn test_check_no_issues() {
    let dir = new_repo();
    let warnings = check(dir.path().to_path_buf());
    assert!(warnings.is_empty());
}
