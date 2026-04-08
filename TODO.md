## Features

- [ ] Add something like `check`, that checks if everything is alright
      so it will print warnings if some file hashes are `ERROR: ...`, or if fl repo is broken
- [ ] maybe create a `fix` command that fixes invalid `.fl/` folder
- [ ] Add `add` and `remove` (`rm`) commands that will add/remove files from STAGE
- [ ] Add a way to edit commit history
- [ ] Add `show` command which shows what a specific commit did
- [ ] Add `log --follow` command, that shows every commit that changed a specific file
- [ ] Add a way to filter commits, like `-ad` for only add or delete actions
- [ ] Add a way to ignore files/folders, like `.gitignore`
- [ ] Add a `config` command, that stores some config options
- [ ] Allow a global `~/.config/fl/config.toml` config (use `config` crate)
- [ ] Make `config reset` work even if the current config file is broken
- [ ] Make interactive commit save file in fl repo rather than temp file, like `git` (git also doesn't remove it)

## Bugs / Improvements

- [ ] Add more tests, to test things other than `diff`
- [ ] maybe store history paths in hex numbers instead of regular numbers (`{:08x}`)
- [ ] Don't panic when `.fl/` folder is invalid (does not contain `history/`, or has weird permissions)
- [ ] Add meaningful exit codes (instead of using raw_os_error)
- [ ] Maybe make config.rs handle all of the get/set toml stuff with its own errors

## Done

- [x] Use custom error types instead of `anyhow::Result`
- [x] maybe remove dead code from `fs_helper.rs`
- [x] Improve Error handling, so I return `Result` instead of always exiting with error
- [x] Allow commits to have body text
- [x] Improve `filelist` so it can return `BTreeMap` directory, instead of parsing strings
- [x] Make commits have a date and time
- [x] maybe store commit files in some known format like `json`, `toml`, or `yaml`
- [x] Add something like `log`, which prints the history with their messages
- [x] use [PathFinding](https://github.com/samueltardieu/pathfinding) instead of hungarian crate
- [x] Add something like `pwd`, that prints current repo path
- [x] Allow commit messages
- [x] When committing, print how many changes are committed (like `Committing 5 changes`)
- [x] Add tests
- [x] When there are 2 same files (same hash), if u delete one and rename the other,
      it should somehow nicely know which one got renamed and which one got deleted (right now it randomly chooses one)
- [x] Print in colors
- [x] When diffing, print everything sorted by the file path
- [x] Print what changed in git like way (A added.rs, R deleted.rs, M edited.rs)
- [x] store history changes in a relative way (not absolute, so when i move root folder nothing breaks)
