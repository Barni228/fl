#![allow(dead_code)]

//! [`std::fs`] functions, but instead of returning [`std::io::Result`] it exits with nice errors

use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn read_to_string(path: impl AsRef<Path>) -> String {
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

pub fn open(path: impl AsRef<Path>) -> fs::File {
    match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "fatal: Failed to open '{}' file: {e}",
                path.as_ref().display()
            );
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    match fs::copy(&from, &to) {
        Ok(_) => println!(
            "Copied '{}' to '{}'",
            from.as_ref().display(),
            to.as_ref().display()
        ),
        Err(e) => {
            eprintln!(
                "fatal: Failed to copy '{}' to '{}': {e}",
                from.as_ref().display(),
                to.as_ref().display()
            );
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

pub fn find_root_path() -> PathBuf {
    let dir = current_dir();

    match find_fl_path(dir) {
        Some(p) => p,
        None => {
            eprintln!("fatal: not inside an fl repository (or any of the parent directories)");
            std::process::exit(1);
        }
    }
}

pub fn find_fl_path(mut dir: PathBuf) -> Option<PathBuf> {
    loop {
        // if dir contains `.fl` folder, return it
        if dir.join(".fl").is_dir() {
            return Some(dir);
        }

        // go one level up, or if there are no more parents then return None
        if !dir.pop() {
            return None;
        }
    }
}

pub fn create_dir(path: impl AsRef<Path>) {
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

pub fn create_file(path: impl AsRef<Path>) {
    match fs::write(&path, "") {
        Ok(_) => {}
        Err(e) => {
            eprintln!("fatal: Failed to create '{}': {e}", path.as_ref().display(),);
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
    match fs::write(&path, content) {
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "fatal: Failed to write to '{}': {e}",
                path.as_ref().display(),
            );
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

pub fn read_dir(path: impl AsRef<Path>) -> fs::ReadDir {
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

pub fn current_dir() -> PathBuf {
    let path = std::env::current_dir();
    match path {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}
