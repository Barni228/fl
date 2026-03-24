## Features

- [ ] Add something like `check`, that checks if everything is alright
      so it will print warnings if some file hashes are `ERROR: ...`, or if fl repo is broken
- [ ] maybe store history paths in hex numbers instead of regular numbers (`{:08x}`)
- [ ] maybe create a `fix` command that fixes invalid `.fl/` folder
- [ ] Add `add` and `remove` (`rm`) commands that will add/remove files from STAGE
- [ ] Add something like `pwd`, that prints current repo path
- [ ] Add something like `log`, which prints the history with their messages

## Bugs / Improvements

- [ ] Don't panic when `.fl/` folder is invalid (does not contain `history/`, or has weird permissions)
- [ ] Improve Error handling, so I return `Result` instead of always exiting with error
- [ ] Add meaningful exit codes (instead of using raw_os_error)

## Done

- [x] Allow commit messages
- [x] When committing, print how many changes are committed (like `Committing 5 changes`)
- [x] Add tests
- [x] When there are 2 same files (same hash), if u delete one and rename the other,
      it should somehow nicely know which one got renamed and which one got deleted (right now it randomly chooses one)
- [x] Print in colors
- [x] When diffing, print everything sorted by the file path
- [x] Print what changed in git like way (A added.rs, R deleted.rs, M edited.rs)
- [x] store history changes in a relative way (not absolute, so when i move root folder nothing breaks)
