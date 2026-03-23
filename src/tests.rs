use std::fs;

use super::*;

// ─── helpers ──────────────────────────────────────────────────────────────────

fn hm<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> HashMap<&'a str, &'a str> {
    pairs.into_iter().collect()
}

// ─── Add ──────────────────────────────────────────────────────────────────────

#[test]
fn test_diff_add_to_empty() {
    // so the hash maps are like this: {"path": "hash"}
    // usually hash should be 64 random characters, but really it can be any string, so i use "0" for simplicity
    let before = HashMap::new();
    let after = hm([("new", "0")]);
    assert_eq!(FL::diff_map(&before, &after), vec![Action::Add("new")]);
}

#[test]
fn test_diff_add() {
    let before = hm([("old", "0")]);
    let after = hm([("old", "0"), ("new", "1")]);
    assert_eq!(FL::diff_map(&before, &after), vec![Action::Add("new")]);
}

#[test]
fn test_diff_add_multiple() {
    let before = HashMap::new();
    let after = hm([("a", "1"), ("b", "2"), ("c", "3")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Add("a"), Action::Add("b"), Action::Add("c")]
    );
}

// ─── Remove ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_remove_single() {
    let before = hm([("old", "0")]);
    let after = HashMap::new();
    assert_eq!(FL::diff_map(&before, &after), vec![Action::Remove("old")]);
}

#[test]
fn test_diff_remove_multiple() {
    let before = hm([("a", "1"), ("b", "2"), ("c", "3")]);
    let after = HashMap::new();
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![
            Action::Remove("a"),
            Action::Remove("b"),
            Action::Remove("c")
        ]
    );
}

#[test]
fn test_diff_remove_and_unchanged() {
    let before = hm([("keep", "0"), ("gone", "1")]);
    let after = hm([("keep", "0")]);
    assert_eq!(FL::diff_map(&before, &after), vec![Action::Remove("gone")]);
}

// ─── Modify ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_modify_single() {
    let before = hm([("file", "aaa")]);
    let after = hm([("file", "bbb")]);
    assert_eq!(FL::diff_map(&before, &after), vec![Action::Modify("file")]);
}

#[test]
fn test_diff_modify_multiple() {
    let before = hm([("a", "1"), ("b", "2")]);
    let after = hm([("a", "X"), ("b", "Y")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Modify("a"), Action::Modify("b")]
    );
}

#[test]
fn test_diff_modify_and_unchanged() {
    let before = hm([("changed", "old"), ("same", "0")]);
    let after = hm([("changed", "new"), ("same", "0")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Modify("changed")]
    );
}

// ─── Rename ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_rename_simple() {
    let before = hm([("old_name.txt", "abc")]);
    let after = hm([("new_name.txt", "abc")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Rename("old_name.txt", "new_name.txt")]
    );
}

#[test]
fn test_diff_move() {
    let before = hm([("file.txt", "abc")]);
    let after = hm([("subdir/file.txt", "abc")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Rename("file.txt", "subdir/file.txt")]
    );
}

#[test]
fn test_diff_rename_and_move() {
    let before = hm([("dir/old.txt", "hash")]);
    let after = hm([("other/new.txt", "hash")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Rename("dir/old.txt", "other/new.txt")]
    );
}

#[test]
fn test_diff_rename_best_match() {
    // alpha.txt → alpha_v2.txt is a better match (lower edit distance) than alpha.txt → gamma.txt
    let before = hm([("alpha.txt", "H"), ("beta.txt", "H")]);
    let after = hm([("alpha_v2.txt", "H"), ("gamma.txt", "H")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![
            Action::Rename("alpha.txt", "alpha_v2.txt"),
            Action::Rename("beta.txt", "gamma.txt")
        ]
    );
}

#[test]
fn test_diff_rename_and_remove() {
    // Two deleted, one added → one rename + one true deletion
    let before = hm([("a.txt", "H"), ("b.txt", "H")]);
    let after = hm([("c.txt", "H")]);
    let actions = FL::diff_map(&before, &after);
    assert_eq!(actions.len(), 2); // two actions
    assert!(actions.iter().any(|a| matches!(a, Action::Remove(_)))); // one of them is added
    assert!(actions.iter().any(|a| matches!(a, Action::Rename(_, _)))); // one of them is rename
}

#[test]
fn test_diff_rename_and_add() {
    // One deleted, two added with same hash → one rename + one true addition
    let before = hm([("a.txt", "H")]);
    let after = hm([("b.txt", "H"), ("c.txt", "H")]);
    let actions = FL::diff_map(&before, &after);
    assert_eq!(actions.len(), 2); // two actions
    assert!(actions.iter().any(|a| matches!(a, Action::Add(_)))); // one of them is added
    assert!(actions.iter().any(|a| matches!(a, Action::Rename(_, _)))); // one of them is rename
}

#[test]
fn test_diff_rename_while_modifying() {
    // Different hashes → must be Remove + Add, never Rename
    let before = hm([("a.txt", "HASH_A")]);
    let after = hm([("b.txt", "HASH_B")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Remove("a.txt"), Action::Add("b.txt")]
    );
}

#[test]
fn test_diff_rename_different_files() {
    // Two renames with distinct hashes — groups must not cross-contaminate
    let before = hm([("a.txt", "HASH_A"), ("b.txt", "HASH_B")]);
    let after = hm([("c.txt", "HASH_A"), ("d.txt", "HASH_B")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![
            Action::Rename("a.txt", "c.txt"),
            Action::Rename("b.txt", "d.txt")
        ]
    );
}

#[test]
fn test_diff_rename_different_dir() {
    let before = hm([("src/widget.rs", "H"), ("src/gadget.rs", "H")]);
    let after = hm([("src/widget_v2.rs", "H"), ("lib/gadget.rs", "H")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![
            Action::Rename("src/gadget.rs", "lib/gadget.rs"),
            Action::Rename("src/widget.rs", "src/widget_v2.rs")
        ]
    );
}

// ─── Mixed actions ────────────────────────────────────────────────────────────

#[test]
fn test_diff_all() {
    let before = hm([
        ("keep.txt", "K"),
        ("gone.txt", "_"),
        ("renamed.txt", "R"),
        ("change.txt", "old"),
    ]);
    let after = hm([
        ("keep.txt", "K"),     // unchanged → no action
        ("new.txt", "N"),      // added
        ("new-name.txt", "R"), // renamed from `renamed.txt` to `new-name.txt`
        ("change.txt", "new"), // modified
                               // gone.txt absent → removed
    ]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![
            Action::Modify("change.txt"),
            Action::Remove("gone.txt"),
            Action::Rename("renamed.txt", "new-name.txt"),
            Action::Add("new.txt"),
        ]
    );
}

#[test]
fn test_diff_modify() {
    // Same path, different hash → Modify, not Rename
    let before = hm([("file.txt", "old_hash")]);
    let after = hm([("file.txt", "new_hash")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![Action::Modify("file.txt")]
    );
}

// ─── empty ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_both_empty() {
    let before: HashMap<&str, &str> = HashMap::new();
    let after: HashMap<&str, &str> = HashMap::new();
    assert!(FL::diff_map(&before, &after).is_empty());
}

#[test]
fn test_diff_identical() {
    let before = hm([("a", "1"), ("b", "2"), ("c", "3")]);
    let after = hm([("a", "1"), ("b", "2"), ("c", "3")]);
    assert!(FL::diff_map(&before, &after).is_empty());
}

// ─── Output ordering ─────────────────────────────────────────────────────────

#[test]
fn test_diff_output_is_sorted() {
    // The returned Vec must already be sorted regardless of HashMap iteration order
    let before = hm([("z_gone.txt", "Z"), ("a_change.txt", "old")]);
    let after = hm([("a_change.txt", "new"), ("m_new.txt", "M")]);
    assert_eq!(
        FL::diff_map(&before, &after),
        vec![
            Action::Modify("a_change.txt"),
            Action::Add("m_new.txt"),
            Action::Remove("z_gone.txt")
        ]
    );
}

// ─── Repo: automatically find the current repo ────────────────────────────────

#[test]
fn test_repo_find() {
    assert_eq!(
        fs_helper::find_fl_path("test_repo".into()),
        Some(PathBuf::from("test_repo"))
    );
}

#[test]
fn test_repo_parent() {
    assert_eq!(
        fs_helper::find_fl_path("test_repo/subfolder".into()),
        Some(PathBuf::from("test_repo"))
    );
    assert_eq!(
        fs_helper::find_fl_path("test_repo/subfolder/sub-sub-folder".into()),
        Some(PathBuf::from("test_repo"))
    );
}

#[test]
fn test_repo_not_found() {
    // the root folder is probably not a fl repo, at least I hope so
    assert_eq!(fs_helper::find_fl_path("/".into()), None);
}

// ─── Update ───────────────────────────────────────────────────────────────────
// TODO: Don't create a temp folder in the current dir, and don't assume fl init just creates `.fl/history`
#[test]
fn test_update() {
    // let test_folder = std::env::temp_dir().join("__temp_test_update_folder");
    let test_folder = PathBuf::from("__temp_test_update_folder");

    // "create" a new repo
    let _ = fs::remove_dir_all(test_folder.clone());
    fs::create_dir_all(test_folder.join(".fl").join("history")).unwrap();

    // create a file in the repo
    let file_path = test_folder.join("file.txt");
    fs::write(&file_path, "hello\n").unwrap();

    let fl = FL::new(test_folder.clone());
    fl.update();

    assert_eq!(
        fs::read_to_string(test_folder.join(".fl").join("STAGE")).unwrap(),
        concat!(
            "7f39224e335994886c26ba8c241fcbe1d474aadaa2bd0a8e842983b098cea894\t./\n",
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\tfile.txt\n",
        )
    );

    // cleanup after
    fs::remove_dir_all(test_folder).unwrap();
}
