use assert_cmd::Command;
use fl::{FL, commit::Commit, config};
use std::{collections::BTreeMap, fs, path::Path};

/// This macro takes a json map, and returns a [`BTreeMap<PathBuf, String>`]
macro_rules! snapshot {
    ($($tt:tt)*) => {{
        let value = serde_json::json!({ $($tt)* });
        // serde_json::from_value::<BTreeMap<PathBuf, String>>(value).unwrap()
        ::serde_json::from_value::<
            ::std::collections::BTreeMap<::std::path::PathBuf, ::std::string::String>
        >(value).unwrap()
    }};
}
// --- helpers ------------------------------------------------------------------

/// Create a new [`assert_cmd::Command`] and return the [`assert_cmd::assert::Assert`]
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
        // clear env vars, so RUST_BACKTRACE is not set (if it is, errors will be messed up)
        .env_clear()
        .assert()
}

#[must_use]
fn new_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    FL::create_fl_repo(dir.path().to_path_buf()).unwrap();
    dir
}

// --- init ---------------------------------------------------------------------

#[test]
fn test_cli_init() {
    for init_cmd in ["init", "i"] {
        let dir = tempfile::tempdir().unwrap();
        cmd(dir.path(), [init_cmd]).success();

        // there should be a valid fl repo at that path
        FL::new(dir.path().to_path_buf(), false).unwrap();
    }
}

#[test]
fn test_cli_init_twice_fails() {
    let dir = tempfile::tempdir().unwrap();
    cmd(dir.path(), ["init"]).success();
    cmd(dir.path(), ["init"]).failure();
}

// --- update -------------------------------------------------------------------

#[test]
fn test_cli_update() {
    for update_cmd in ["update", "u"] {
        let dir = new_repo();

        fs::write(dir.path().join("file.txt"), "hello").unwrap();
        cmd(dir.path(), [update_cmd]).success();

        let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
        let stage = Commit::load_from(fl.stage_path()).unwrap();

        assert_eq!(
            Commit {
                title: None,
                body: None,
                timestamp: stage.timestamp,
                snapshot: snapshot! {
                   ".": "d7914fe546b684688bb95f4f888a92dfc680603a75f23eb823658031fff766d9",
                   "file.txt": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                },
            },
            stage
        );
    }
}

// --- commit -------------------------------------------------------------------

#[test]
fn test_cli_commit() {
    for commit_cmd in ["commit", "c"] {
        let dir = new_repo();
        cmd(dir.path(), [commit_cmd, "title"]).success();

        let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
        assert_eq!(1, fl.commits());

        let commit = fl.get_commit(-1).unwrap();
        assert_eq!(
            Commit {
                title: Some("title".to_string()),
                body: None,
                timestamp: commit.timestamp,
                snapshot: BTreeMap::new(),
            },
            commit
        );
    }
}

#[test]
fn test_cli_commit_empty_flag() {
    for empty_flag in ["--empty", "-e"] {
        let dir = new_repo();
        cmd(dir.path(), ["commit", empty_flag]).success();

        let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
        assert_eq!(1, fl.commits());
        let commit = fl.get_commit(0).unwrap();
        assert_eq!(
            Commit {
                title: None,
                body: None,
                timestamp: commit.timestamp,
                snapshot: BTreeMap::new(),
            },
            commit
        );
    }
}

#[test]
fn test_cli_commit_with_body() {
    let dir = new_repo();
    cmd(dir.path(), ["commit", "title\nbody line"]).success();

    let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    let commit = fl.get_commit(0).unwrap();
    assert_eq!(
        Commit {
            title: Some("title".to_string()),
            body: Some("body line".to_string()),
            timestamp: commit.timestamp,
            snapshot: BTreeMap::new(),
        },
        commit
    );
}

#[test]
fn test_cli_commit_increments_count() {
    let dir = new_repo();

    for _ in 0..3 {
        cmd(dir.path(), ["commit", "-e"]).success();
    }

    let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    assert_eq!(3, fl.commits());
}

#[test]
fn test_cli_commit_timestamp() {
    let dir = new_repo();
    cmd(dir.path(), ["commit", "-e"]).success();

    let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    let commit = fl.get_commit(0).unwrap();

    let now = time::OffsetDateTime::now_utc();
    let diff = (now - commit.timestamp.unwrap()).abs();
    assert!(diff < time::Duration::seconds(2));
}

// --- auto-update flag ---------------------------------------------------------
#[test]
fn test_cli_auto_update() {
    for update_flag in ["-u", "--update"] {
        let dir = new_repo();

        fs::write(dir.path().join("file.txt"), "hello").unwrap();

        // -u should automatically run `update` command before doing the commit
        cmd(dir.path(), [update_flag, "commit", "-e"]).success();

        let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
        assert_eq!(1, fl.commits());

        let commit = fl.get_commit(-1).unwrap();
        assert_eq!(
            Commit {
                title: None,
                body: None,
                timestamp: commit.timestamp,
                snapshot: snapshot! {
                    ".": "d7914fe546b684688bb95f4f888a92dfc680603a75f23eb823658031fff766d9",
                    "file.txt": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                },
            },
            commit
        );
    }
}

#[test]
fn test_cli_no_update_cancels_update() {
    let dir = new_repo();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    // -U should cancel out -u, because it is more recent
    cmd(dir.path(), ["-u", "-U", "commit", "-e"]).success();

    let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    // commit was made but snapshot is empty because update was NOT run
    let commit = fl.get_commit(0).unwrap();
    assert_eq!(
        Commit {
            title: None,
            body: None,
            timestamp: commit.timestamp,
            snapshot: BTreeMap::new(),
        },
        commit
    );
}

// --- status -------------------------------------------------------------------

#[test]
fn test_cli_status() {
    for status_cmd in ["status", "st"] {
        let dir = new_repo();
        cmd(dir.path(), ["-u", "commit", "-e"]).success();
        fs::write(dir.path().join("file.txt"), "hello").unwrap();

        cmd(dir.path(), ["-u", status_cmd])
            .success()
            .stdout("A  file.txt\n");
    }
}

#[test]
fn test_cli_status_no_changes() {
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    cmd(dir.path(), ["-u", "status"])
        .success()
        .stdout("No changes\n");
}

#[test]
fn test_cli_status_shows_modification() {
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    fs::create_dir(dir.path().join("dir")).unwrap();
    fs::write(dir.path().join("dir").join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "status"]).success().stdout(
        "\
        A  dir/\n\
        A  dir/file.txt\n",
    );

    cmd(dir.path(), ["-u", "commit", "-e"]).success();
    fs::write(dir.path().join("dir").join("file.txt"), "changed").unwrap();
    cmd(dir.path(), ["-u", "status"])
        .success()
        // this will only say that the file got modified, even though technically dir was also modified
        // since this feels nicer
        .stdout("M  dir/file.txt\n");
}

#[test]
fn test_cli_status_shows_deletion() {
    let dir = new_repo();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();
    fs::remove_file(dir.path().join("file.txt")).unwrap();

    cmd(dir.path(), ["-u", "status"])
        .success()
        .stdout("D  file.txt\n");
}

#[test]
fn test_cli_status_no_commits_yet() {
    // status with no commits should diff against empty
    let dir = new_repo();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "status"]).success().stdout(
        "\
        A  ./\n\
        A  file.txt\n",
    );
}

// --- diff ---------------------------------------------------------------------

#[test]
fn test_cli_diff_stage_default() {
    for diff_cmd in ["diff", "d"] {
        // `diff` with no args diffs -1 (last commit) against stage
        let dir = new_repo();
        cmd(dir.path(), ["-u", "commit", "-e"]).success();
        cmd(dir.path(), ["-u", diff_cmd]).success().stdout(
            "\
            Diffing 0 and STAGE\n\
            No changes\n",
        );
    }
}

#[test]
fn test_cli_diff_between_commits() {
    let dir = new_repo();
    // commit 0: empty
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    // commit 1: add file
    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    cmd(dir.path(), ["diff", "0", "1"]).success().stdout(
        "\
        Diffing 0 and 1\n\
        M  ./\n\
        A  file.txt\n",
    );
}

#[test]
fn test_cli_diff_negative_index() {
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    cmd(dir.path(), ["diff", "-1", "-2"]).success().stdout(
        "\
        Diffing 0 and 1\n\
        No changes\n",
    );
}

#[test]
fn test_cli_diff_reversed_order_same_result() {
    // diff always compares older to newer, so 0 1 and 1 0 should give same output
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();
    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    cmd(dir.path(), ["diff", "0", "1"]).stdout(
        "\
        Diffing 0 and 1\n\
        M  ./\n\
        A  file.txt\n",
    );
    cmd(dir.path(), ["diff", "1", "0"]).stdout(
        "\
        Diffing 0 and 1\n\
        M  ./\n\
        A  file.txt\n",
    );
}

#[test]
fn test_cli_diff_invalid_commit_fails() {
    let dir = new_repo();
    // no commits exist, so index 99 is invalid
    cmd(dir.path(), ["diff", "99"]).failure();
}

#[test]
fn test_cli_diff_invalid_commit_error_message() {
    let dir = new_repo();
    // no commits exist, so index 99 is invalid
    cmd(dir.path(), ["diff", "99"])
        .failure()
        .stderr("Error:   × fatal: Invalid commit index: 99 (no commits exist)\n\n");
    cmd(dir.path(), ["commit", "-e"]).success();

    cmd(dir.path(), ["diff", "99"])
        .failure()
        .stderr("Error:   × fatal: Invalid commit index: 99 (must be between -1 and 0)\n\n");
}

#[test]
fn test_cli_diff_shows_rename() {
    let dir = new_repo();

    fs::write(dir.path().join("old.txt"), "content").unwrap();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();

    fs::remove_file(dir.path().join("old.txt")).unwrap();
    fs::write(dir.path().join("new.txt"), "content").unwrap();
    cmd(dir.path(), ["-u", "commit", "-e"]).success();
    cmd(dir.path(), ["diff", "0", "1"]).success().stdout(
        "\
        Diffing 0 and 1\n\
        R  \"old.txt\" -> \"new.txt\"\n",
    );
}

// --- log ----------------------------------------------------------------------

#[test]
fn test_cli_log_empty() {
    for log_cmd in ["log", "l"] {
        let dir = new_repo();
        // log with no commits should just succeed and print nothing
        cmd(dir.path(), [log_cmd]).success().stdout("");
    }
}

#[test]
fn test_cli_log_shows_commits() {
    let dir = new_repo();
    cmd(dir.path(), ["commit", "first commit"]).success();
    cmd(dir.path(), ["commit", "second commit"]).success();

    cmd(dir.path(), ["log"]).success().stdout(
        "\
        0: first commit (just now)\n\
        1: second commit (just now)\n",
    );
}

#[test]
fn test_cli_log_no_title_for_empty_commit() {
    let dir = new_repo();
    cmd(dir.path(), ["commit", "-e"]).success();

    cmd(dir.path(), ["log"]).success().stdout(
        "\
        0: <no title> (just now)\n",
    );
}

#[test]
fn test_cli_log_follow() {
    let dir = new_repo();
    cmd(dir.path(), ["commit", "empty commit"]).success();
    cmd(dir.path(), ["-u", "commit", "Added ."]).success();

    fs::write(dir.path().join("file.txt"), "").unwrap();
    cmd(dir.path(), ["-u", "commit", "Added file.txt"]).success();

    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "Modified file.txt"]).success();

    cmd(dir.path(), ["-u", "commit", "Did nothing"]).success();

    fs::write(dir.path().join("file.txt"), "new").unwrap();
    cmd(dir.path(), ["-u", "commit", "Modified file.txt again"]).success();

    cmd(dir.path(), ["-u", "commit", "Unrelated commit"]).success();

    cmd(dir.path(), ["log", "file.txt"]).success().stdout(
        "\
2: Added file.txt (just now):
    A  file.txt
3: Modified file.txt (just now):
    M  file.txt
...
5: Modified file.txt again (just now):
    M  file.txt
",
    );
}

#[test]
fn test_cli_log_follow_renames() {
    let dir = new_repo();
    cmd(dir.path(), ["commit", "empty commit"]).success();
    cmd(dir.path(), ["-u", "commit", "Added ."]).success();

    fs::write(dir.path().join("file.txt"), "").unwrap();
    cmd(dir.path(), ["-u", "commit", "Added file.txt"]).success();

    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "Modified file.txt"]).success();

    fs::rename(dir.path().join("file.txt"), dir.path().join("new_name")).unwrap();

    cmd(dir.path(), ["-u", "commit", "Renamed"]).success();

    fs::write(dir.path().join("new_name"), "new").unwrap();
    cmd(dir.path(), ["-u", "commit", "Modified again"]).success();

    cmd(dir.path(), ["log", "new_name"]).success().stdout(
        r#"2: Added file.txt (just now):
    A  file.txt
3: Modified file.txt (just now):
    M  file.txt
4: Renamed (just now):
    R  "file.txt" -> "new_name"
5: Modified again (just now):
    M  new_name
"#,
    );
}

#[test]
fn test_cli_log_follow_deleted_then_added() {
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "Added ."]).success();

    fs::write(dir.path().join("file.txt"), "").unwrap();
    cmd(dir.path(), ["-u", "commit", "Added file.txt"]).success();

    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "Modified file.txt"]).success();

    cmd(dir.path(), ["-u", "commit", "Did nothing"]).success();

    fs::remove_file(dir.path().join("file.txt")).unwrap();
    cmd(dir.path(), ["-u", "commit", "Removed file.txt"]).success();

    fs::write(dir.path().join("file.txt"), "").unwrap();
    cmd(dir.path(), ["-u", "commit", "Added file.txt again"]).success();

    fs::write(dir.path().join("file.txt"), "hello").unwrap();
    cmd(dir.path(), ["-u", "commit", "Modified file.txt again"]).success();

    cmd(dir.path(), ["log", "file.txt"]).success().stdout(
        "\
5: Added file.txt again (just now):
    A  file.txt
6: Modified file.txt again (just now):
    M  file.txt
",
    );
}

#[test]
fn test_cli_log_follow_no_exist() {
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "Added ."]).success();

    cmd(dir.path(), ["log", "file.txt"]).success().stdout("");
}

#[test]
fn test_cli_log_follow_deleted() {
    let dir = new_repo();
    fs::write(dir.path().join("file.txt"), "").unwrap();
    cmd(dir.path(), ["-u", "commit", "Added file.txt"]).success();

    fs::remove_file(dir.path().join("file.txt")).unwrap();
    cmd(dir.path(), ["-u", "commit", "Removed file.txt"]).success();

    cmd(dir.path(), ["log", "file.txt"]).success().stdout(
        "\
0: Added file.txt (just now):
    A  file.txt
1: Removed file.txt (just now):
    D  file.txt
",
    );
}

// --- show ---------------------------------------------------------------------

#[test]
fn test_cli_show_no_commits() {
    let dir = new_repo();
    cmd(dir.path(), ["show", "0"])
        .failure()
        .stderr("Error:   × fatal: Invalid commit index: 0 (no commits exist)\n\n");
}

#[test]
fn test_cli_show() {
    let dir = new_repo();
    cmd(dir.path(), ["-u", "commit", "Title"]).success();
    let today = time::OffsetDateTime::now_utc().date();

    for i in ["0", "-1"] {
        cmd(dir.path(), ["show", i]).success().stdout(format!(
            "\
0: Title (1 Change) (just now) ({today})
Title:
    Title
Body:
    <no body>

Changes:
A  ./
",
        ));
    }
}

// --- pwd ----------------------------------------------------------------------

#[test]
fn test_cli_pwd() {
    for pwd_cmd in ["pwd", "p"] {
        let dir = new_repo();
        cmd(dir.path(), [pwd_cmd]).success().stdout(format!(
            "{}\n",
            dir.path().canonicalize().unwrap().display()
        ));
    }
}

#[test]
fn test_cli_pwd_from_subdir() {
    let dir = new_repo();

    let subdir = dir.path().join("sub").join("deeper");
    fs::create_dir_all(&subdir).unwrap();
    cmd(subdir, ["pwd"]).success().stdout(format!(
        "{}\n",
        dir.path().canonicalize().unwrap().display()
    ));
}

#[test]
fn test_cli_pwd_no_repo_fails() {
    cmd("/tmp", ["pwd"]).failure();
}

// --- config default -----------------------------------------------------------

#[test]
fn test_cli_config_default_no_args() {
    // config default does not need a repo to work
    let dir = tempfile::tempdir().unwrap();

    cmd(dir.path(), ["config", "default"])
        .success()
        .stdout(config::DEFAULT_CONFIG);
}

#[test]
fn test_cli_config_default_with_args() {
    let dir = tempfile::tempdir().unwrap();

    cmd(dir.path(), ["config", "default", "log.max"])
        .success()
        .stdout("0\n");
}

#[test]
fn test_cli_config_default_invalid() {
    let dir = tempfile::tempdir().unwrap();

    cmd(dir.path(), ["config", "default", "no_exist"])
        .failure()
        .stderr(
            "\
Error:   × Unrecognized key `no_exist`, could not find a default value for it
  ╰─▶ `no_exist` not found

",
        );
}
// --- config get ---------------------------------------------------------------

#[test]
fn test_cli_config_alias() {
    for config_cmd in ["config", "conf", "cfg"] {
        let dir = new_repo();

        cmd(dir.path(), [config_cmd, "get", "log.max"])
            .success()
            .stdout("0\n");
    }
}

#[test]
fn test_cli_config_get() {
    let dir = new_repo();

    cmd(dir.path(), ["config", "get", "log.max"])
        .success()
        .stdout("0\n");

    cmd(dir.path(), ["config", "get", "log.print_title"])
        .success()
        .stdout("true\n");

    cmd(dir.path(), ["config", "get", "color"])
        .success()
        .stdout("\"auto\"\n");
}

#[test]
fn test_cli_config_get_nonexistent_key_fails() {
    let dir = new_repo();
    cmd(dir.path(), ["config", "get", "log.does_not_exist"])
        .failure()
        .stderr(
            "\
Error:   × Config error
  ├─▶ Failed to get `log.does_not_exist` in config file
  ╰─▶ `log.does_not_exist` not found

",
        );
}

// --- config set ---------------------------------------------------------------

#[test]
fn test_cli_config_set() {
    let dir = new_repo();

    fs::write(
        dir.path().join(".fl").join("config.toml"),
        config::DEFAULT_CONFIG,
    )
    .unwrap();

    cmd(dir.path(), ["config", "set", "log.max", "5"])
        .success()
        .stdout(
            "\
            Successfully updated config:\n\
            log.max = 5\n",
        );

    assert_eq!(
        // nothing except max has changed, so all the comments are the same
        config::DEFAULT_CONFIG.replace("max = 0", "max = 5"),
        fs::read_to_string(dir.path().join(".fl").join("config.toml")).unwrap()
    );
}

#[test]
fn test_cli_config_set_bool() {
    let dir = new_repo();

    fs::write(
        dir.path().join(".fl").join("config.toml"),
        config::DEFAULT_CONFIG,
    )
    .unwrap();

    cmd(dir.path(), ["config", "set", "log.print_title", "false"])
        .success()
        .stdout("Successfully updated config:\nlog.print_title = false\n");
    assert_eq!(
        config::DEFAULT_CONFIG.replace("print_title = true", "print_title = false"),
        fs::read_to_string(dir.path().join(".fl").join("config.toml")).unwrap()
    );
}

#[test]
fn test_cli_config_set_str() {
    let dir = new_repo();

    fs::write(
        dir.path().join(".fl").join("config.toml"),
        config::DEFAULT_CONFIG,
    )
    .unwrap();

    cmd(dir.path(), ["config", "set", "color", "never"])
        .success()
        .stdout(
            "Successfully updated config:\n\
            color = \"never\"\n",
        );
    assert_eq!(
        config::DEFAULT_CONFIG.replace(r#"color = "auto""#, r#"color = "never""#),
        fs::read_to_string(dir.path().join(".fl").join("config.toml")).unwrap()
    );
}

#[test]
fn test_cli_config_set_invalid_key_fails() {
    let dir = new_repo();

    cmd(dir.path(), ["config", "set", "nonexistent.key", "value"])
        .failure()
        .stdout("Error Detected, config not updated\n")
        .stderr(
            "\
Error:   × Config error
  ├─▶ Failed to parse config
  ╰─▶ unknown field `nonexistent`, expected one of `color`, `auto_update`,
      `rm_commit_file`, `track`, `editor`, `log`

",
        );
}

#[test]
fn test_cli_config_set_invalid_value_fails() {
    let dir = new_repo();
    // log.max expects a number
    cmd(dir.path(), ["config", "set", "log.max", "not_a_number"])
        .failure()
        .stdout("Error Detected, config not updated\n")
        .stderr(
            "\
Error:   × Config error
  ├─▶ Failed to parse config
  ╰─▶ invalid type: string \"not_a_number\", expected an integer for key
      `log.max`

",
        );
}

#[test]
fn test_cli_config_set_default() {
    let dir = new_repo();

    fs::write(
        dir.path().join(".fl").join("config.toml"),
        config::DEFAULT_CONFIG,
    )
    .unwrap();

    // change, then reset
    cmd(dir.path(), ["config", "set", "log.max", "99"]).success();
    cmd(dir.path(), ["config", "set", "log.max"])
        .success()
        .stdout(
            "Successfully updated config:\n\
            log.max = 0\n",
        );

    assert_eq!(
        config::DEFAULT_CONFIG,
        fs::read_to_string(dir.path().join(".fl").join("config.toml")).unwrap()
    );
}

#[test]
fn test_cli_config_set_default_no_exist_key() {
    let dir = new_repo();
    cmd(dir.path(), ["config", "set", "log.no_exist"])
        .failure()
        .stdout("Error Detected, config not updated\n")
        .stderr(
            "\
Error:   × Config error
  ├─▶ Unrecognized key `log.no_exist`, could not find a default value for it
  ╰─▶ `log.no_exist` not found

",
        );
}

// --- config unset -------------------------------------------------------------

#[test]
fn test_cli_config_unset() {
    let dir = new_repo();

    fs::write(
        dir.path().join(".fl").join("config.toml"),
        config::DEFAULT_CONFIG,
    )
    .unwrap();

    // change, then reset
    cmd(dir.path(), ["config", "set", "log.max", "99"]).success();
    cmd(dir.path(), ["config", "unset", "log.max"])
        .success()
        .stdout("Successfully unset `log.max` (Now its `0`, from default)\n");

    assert_eq!(
        config::DEFAULT_CONFIG.replace(
            "# Maximum number of most recent commits to print, set to 0 to print all\nmax = 0\n",
            ""
        ),
        fs::read_to_string(dir.path().join(".fl").join("config.toml")).unwrap()
    );
}

#[test]
fn test_cli_config_unset_no_exist_key() {
    for unset_cmd in ["unset", "reset"] {
        let dir = new_repo();
        cmd(dir.path(), ["config", unset_cmd, "log.no_exist"])
            .failure()
            .stdout("Error Detected, config not updated\n")
            .stderr(
                "\
Error:   × Config error
  ├─▶ Unrecognized key `log.no_exist`, could not find a default value for it
  ╰─▶ `log.no_exist` not found

",
            );
    }
}

// --- no repo -> failures ------------------------------------------------------

#[test]
fn test_cli_update_no_repo_fails() {
    cmd("/tmp", ["update"]).failure();
}

#[test]
fn test_cli_commit_no_repo_fails() {
    cmd("/tmp", ["commit", "-e"]).failure();
}

#[test]
fn test_cli_status_no_repo_fails() {
    cmd("/tmp", ["status"]).failure();
}

#[test]
fn test_cli_diff_no_repo_fails() {
    cmd("/tmp", ["diff"]).failure();
}

#[test]
fn test_cli_log_no_repo_fails() {
    cmd("/tmp", ["log"]).failure();
}

// --- Everything ---------------------------------------------------------------

#[test]
fn test_cli_full_workflow() {
    let dir = new_repo();

    // add files and commit
    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    cmd(dir.path(), ["update"]).success();
    cmd(dir.path(), ["commit", "add a.txt"]).success();

    // modify file and commit
    fs::write(dir.path().join("a.txt"), "bbb").unwrap();
    cmd(dir.path(), ["update"]).success();
    cmd(dir.path(), ["commit", "modify a.txt"]).success();

    // rename file and commit
    fs::remove_file(dir.path().join("a.txt")).unwrap();
    fs::write(dir.path().join("b.txt"), "bbb").unwrap();
    cmd(dir.path(), ["update"]).success();
    cmd(dir.path(), ["commit", "rename a to b"]).success();

    let fl = FL::new(dir.path().to_path_buf(), false).unwrap();
    assert_eq!(3, fl.commits());

    // diff commit 0 -> 1 should show a modification
    cmd(dir.path(), ["diff", "0", "1"]).success().stdout(
        "Diffing 0 and 1\n\
        M  ./\n\
        M  a.txt\n",
    );

    cmd(dir.path(), ["diff", "1", "2"]).success().stdout(
        "Diffing 1 and 2\n\
        R  \"a.txt\" -> \"b.txt\"\n",
    );

    // log should have 3 entries
    cmd(dir.path(), ["log"]).success().stdout(
        "0: add a.txt (just now)\n\
        1: modify a.txt (just now)\n\
        2: rename a to b (just now)\n",
    );
}
