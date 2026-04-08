use super::*;

// ─── helpers ──────────────────────────────────────────────────────────────────

fn hm<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> HashMap<&'a Path, &'a str> {
    pairs
        .into_iter()
        .map(|(path, hash)| (Path::new(path), hash))
        .collect()
}

fn add(path: &'_ str) -> Action<'_> {
    Action::Add(Path::new(path))
}

fn remove(path: &'_ str) -> Action<'_> {
    Action::Remove(Path::new(path))
}

fn modify(path: &'_ str) -> Action<'_> {
    Action::Modify(Path::new(path))
}

fn rename<'a>(from: &'a str, to: &'a str) -> Action<'a> {
    Action::Rename(Path::new(from), Path::new(to))
}

// ─── Add ──────────────────────────────────────────────────────────────────────

#[test]
fn test_diff_add_to_empty() {
    // so the hash maps are like this: {"path": "hash"}
    // usually hash should be 64 random characters, but really it can be any string, so i use "0" for simplicity
    let before = HashMap::new();
    let after = hm([("new", "0")]);
    assert_eq!(vec![add("new")], FL::diff_map(&before, &after));
}

#[test]
fn test_diff_add() {
    let before = hm([("old", "0")]);
    let after = hm([("old", "0"), ("new", "1")]);
    assert_eq!(vec![add("new")], FL::diff_map(&before, &after));
}

#[test]
fn test_diff_add_multiple() {
    let before = HashMap::new();
    let after = hm([("a", "1"), ("b", "2"), ("c", "3")]);
    assert_eq!(
        vec![add("a"), add("b"), add("c")],
        FL::diff_map(&before, &after)
    );
}

// ─── Remove ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_remove_single() {
    let before = hm([("old", "0")]);
    let after = HashMap::new();
    assert_eq!(vec![remove("old")], FL::diff_map(&before, &after));
}

#[test]
fn test_diff_remove_multiple() {
    let before = hm([("a", "1"), ("b", "2"), ("c", "3")]);
    let after = HashMap::new();
    assert_eq!(
        vec![remove("a"), remove("b"), remove("c")],
        FL::diff_map(&before, &after)
    );
}
#[test]
fn test_diff_remove_and_unchanged() {
    let before = hm([("keep", "0"), ("gone", "1")]);
    let after = hm([("keep", "0")]);
    assert_eq!(vec![remove("gone")], FL::diff_map(&before, &after));
}

// ─── Modify ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_modify_single() {
    let before = hm([("file", "aaa")]);
    let after = hm([("file", "bbb")]);
    assert_eq!(vec![modify("file")], FL::diff_map(&before, &after));
}

#[test]
fn test_diff_modify_multiple() {
    let before = hm([("a", "1"), ("b", "2")]);
    let after = hm([("a", "X"), ("b", "Y")]);
    assert_eq!(
        vec![modify("a"), modify("b")],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_modify_and_unchanged() {
    let before = hm([("changed", "old"), ("same", "0")]);
    let after = hm([("changed", "new"), ("same", "0")]);
    assert_eq!(vec![modify("changed")], FL::diff_map(&before, &after));
}

// ─── Rename ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_rename_simple() {
    let before = hm([("old_name.txt", "abc")]);
    let after = hm([("new_name.txt", "abc")]);
    assert_eq!(
        vec![rename("old_name.txt", "new_name.txt")],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_move() {
    let before = hm([("file.txt", "abc")]);
    let after = hm([("subdir/file.txt", "abc")]);
    assert_eq!(
        vec![rename("file.txt", "subdir/file.txt")],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_rename_and_move() {
    let before = hm([("dir/old.txt", "hash")]);
    let after = hm([("other/new.txt", "hash")]);
    assert_eq!(
        vec![rename("dir/old.txt", "other/new.txt")],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_rename_best_match() {
    // alpha.txt → alpha_v2.txt is a better match (lower edit distance) than alpha.txt → gamma.txt
    let before = hm([("alpha.txt", "H"), ("beta.txt", "H")]);
    let after = hm([("alpha_v2.txt", "H"), ("gamma.txt", "H")]);
    assert_eq!(
        vec![
            rename("alpha.txt", "alpha_v2.txt"),
            rename("beta.txt", "gamma.txt"),
        ],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_rename_best_many() {
    // alpha.txt → alpha_v2.txt is a better match (lower edit distance) than alpha.txt → gamma.txt
    let before = hm([
        ("one", "H"),
        ("two", "H"),
        ("three", "H"),
        ("four", "H"),
        ("five", "H"),
        ("six", "H"),
        ("seven", "H"),
        ("eight", "H"),
        ("nine", "H"),
        ("ten", "H"),
    ]);
    let after = hm([
        ("One.txt", "H"),
        ("Two.txt", "H"),
        ("Three.txt", "H"),
        ("Four.txt", "H"),
        ("Five.txt", "H"),
        ("Six.txt", "H"),
        ("Seven.txt", "H"),
        ("Eight.txt", "H"),
        ("Nine.txt", "H"),
        ("Ten.txt", "H"),
    ]);
    assert_eq!(
        vec![
            rename("eight", "Eight.txt"),
            rename("five", "Five.txt"),
            rename("four", "Four.txt"),
            rename("nine", "Nine.txt"),
            rename("one", "One.txt"),
            rename("seven", "Seven.txt"),
            rename("six", "Six.txt"),
            rename("ten", "Ten.txt"),
            rename("three", "Three.txt"),
            rename("two", "Two.txt"),
        ],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_rename_and_remove() {
    // Two deleted, one added → one rename + one true deletion
    let before = hm([("a.txt", "H"), ("b.txt", "H")]);
    let after = hm([("c.txt", "H")]);
    let actions = FL::diff_map(&before, &after);
    assert_eq!(2, actions.len()); // two actions
    assert!(actions.iter().any(|a| matches!(a, Action::Remove(_)))); // one of them is added
    assert!(actions.iter().any(|a| matches!(a, Action::Rename(_, _)))); // one of them is rename
}

#[test]
fn test_diff_rename_and_add() {
    // One deleted, two added with same hash → one rename + one true addition
    let before = hm([("a.txt", "H")]);
    let after = hm([("b.txt", "H"), ("c.txt", "H")]);
    let actions = FL::diff_map(&before, &after);
    assert_eq!(2, actions.len()); // two actions
    assert!(actions.iter().any(|a| matches!(a, Action::Add(_)))); // one of them is added
    assert!(actions.iter().any(|a| matches!(a, Action::Rename(_, _)))); // one of them is rename
}

#[test]
fn test_diff_rename_while_modifying() {
    // Different hashes → must be Remove + Add, never Rename
    let before = hm([("a.txt", "HASH_A")]);
    let after = hm([("b.txt", "HASH_B")]);
    assert_eq!(
        vec![remove("a.txt"), add("b.txt")],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_rename_different_files() {
    // Two renames with distinct hashes — groups must not cross-contaminate
    let before = hm([("a.txt", "HASH_A"), ("b.txt", "HASH_B")]);
    let after = hm([("c.txt", "HASH_A"), ("d.txt", "HASH_B")]);
    assert_eq!(
        vec![rename("a.txt", "c.txt"), rename("b.txt", "d.txt")],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_rename_different_dir() {
    let before = hm([("src/widget.rs", "H"), ("src/gadget.rs", "H")]);
    let after = hm([("src/widget_v2.rs", "H"), ("lib/gadget.rs", "H")]);
    assert_eq!(
        vec![
            rename("src/gadget.rs", "lib/gadget.rs"),
            rename("src/widget.rs", "src/widget_v2.rs"),
        ],
        FL::diff_map(&before, &after)
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
        vec![
            modify("change.txt"),
            remove("gone.txt"),
            rename("renamed.txt", "new-name.txt"),
            add("new.txt"),
        ],
        FL::diff_map(&before, &after)
    );
}

#[test]
fn test_diff_modify() {
    // Same path, different hash → Modify, not Rename
    let before = hm([("file.txt", "old_hash")]);
    let after = hm([("file.txt", "new_hash")]);
    assert_eq!(vec![modify("file.txt")], FL::diff_map(&before, &after));
}

// ─── empty ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_both_empty() {
    let before = hm([]);
    let after = hm([]);
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
        vec![
            modify("a_change.txt"),
            add("m_new.txt"),
            remove("z_gone.txt")
        ],
        FL::diff_map(&before, &after)
    );
}
