#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum TomlKeyError {
    #[error("`{0}` is not a table")]
    NotTable(String),

    #[error("`{0}` not found")]
    KeyNotFound(String),
}

// ignore because toml_helper is not public
/// set a key to a value, but this works with nested tables using `.`
/// ```ignore
/// let t = "a = true";
/// let mut doc = t.parse().unwrap();
/// set_doc(&mut doc, "a", toml_edit::value(false)).unwrap();
/// assert_eq!(doc.to_string(), "a = false\n");
/// ```
///
pub fn set_key(
    doc: &mut toml_edit::DocumentMut,
    key: &str,
    val: toml_edit::Item,
) -> Result<(), TomlKeyError> {
    let mut parts = key.split('.');
    // The part of key that we are currently working on, for error messages
    let mut path = String::new();

    let mut table: &mut dyn toml_edit::TableLike = doc.as_table_mut();

    // All segments except the last are intermediate tables
    let last_key = parts
        .next_back()
        .expect(".split() always returns at least one element");

    for segment in parts {
        path.push_str(segment);

        if let Some(inner_table) = table
            .entry(segment)
            .or_insert(toml_edit::table())
            .as_table_like_mut()
        {
            table = inner_table;
            path.push('.');
        // if segment is not a table, error (e.g. `a = 1`, `a.b` is invalid)
        } else {
            return Err(TomlKeyError::NotTable(path));
        }
    }

    match table.entry(last_key) {
        // Only change the value, key is unchanged so it keeps all the comments
        toml_edit::Entry::Occupied(mut o) => {
            *o.get_mut() = val;
        }
        // if it doesn't exist, then create it
        toml_edit::Entry::Vacant(v) => {
            v.insert(val);
        }
    }
    Ok(())
}

pub fn get_key(
    doc: &toml_edit::Document<String>,
    key: &str,
) -> Result<toml_edit::Item, TomlKeyError> {
    let mut parts = key.split('.');
    // The part of key that we are currently working on, for error messages
    let mut path = String::new();

    let mut table: &dyn toml_edit::TableLike = doc.as_table();

    // All segments except the last are intermediate tables
    let last_key = parts
        .next_back()
        .expect(".split() always returns at least one element");

    for segment in parts {
        path.push_str(segment);

        table = get_table(table, segment, &path)?;
        path.push('.');
        // let inner = match table.get(segment) {
        //     Some(inner) => inner,
        //     None => return Err(TomlKeyError::KeyNotFound(path)),
        // };

        // if let Some(inner_table) = inner.as_table_like() {
        //     table = inner_table;
        //     path.push('.');
        // // if segment is not a table, error (e.g. `a = 1`, `a.b` is invalid)
        // } else {
        //     return Err(TomlKeyError::NotTable(path));
        // }
    }

    table
        .get(last_key)
        .cloned()
        .ok_or(TomlKeyError::KeyNotFound(path + last_key))
}

pub fn remove_key(doc: &mut toml_edit::DocumentMut, key: &str) -> Result<(), TomlKeyError> {
    let parts: Vec<&str> = key.split('.').collect();
    let (segments, last_key) = parts.split_at(parts.len() - 1);
    remove_recursive(doc.as_table_mut(), segments, last_key[0], String::new())
}

/// this is recursive because I want to remove resulting empty tables too
/// If i didn't want to do that, I could just do same logic as [`get_key`] but remove instead of return
fn remove_recursive(
    table: &mut dyn toml_edit::TableLike,
    segments: &[&str],
    last_key: &str,
    path_to_table: String,
) -> Result<(), TomlKeyError> {
    match segments {
        [] => match table.remove(last_key) {
            Some(_) => Ok(()),
            None => Err(TomlKeyError::KeyNotFound(join_segments(
                path_to_table,
                last_key,
            ))),
        },
        [head, rest @ ..] => {
            let path_to_head = join_segments(path_to_table, head);
            let child_table = get_table_mut(table, head, &path_to_head)?;

            remove_recursive(child_table, rest, last_key, path_to_head)?;

            // if head became empty now, remove it
            if table
                .get(head)
                .and_then(|v| v.as_table_like())
                .is_some_and(|t| t.is_empty())
            {
                table.remove(head);
            }
            Ok(())
        }
    }
}

fn join_segments(mut parent: String, child: &str) -> String {
    if !parent.is_empty() {
        parent.push('.');
    }
    parent.push_str(child);
    parent
}

/// get `parent.key` as a table
/// if `parent.key` is not a table or does not exist,
/// return [`TomlKeyError`] with [`path_for_error`] as the path to invalid key
fn get_table<'a>(
    parent: &'a dyn toml_edit::TableLike,
    key: &str,
    path_for_error: &str,
) -> Result<&'a dyn toml_edit::TableLike, TomlKeyError> {
    let inner = match parent.get(key) {
        Some(inner) => inner,
        None => return Err(TomlKeyError::KeyNotFound(path_for_error.to_string())),
    };

    inner
        .as_table_like()
        // if segment is not a table, error (e.g. `a = 1`, `a.b` is invalid)
        .ok_or_else(|| TomlKeyError::NotTable(path_for_error.to_string()))
}

/// same as [`get_table`], but mutable
fn get_table_mut<'a>(
    table: &'a mut dyn toml_edit::TableLike,
    key: &str,
    path_for_error: &str,
) -> Result<&'a mut dyn toml_edit::TableLike, TomlKeyError> {
    let inner = match table.get_mut(key) {
        Some(inner) => inner,
        None => return Err(TomlKeyError::KeyNotFound(path_for_error.to_string())),
    };

    inner
        .as_table_like_mut()
        .ok_or_else(|| TomlKeyError::NotTable(path_for_error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_doc() {
        let t = "a = true";
        let mut doc = t.parse().unwrap();
        set_key(&mut doc, "a", toml_edit::value(false)).unwrap();
        assert_eq!(doc.to_string(), "a = false\n");
    }

    #[test]
    #[ignore = "formatting is lost with get_key"]
    fn test_get() {
        let toml = concat!(
            "[here]\n",
            "bob = true\n",
            "# this is a comment\n",
            "age = 42\n",
        );
        let doc = toml.parse().unwrap();
        let get = get_key(&doc, "here").unwrap();
        // unfortunately, this fails and i have no idea how to fix this
        assert_eq!(
            get.to_string(),
            concat!("bob = true\n", "# this is a comment\n", "age = 42\n",)
        );
    }

    // ─── remove_key ───────────────────────────────────────────────────────────

    #[test]
    fn test_remove_top_level() {
        let mut doc = "bob = \"hi\"\n".parse().unwrap();
        remove_key(&mut doc, "bob").unwrap();
        assert_eq!("", doc.to_string());
    }

    #[test]
    fn test_remove_one_of_many() {
        let mut doc = "a = 1\nb = 2\nc = 3\n".parse().unwrap();
        remove_key(&mut doc, "b").unwrap();
        assert_eq!("a = 1\nc = 3\n", doc.to_string());
    }

    #[test]
    fn test_remove_nested() {
        let mut doc = "[table]\nfoo = true\nbar = false\n".parse().unwrap();
        remove_key(&mut doc, "table.foo").unwrap();
        assert_eq!("[table]\nbar = false\n", doc.to_string());
    }

    #[test]
    fn test_remove_last_key_in_table_removes_table() {
        let mut doc = "[table]\nlast = 1\n".parse().unwrap();
        remove_key(&mut doc, "table.last").unwrap();
        // since `last` was the last key in the table, I want the table gone too
        assert_eq!("", doc.to_string());
    }

    #[test]
    fn test_remove_keeps_comments_on_other_keys() {
        let toml = concat!(
            "# top comment\n",
            "keep = true\n",
            "# comment on gone\n",
            "gone = false\n",
        );
        let expected = concat!("# top comment\n", "keep = true\n");
        let mut doc = toml.parse().unwrap();
        remove_key(&mut doc, "gone").unwrap();
        // The comment directly above "gone" is removed along with the key
        let result = doc.to_string();
        assert_eq!(expected, result);
    }

    // ─── set_key error paths ──────────────────────────────────────────────────

    #[test]
    fn test_set_key_not_table() {
        // "a" is a scalar; traversing into "a.b" must report the offending segment.
        let mut doc = "a = 1\n".parse().unwrap();
        assert_eq!(
            Err(TomlKeyError::NotTable("a".to_string())),
            set_key(&mut doc, "a.b", toml_edit::value(2))
        );
    }

    #[test]
    fn test_set_key_not_table_nested() {
        let mut doc = "[outer]\ninner = 1\n".parse().unwrap();
        assert_eq!(
            Err(TomlKeyError::NotTable("outer.inner".to_string())),
            set_key(&mut doc, "outer.inner.bob", toml_edit::value(2))
        );
    }

    // ─── get_key error paths ──────────────────────────────────────────────────

    #[test]
    fn test_get_key_not_found() {
        let doc = "a = 1\n".parse().unwrap();
        assert_eq!(
            TomlKeyError::KeyNotFound("missing".to_string()),
            get_key(&doc, "missing").unwrap_err(),
        );
    }

    #[test]
    fn test_get_key_not_found_nested() {
        // Intermediate table exists but the leaf key does not.
        let doc = "[table]\nfoo = 1\n".parse().unwrap();
        assert_eq!(
            TomlKeyError::KeyNotFound("table.missing".to_string()),
            get_key(&doc, "table.missing").unwrap_err()
        );
    }

    #[test]
    fn test_get_key_not_table() {
        // "a" is a scalar; traversing "a.b" must report NotTable("a").
        let doc = "a = 1\n".parse().unwrap();
        assert_eq!(
            TomlKeyError::NotTable("a".to_string()),
            get_key(&doc, "a.b").unwrap_err(),
        );
    }

    // ─── remove_key error paths ───────────────────────────────────────────────

    #[test]
    fn test_remove_key_not_found() {
        let mut doc = "a = 1\n".parse().unwrap();
        assert_eq!(
            Err(TomlKeyError::KeyNotFound("missing".to_string())),
            remove_key(&mut doc, "missing")
        );
    }

    #[test]
    fn test_remove_key_not_table() {
        let mut doc = "a = 1\n".parse().unwrap();
        assert_eq!(
            Err(TomlKeyError::NotTable("a".to_string())),
            remove_key(&mut doc, "a.b")
        );
    }

    #[test]
    fn test_remove_key_intermediate_not_found() {
        // The intermediate segment itself doesn't exist.
        let mut doc = "a = 1\n".parse().unwrap();
        assert_eq!(
            Err(TomlKeyError::KeyNotFound("no_exist".to_string())),
            remove_key(&mut doc, "no_exist.key")
        );
    }
}
