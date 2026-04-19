/// Tests that each config option is actually respected by the CLI.
use assert_cmd::Command;
use fl::{FL, config};
use predicates::prelude::*;
use std::{fs, path::Path};

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Run `fl` in `dir` with `--no-global-config`
#[must_use]
fn cmd<P, I, S>(dir: P, args: I) -> assert_cmd::assert::Assert
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::cargo_bin("fl")
        .unwrap()
        .current_dir(dir)
        .arg("--no-global-config")
        .args(args)
        .env_clear()
        .assert()
}

/// Run `fl` in `dir` *with* global config (no `--no-global-config`),
/// pointing `FL_GLOBAL_CONFIG` at a specific file.
#[must_use]
fn cmd_global<P, I, S>(dir: P, args: I, global_config_path: &Path) -> assert_cmd::assert::Assert
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::cargo_bin("fl")
        .unwrap()
        .current_dir(dir)
        .args(args)
        .env_clear()
        .env("FL_GLOBAL_CONFIG", global_config_path)
        .assert()
}

#[must_use]
fn new_repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    dir
}

/// Write `content` into `.fl/config.toml` inside `repo`.
fn set_config(repo: &tempfile::TempDir, content: &str) {
    fs::write(repo.path().join(".fl").join("config.toml"), content).unwrap();
}

// functions

#[test]
fn test_config_set() {
    let dir = new_repo();
    let mut fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    // set something without setter
    fl.config.log.max = 10;
    fl.set_config_key("color", "never").unwrap();
    assert_eq!(fl.config.color, config::ColorOptions::Never);
    // log didn't change, since i did not set it
    assert_eq!(fl.config.log.max, 10);
}

#[test]
fn test_config_get() {
    let dir = new_repo();
    let mut fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    fl.config.log.max = 10;
    assert_eq!(fl.get_config_key("log.max").unwrap(), "10");
    assert_eq!(fl.config.log.max, 10);
}

// ─── color ────────────────────────────────────────────────────────────────────

#[test]
fn test_config_color_never() {
    let dir = new_repo();
    set_config(&dir, r#"color = "never""#);

    cmd(dir.path(), ["-u", "status"])
        .success()
        .stdout(predicates::str::contains('\x1b').not());
}

#[test]
fn test_config_color_always() {
    let dir = new_repo();
    set_config(&dir, r#"color = "always""#);

    cmd(dir.path(), ["-u", "status"])
        .success()
        .stdout(predicates::str::contains('\x1b'));
}

#[test]
fn test_config_color_auto() {
    let dir = new_repo();
    set_config(&dir, r#"color = "auto""#);

    // automatically, it will not print colors because stdout is not a tty in tests
    cmd(dir.path(), ["-u", "status"])
        .success()
        .stdout(predicates::str::contains('\x1b').not());
}

#[test]
fn test_config_color_respect_env_vars() {
    let dir = new_repo();

    Command::cargo_bin("fl")
        .unwrap()
        .current_dir(dir.path())
        .args(["--no-global-config", "-u", "status"])
        .env_clear()
        .env("NO_COLOR", "1")
        .assert()
        .stdout(predicates::str::contains('\x1b').not());

    Command::cargo_bin("fl")
        .unwrap()
        .current_dir(dir.path())
        .args(["--no-global-config", "-u", "status"])
        .env_clear()
        .env("CLICOLOR_FORCE", "1")
        .assert()
        .stdout(predicates::str::contains('\x1b'));
}

// ─── auto_update ──────────────────────────────────────────────────────────────

#[test]
fn test_config_auto_update_false() {
    let dir = new_repo();
    set_config(&dir, "auto_update = false");

    // auto update is false, so you have to run `fl update` to see the changes
    cmd(dir.path(), ["status"]).success().stdout("No changes\n");

    cmd(dir.path(), ["update"]).success();
    cmd(dir.path(), ["status"]).success().stdout("A  .\n");
}

#[test]
fn test_config_auto_update_true() {
    let dir = new_repo();
    set_config(&dir, "auto_update = true");

    // auto update is true, so update is run automatically
    cmd(dir.path(), ["status"]).success().stdout("A  .\n");
}

// ─── rm_commit_file ───────────────────────────────────────────────────────────

#[test]
fn test_config_rm_commit_file_false() {
    let dir = new_repo();
    let msg_path = dir.path().join(".fl").join("FL_COMMIT_MESSAGE");
    // Use `true` as the editor so it exits immediately without changing the file.
    set_config(
        &dir,
        "rm_commit_file = false\n\
        editor.command = [\"true\"]",
    );
    cmd(dir.path(), ["commit"]).success();

    assert!(msg_path.exists());
}

#[test]
fn test_config_rm_commit_file_true() {
    let dir = new_repo();
    let msg_path = dir.path().join(".fl").join("FL_COMMIT_MESSAGE");
    set_config(
        &dir,
        "rm_commit_file = true\n\
        editor.command = [\"true\"]",
    );

    cmd(dir.path(), ["commit"]).success();

    assert!(!msg_path.exists());
}

// ─── log.max ──────────────────────────────────────────────────────────────────

#[test]
fn test_config_log_max_0_shows_all() {
    let dir = new_repo();
    set_config(&dir, "[log]\nmax = 0");

    for title in ["first", "second", "third"] {
        cmd(dir.path(), ["commit", title]).success();
    }

    cmd(dir.path(), ["log"]).success().stdout(
        "0: first (just now)\n\
        1: second (just now)\n\
        2: third (just now)\n",
    );
}

#[test]
fn test_config_log_max_2() {
    let dir = new_repo();
    set_config(&dir, "log.max = 2");

    for title in ["first", "second", "third"] {
        cmd(dir.path(), ["commit", title]).success();
    }

    cmd(dir.path(), ["log"]).success().stdout(
        "1: second (just now)\n\
        2: third (just now)\n",
    );
}

#[test]
fn test_config_log_max_1() {
    let dir = new_repo();
    set_config(&dir, "[log]\nmax = 1");

    for title in ["first", "second", "third"] {
        cmd(dir.path(), ["commit", title]).success();
    }

    cmd(dir.path(), ["log"])
        .success()
        .stdout("2: third (just now)\n");
}

#[test]
fn test_config_log_max_too_big() {
    let dir = new_repo();
    set_config(&dir, "[log]\nmax = 999");

    cmd(dir.path(), ["commit", "only"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: only (just now)\n");
}

// ─── log.print_title ──────────────────────────────────────────────────────────

#[test]
fn test_config_log_print_title_true() {
    let dir = new_repo();
    set_config(&dir, "[log]\nprint_title = true");
    cmd(dir.path(), ["commit", "my title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: my title (just now)\n");
}

#[test]
fn test_config_log_print_title_false() {
    let dir = new_repo();
    set_config(&dir, "log.print_title = false");
    cmd(dir.path(), ["commit", "my title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0:  (just now)\n");
}

// ─── log.print_title_quotes ───────────────────────────────────────────────────

#[test]
fn test_config_log_print_title_quotes_false() {
    let dir = new_repo();
    set_config(&dir, "log.print_title_quotes = false");
    cmd(dir.path(), ["commit", "quoted title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: quoted title (just now)\n");
}

#[test]
fn test_config_log_print_title_quotes_true() {
    let dir = new_repo();
    set_config(&dir, "log.print_title_quotes = true");
    cmd(dir.path(), ["commit", "quoted title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: \"quoted title\" (just now)\n");
}

// ─── log.print_number_of_changes ─────────────────────────────────────────────

#[test]
fn test_config_log_print_number_of_changes_false() {
    let dir = new_repo();
    set_config(&dir, "log.print_number_of_changes = false");
    cmd(dir.path(), ["commit", "title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: title (just now)\n");
}

#[test]
fn test_config_log_print_number_of_changes_true() {
    let dir = new_repo();
    set_config(&dir, "log.print_number_of_changes = true");

    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    cmd(dir.path(), ["-u", "commit", "added file"]).success();

    // The commit has changes (the file + root dir), so "(N Changes)" appears.
    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: added file (2 Changes) (just now)\n");
}

#[test]
fn test_config_log_print_number_of_changes_zero() {
    let dir = new_repo();
    set_config(&dir, "log.print_number_of_changes = true");

    // Empty commit → 0 changes relative to previous (also empty).
    cmd(dir.path(), ["commit", "title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: title (0 Changes) (just now)\n");
}

// ─── log.print_time_ago ───────────────────────────────────────────────────────

#[test]
fn test_config_log_print_time_ago_true() {
    let dir = new_repo();
    set_config(&dir, "log.print_time_ago = true");
    cmd(dir.path(), ["commit", "title"]).success();

    // "just now" (or some time-ago string) should appear.
    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: title (just now)\n");
}

#[test]
fn test_config_log_print_time_ago_false() {
    let dir = new_repo();
    set_config(&dir, "log.print_time_ago = false");
    cmd(dir.path(), ["commit", "title"]).success();

    cmd(dir.path(), ["log"]).success().stdout("0: title\n");
}

// ─── log.print_date ───────────────────────────────────────────────────────────

#[test]
fn test_config_log_print_date_false() {
    let dir = new_repo();
    set_config(&dir, "log.print_date = false");
    cmd(dir.path(), ["commit", "title"]).success();

    cmd(dir.path(), ["log"])
        .success()
        .stdout("0: title (just now)\n");
}

#[test]
fn test_config_log_print_date_true() {
    let dir = new_repo();
    set_config(&dir, "log.print_date = true");
    cmd(dir.path(), ["commit", "title"]).success();
    let today = time::OffsetDateTime::now_utc().date();

    cmd(dir.path(), ["log"])
        .success()
        .stdout(format!("0: title (just now) ({today})\n"));
}

#[test]
fn test_config_log_everything() {
    let dir = new_repo();
    // this string is very indented, but that should still be valid toml
    set_config(
        &dir,
        r#"
            [log]
            max = 1
            print_title = true
            print_title_quotes = true
            print_number_of_changes = true
            print_time_ago = true
            print_date = true
    "#,
    );
    cmd(dir.path(), ["commit", "first"]).success();
    cmd(dir.path(), ["-u", "commit", "last"]).success();

    let today = time::OffsetDateTime::now_utc().date();
    cmd(dir.path(), ["log"])
        .success()
        .stdout(format!("1: \"last\" (1 Change) (just now) ({today})\n"));
}

#[test]
fn test_config_log_nothing() {
    let dir = new_repo();
    set_config(
        &dir,
        r#"
            [log]
            max = 0
            print_title = false
            print_title_quotes = false
            print_number_of_changes = false
            print_time_ago = false
            print_date = false
    "#,
    );
    cmd(dir.path(), ["commit", "first"]).success();
    cmd(dir.path(), ["-u", "commit", "last"]).success();

    cmd(dir.path(), ["log"]).success().stdout(
        "0: \n\
        1: \n",
    );
}

// ─── Global config is respected ───────────────────────────────────────────────

#[test]
fn test_global_config() {
    let dir = new_repo();
    // local config says nothing about log.max
    set_config(&dir, "");

    for title in ["first", "second", "third"] {
        cmd(dir.path(), ["commit", title]).success();
    }

    // Global config caps at 1
    let global = tempfile::NamedTempFile::new().unwrap();
    fs::write(global.path(), "log.max = 1\n").unwrap();

    cmd_global(dir.path(), ["log"], global.path())
        .success()
        .stdout("2: third (just now)\n");
}

#[test]
fn test_config_local_overrides_global() {
    let dir = new_repo();

    // local config says there are no limits
    set_config(&dir, "log.max = 0");

    for title in ["first", "second", "third"] {
        cmd(dir.path(), ["commit", title]).success();
    }

    // Global wants only 1, local wants all.
    let global = tempfile::NamedTempFile::new().unwrap();
    fs::write(global.path(), "log.max = 1\n").unwrap();

    cmd_global(dir.path(), ["log"], global.path())
        .success()
        .stdout(
            "0: first (just now)\n\
            1: second (just now)\n\
            2: third (just now)\n",
        );
}

#[test]
fn test_no_global_config_flag_ignores_global() {
    let dir = new_repo();

    for title in ["first", "second", "third"] {
        cmd(dir.path(), ["commit", title]).success();
    }

    let global = tempfile::NamedTempFile::new().unwrap();
    fs::write(global.path(), "log.max = 1\n").unwrap();

    // --no-global-config must ignore the global file entirely.
    cmd_global(dir.path(), ["--no-global-config", "log"], global.path())
        .success()
        .stdout(
            "0: first (just now)\n\
             1: second (just now)\n\
             2: third (just now)\n",
        );
}

#[test]
fn test_global_config_missing_file_is_silently_ignored() {
    let dir = new_repo();

    // tempfile object gets dropped here, so the file goes away
    let no_exist = tempfile::NamedTempFile::new().unwrap().path().to_path_buf();
    assert!(!no_exist.exists());

    cmd(dir.path(), ["commit", "title"]).success();

    // Point FL_GLOBAL_CONFIG at a path that does not exist — should not fail.
    cmd_global(dir.path(), ["log"], &no_exist)
        .success()
        .stdout("0: title (just now)\n");
}
