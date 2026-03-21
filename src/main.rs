use clap::{Command, arg, command, value_parser};
use fl::FL;

fn main() {
    let matches = get_clap_cmd().get_matches();

    match matches.subcommand() {
        Some(("init", _)) => {
            FL::init();
        }
        Some(("update", _)) => {
            FL::in_current_dir().update();
        }
        // TODO: when I implement the STAGE file, by default compare -1 to STAGE instead of -2
        Some(("diff", sub)) => {
            let first = *sub.get_one::<i32>("FIRST").unwrap();
            let second = *sub.get_one::<i32>("SECOND").unwrap_or(&-2);
            // println!("Diffing {} and {}", first, second);
            FL::in_current_dir().diff_history(first, second);
        }
        Some(("commit", _)) => {
            println!("Commit");
        }
        _ => {}
    }
}

fn get_clap_cmd() -> Command {
    command!()
        .subcommand(Command::new("init").about("Initialize a new fl repo in current directory"))
        .subcommand(Command::new("update").about("Update"))
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
