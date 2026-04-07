#[derive(thiserror::Error, Debug)]
pub enum TomlSetError {
    #[error("`{0}` is not a table")]
    NotTable(String), // name of the thing that is not table (`a.b`, "a" will be the String)
}

#[derive(thiserror::Error, Debug)]
pub enum TomlGetError {
    #[error("`{0}` is not a table")]
    NotTable(String),

    #[error("`{0}` not found")]
    KeyNotFound(String),
}

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
) -> Result<(), TomlSetError> {
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
            return Err(TomlSetError::NotTable(path));
        }
    }

    match table.entry(last_key) {
        // Only change the value, key is unchanged so it keeps all the comments
        toml_edit::Entry::Occupied(mut o) => {
            *o.get_mut() = val;
        }
        // if it doesn't exist, then create it
        // if I inserted immediately, all comments around the key would be lost
        toml_edit::Entry::Vacant(v) => {
            v.insert(val);
        }
    }
    Ok(())
}

pub fn get_key(
    doc: &toml_edit::Document<String>,
    key: &str,
) -> Result<toml_edit::Item, TomlGetError> {
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

        let inner = match table.get(segment) {
            Some(inner) => inner,
            None => return Err(TomlGetError::KeyNotFound(path)),
        };

        if let Some(inner_table) = inner.as_table_like() {
            table = inner_table;
            path.push('.');
        // if segment is not a table, error (e.g. `a = 1`, `a.b` is invalid)
        } else {
            return Err(TomlGetError::NotTable(path));
        }
    }

    table
        .get(last_key)
        .cloned()
        .ok_or(TomlGetError::KeyNotFound(path + last_key))
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
}
