use clap::{Command, arg, command, value_parser};
use fl::FL;

fn main() -> anyhow::Result<()> {
    let matches = get_clap_cmd().get_matches();
    let use_global = !matches.get_flag("no-global");

    let get_fl = || -> fl::Result<FL> {
        let mut fl = FL::in_current_dir(use_global)?;
        // only set auto_update if its true
        fl.config.auto_update |= matches.get_flag("update");
        Ok(fl)
    };

    match matches.subcommand() {
        Some(("init", _)) => {
            FL::init()?;
        }
        Some(("update", _)) => {
            let fl = get_fl()?;
            println!("Updating {}", fl.root().display());
            fl.update()?;
        }
        Some(("status", _)) => {
            let fl = get_fl()?;
            fl.status()?;
        }
        Some(("diff", sub)) => {
            let fl = get_fl()?;
            let first = *sub.get_one::<i32>("FIRST").unwrap();
            match sub.get_one::<i32>("SECOND") {
                Some(&second) => fl.diff_history(first, second)?,
                None => fl.diff_stage(first)?,
            }
        }
        Some(("commit", sub)) => {
            let mut fl = get_fl()?;
            let message = sub.get_one::<String>("MESSAGE");
            let empty = sub.get_flag("empty");

            if empty {
                fl.commit_empty()?;
            } else if let Some(m) = message {
                fl.commit_message(m)?;
            } else {
                fl.commit_interactive()?;
            }
        }
        Some(("log", _)) => {
            get_fl()?.print_short_log()?;
        }
        Some(("config", sub)) => match sub.subcommand() {
            Some(("default", _)) => {
                // don't print a new line at the end, so that the file is printed as is
                print!("{}", fl::config::DEFAULT_CONFIG);
            }
            Some(("path", _)) => {
                let config_path = get_fl()?.config_path();
                println!("{}", config_path.display());
            }
            Some(("open", _)) => {
                let fl = get_fl()?;
                fl.open_interactive(fl.config_path())?;
            }
            Some(("get", sub)) => {
                let fl = get_fl()?;
                let key = sub.get_one::<String>("KEY").unwrap();
                println!("{}", fl.get_config_key(key)?);
            }
            Some(("set", sub)) => {
                let mut fl = get_fl()?;
                let key = sub.get_one::<String>("KEY").unwrap();
                if let Some(value) = sub.get_one::<String>("VALUE") {
                    // tell the user that config is not updated if there is an error
                    fl.set_config_key(key, value)
                        .inspect_err(|_| println!("Error Detected, config not updated"))?;
                } else {
                    fl.set_config_key_default(key)
                        .inspect_err(|_| println!("Error Detected, config not updated"))?;
                }
            }
            Some(("unset", sub)) => {
                let mut fl = get_fl()?;
                let key = sub.get_one::<String>("KEY").unwrap();
                fl.unset_config_key(key)
                    .inspect_err(|_| println!("Error Detected, config not updated"))?;
            }
            _ => {}
        },
        Some(("pwd", _)) => {
            println!("{}", get_fl()?.root().display());
        }
        _ => {}
    }

    Ok(())
}

fn get_clap_cmd() -> Command {
    command!()
        .arg_required_else_help(true)
        .args([
            arg!(-u --update "Automatically update the repo, this will run \
                             `update` command, if the command you are running depends on it")
            .overrides_with("no-update"),
            arg!(-U --"no-update" "Don't automatically update the repo, \
                       this just cancels out --update flag and has no effect on `update` command"),
            arg!(--"no-global" "Don't load global config").alias("no-global-config"),
        ])
        .subcommands([
            Command::new("init")
                .about("Initialize a new fl repo in current directory")
                .alias("i"),
            Command::new("update")
                .about("Update the repo, so all new changes are tracked")
                .alias("u"),
            Command::new("status")
                .about("Print changes to files compared to last commit")
                .aliases(["s", "st"]),
            Command::new("diff")
                .about("Print what has changed between 2 commits")
                .alias("d")
                .args([
                    arg!([FIRST] "First commit (can be negative)")
                        .default_value("-1")
                        .value_parser(value_parser!(i32))
                        .allow_negative_numbers(true),
                    arg!([SECOND] "Second commit (STAGE by default)")
                        .value_parser(value_parser!(i32))
                        .allow_negative_numbers(true),
                ]),
            Command::new("commit")
                .about("Commit changes")
                .alias("c")
                .args([
                    arg!([MESSAGE] "Commit message, first line will be used as title, \
                        while all other lines will be used as body"),
                    arg!(-e --empty "Commit with no message"),
                ]),
            Command::new("log").about("Print history log").alias("l"),
            Command::new("config")
                .about("Edit fl config file")
                .aliases(["conf", "cfg"])
                .subcommands([
                    Command::new("default").about("Print default fl config file"),
                    Command::new("path").about("Print path to fl config file"),
                    Command::new("open").about("Open fl config file in editor"),
                    Command::new("get")
                        .about("Get a key from fl config file")
                        .arg(arg!(<KEY> "Key")),
                    Command::new("set")
                        .about("Set a key in fl config file")
                        .arg(arg!(<KEY> "Key to modify"))
                        .arg(arg!([VALUE] "The new value, leave empty to set to default")),
                    Command::new("unset")
                        .about(
                            "Reset a key to its default value, \
                            this is different from `set` without value,\
                            `set` will set something to default, like `log.max = 0`,\
                            `unset` will remove the key from config.toml, as if you never touched it\
                            so if `log.max = 5` in local, and `log.max = 7` in global,\
                            `unset` will remove `log.max` from local, so it becomes `7` from global",
                        )
                        .alias("reset")
                        .arg(arg!(<KEY> "Key to reset to default")),
                ]),
            Command::new("pwd")
                .about("Print the current fl repo path")
                .alias("p"),
        ])
}
