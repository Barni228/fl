use clap::{Command, arg, command, value_parser};
use fl::FL;

fn main() -> anyhow::Result<()> {
    let matches = get_clap_cmd().get_matches();
    let auto_update = matches.get_flag("update");

    match matches.subcommand() {
        Some(("init", _)) => {
            FL::init()?;
        }
        Some(("update", _)) => {
            let fl = FL::in_current_dir()?;
            println!("Updating {}", fl.root().display());
            fl.update()?;
        }
        Some(("status", _)) => {
            let fl = FL::in_current_dir()?;
            if auto_update {
                fl.update()?;
            }
            fl.status()?;
        }
        Some(("diff", sub)) => {
            let fl = FL::in_current_dir()?;
            if auto_update {
                fl.update()?;
            }
            let first = *sub.get_one::<i32>("FIRST").unwrap();
            match sub.get_one::<i32>("SECOND") {
                Some(&second) => fl.diff_history(first, second)?,
                None => fl.diff_stage(first)?,
            }
        }
        Some(("commit", sub)) => {
            let mut fl = FL::in_current_dir()?;
            if auto_update {
                fl.update()?;
            }
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
            FL::in_current_dir()?.print_short_log()?;
        }
        Some(("config", sub)) => match sub.subcommand() {
            Some(("default", _)) => {
                // don't print a new line at the end, so that the file is printed as is
                print!("{}", fl::config::DEFAULT_CONFIG);
            }
            Some(("path", _)) => {
                let config_path = FL::in_current_dir()?.config_path();
                println!("{}", config_path.display());
            }
            Some(("open", _)) => {
                let fl = FL::in_current_dir()?;
                fl.open_interactive(fl.config_path())?;
            }
            Some(("get", sub)) => {
                let key = sub.get_one::<String>("KEY").unwrap();
                println!("{}", FL::in_current_dir()?.get_config_key(key)?);
            }
            Some(("set", sub)) => {
                let mut fl = FL::in_current_dir()?;
                let key = sub.get_one::<String>("KEY").unwrap();
                let value = sub.get_one::<String>("VALUE").unwrap();

                // tell the user that config is not updated if there is an error
                fl.set_config_key(key, value)
                    .inspect_err(|_| println!("Error Detected, config not updated"))?;
            }
            Some(("reset", sub)) => {
                let key = sub.get_one::<String>("KEY").unwrap();
                let mut fl = FL::in_current_dir()?;
                fl.reset_config_key(key)
                    .inspect_err(|_| println!("Error Detected, config not updated"))?;
            }
            _ => {}
        },
        Some(("pwd", _)) => {
            println!("{}", FL::in_current_dir()?.root().display());
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
                        .arg(arg!(<VALUE> "The new value")),
                    Command::new("reset")
                        .about("Reset a key to its default value")
                        .arg(arg!(<KEY> "Key to reset to default")),
                ]),
            Command::new("pwd")
                .about("Print the current fl repo path")
                .alias("p"),
        ])
}
