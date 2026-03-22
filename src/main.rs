use clap::{Command, arg, command, value_parser};
use fl::FL;

fn main() {
    let matches = get_clap_cmd().get_matches();
    let auto_update = matches.get_flag("update");

    match matches.subcommand() {
        Some(("init", _)) => {
            FL::init();
        }
        Some(("update", _)) => {
            FL::in_current_dir().update();
        }
        Some(("diff", sub)) => {
            let fl = FL::in_current_dir();
            if auto_update {
                fl.update();
            }
            let first = *sub.get_one::<i32>("FIRST").unwrap();
            match sub.get_one::<i32>("SECOND") {
                Some(&second) => fl.diff_history(first, second),
                None => fl.diff_stage(first),
            }
        }
        Some(("commit", _)) => {
            let mut fl = FL::in_current_dir();
            if auto_update {
                fl.update();
            }
            fl.commit();
        }
        _ => {}
    }
}

fn get_clap_cmd() -> Command {
    command!()
        .arg(arg!(-u --update "Automatically update the repo, this is same as `update` command, but you can pair it with other commands").overrides_with("no-update"))
        .arg(arg!(-U --"no-update" "Don't automatically update the repo, this just cancels out --update flag and has no effect on `update` command").overrides_with("update"))
        .subcommand(Command::new("init").about("Initialize a new fl repo in current directory"))
        .subcommand(Command::new("update").about("Update the repo, so all new changes are tracked"))
        .subcommand(
            Command::new("diff")
                .about("Print what has changed between 2 commits")
                .arg(
                    arg!([FIRST] "First commit")
                        .default_value("-1")
                        .value_parser(value_parser!(i32))
                        .allow_negative_numbers(true),
                )
                .arg(
                    arg!([SECOND] "Second commit (STAGE by default)")
                        .value_parser(value_parser!(i32))
                        .allow_negative_numbers(true),
                ),
        )
        .subcommand(Command::new("commit").about("Commit changes"))
}
