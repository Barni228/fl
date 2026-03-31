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
            FL::in_current_dir()?.update()?;
        }
        Some(("status", _)) => {
            let mut fl = FL::in_current_dir()?;
            if auto_update {
                fl.update()?;
            }
            // Don't print dir changes, because it will print the files that got changed anyway
            // This will make it feel more like `git status`
            fl.ignore_dir_modifications = true;
            fl.diff_stage(-1)?;
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
            arg!(-u --update "Automatically update the repo, \
                this will run `update` command, if the command you are running depends on it")
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
                .alias("s")
                .alias("st"),
            Command::new("diff")
                .about("Print what has changed between 2 commits")
                .alias("d")
                .args([
                    arg!([FIRST] "First commit")
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
                    arg!([MESSAGE] "Commit message, first line will be used as title while all other lines will be used as body"),
                    arg!(-e --empty "Commit with no message"),
                ]),
            Command::new("log").about("Print history log").alias("l"),
            Command::new("pwd")
                .about("Print the current fl repo path")
                .alias("p"),
        ])
}
