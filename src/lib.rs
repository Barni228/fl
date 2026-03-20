use filelist::FileList;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub fn init() {
    mkdir(".fl");
    mkdir(".fl/history");
}

// TODO: make this `FL` struct that stores needed info like root folder and history indexes
// TODO: so its going to work like this:
// update creates a .fl/STAGE file
// commit copies the .fl/STAGE file to .fl/history/
// something like "fl st" will compare the STAGE file to the latest history file
// "fl diff" will compare 2 history files together OR
// "fl diff" will compare given history file to the STAGE OR
// "fl diff" will compare most recent history file to the STAGE
// you can give history files as numbers like (1 instead of 00000001)
// you can also give history numbers as negative (-1 means last history file, -2 means second to last history file)

pub fn update() {
    let root = get_root_path();
    let out_path = get_output_path(root.clone());
    let mut fl = get_filelist(&root);
    // fl.set_output(Some(root.join(".fl/STAGE")));
    fl.set_output(Some(out_path));

    println!("Updating {}", root.display());
    fl.run(vec![root]).unwrap();
}

pub fn diff_history(first: i32, second: i32) {
    diff_paths(&get_history_file(first), &get_history_file(second));
}

pub fn diff_paths(first: &Path, second: &Path) {
    let content1 = read_file(first);
    let content2 = read_file(second);

    let old_by_path: HashMap<&str, &str> = parse_filelist(&content1);
    let new_by_path: HashMap<&str, &str> = parse_filelist(&content2);
    let mut deleted_by_hash: HashMap<&str, Vec<&str>> = HashMap::new();

    // detect deletions and modifications
    for (path, old_hash) in &old_by_path {
        match new_by_path.get(path) {
            None => deleted_by_hash.entry(old_hash).or_default().push(path),
            Some(new_hash) if new_hash != old_hash => println!("M  {path}"),
            _ => {}
        }
    }

    // detect additions
    for (path, new_hash) in &new_by_path {
        if old_by_path.contains_key(path) {
            continue;
        // if I "deleted" a file, but also "added" exactly the same file but with different path
        // then it is just a rename / move
        } else if let Some(deleted_paths) = deleted_by_hash.get_mut(new_hash)
            && let Some(deleted_path) = deleted_paths.pop()
        {
            println!(r#"R  "{deleted_path}" -> "{path}""#);
        } else {
            println!("A  {path}");
        }
    }
    for paths in deleted_by_hash.values() {
        for path in paths {
            println!("D  {path}");
        }
    }
}

fn get_history_file(index: i32) -> PathBuf {
    PathBuf::from(format!(
        "{}/.fl/history/{index:08}",
        get_root_path().display()
    ))
}

// take `&str` content, so that I can return `&str` HashMap
// if I take something like file path, then I would need to return `String` HashMap
fn parse_filelist(content: &str) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let (hash, path) = line.split_once('\t').unwrap();
        map.insert(path, hash);
    }
    map
}

fn get_output_path(root: PathBuf) -> PathBuf {
    let history_path = root.join(".fl/history");

    let next = read_dir(&history_path)
        .filter_map(Result::ok) // ignore read errors
        .filter_map(|e| e.file_name().into_string().ok()) // convert file name to string
        .filter_map(|s| {
            // convert string to number
            s.parse::<u32>()
                .inspect_err(|_| {
                    eprintln!(
                        "WARNING: invalid history file name: '{}' ({})",
                        s,
                        history_path.join(&s).display()
                    )
                })
                .ok()
        })
        .max()
        .map_or(0, |n| n + 1);

    history_path.join(format!("{next:08}"))
}

fn read_file(path: impl AsRef<Path>) -> String {
    match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "fatal: Failed to read '{}' file: {e}",
                path.as_ref().display()
            );
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}
fn get_root_path() -> PathBuf {
    let mut dir = get_current_dir();

    loop {
        let candidate = dir.join(".fl");

        if candidate.is_dir() {
            return dir;
        }

        if !dir.pop() {
            eprintln!("fatal: not inside an fl repository (or any of the parent directories)");
            std::process::exit(1);
        }
    }
}

fn get_filelist(root: &Path) -> FileList {
    let mut fl = FileList::new();
    fl.set_hash_length(64); // show 64 chars of hash, to minimize chance of collision
    fl.set_sep('\t'); // use tab as separator
    fl.set_all(false); // don't track hidden files
    fl.set_hash_directory(true); // track directories
    fl.set_relative_to(root); // output everything relative to the root, so that this works even if root folder is moved
    fl.set_use_progress_bar(true); // show progress bar, so that user knows how much to wait
    fl.set_force(true); // replace output file if it already exists
    fl
}

fn mkdir(path: impl AsRef<Path>) {
    match fs::create_dir(&path) {
        Ok(_) => println!("Created '{}' directory", path.as_ref().display()),
        Err(e) => {
            eprintln!(
                "fatal: Failed to create '{}' directory: {e}",
                path.as_ref().display(),
            );
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

fn read_dir(path: impl AsRef<Path>) -> fs::ReadDir {
    let read = fs::read_dir(&path);
    match read {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "fatal: Failed to read '{}' directory: {e}",
                path.as_ref().display(),
            );
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

fn get_current_dir() -> PathBuf {
    let path = std::env::current_dir();
    match path {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}
