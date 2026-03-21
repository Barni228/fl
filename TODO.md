- [ ] Don't panic when `.fl/` folder is invalid (does not contain `history/`, or has weird permissions)
- [ ] maybe create a `fix` command that fixes invalid `.fl/` folder
- [ ] Print in colors
- [ ] maybe store history paths in hex numbers instead of regular numbers (`{:08x}`)
- [ ] Add something like `check`, that checks if everything is alright
      so it will print warnings if some file hashes are `ERROR: ...`, or if fl repo is broken

## Done

- [x] Print what changed in git like way (A added.rs, R deleted.rs, M edited.rs)
- [x] store history changes in a relative way (not absolute, so when i move root folder nothing breaks)
