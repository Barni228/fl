# fl

A lightweight, git-inspired version control tool for tracking changes to large files using content hashes.

`fl` snapshots your directory with SHA-256 hashes without the overhead of storing file contents.

---

## Why fl?

Git stores the actual content of every version of every file.
That works great for source code, but becomes painful when you're working with large binary files, datasets, or media assets.
`fl` tracks _what changed and when_ using hashes, without ever duplicating your data.

---

## Installation

### From a GitHub Release (recommended)

Pre-built binaries are available for Linux, macOS, and Windows.

1. Go to the [Releases](https://github.com/Barni228/fl/releases) page
2. Select the most recent version
3. Copy the install command for your platform and run it in your terminal

### From Source

Requires Rust 2024 edition (Rust 1.85+).

```bash
git clone https://github.com/Barni228/fl
cd fl
cargo install --path .
```

---

## Uninstall

There are no special uninstallers, if you downloaded from release, just uninstall the binary:

```bash
rm -i $(which fl)
```

if installed with `cargo`:

```bash
cargo uninstall fl
```

---

## Quick Start

```bash
# Initialize a repo in the current directory
fl init

# Snapshot the current state
fl update

# Commit the snapshot
fl commit "initial snapshot"

# make some changes
# ...
# See what changed (-u basically makes this)
fl update
fl status

# or run both of those commands together:
fl -u status

# Commit again
fl commit "added new data files"
# or `fl -u commit "..."` if you need to update the snapshot

# See the full history
fl log

# See what the most recent commit did
fl show -1
```

---

### Ignore

By default, `fl` will respect `.ignore` files (same as `.gitignore`, just not git specific)
You can optionally also respect `git` ignore files by enabling `ignore_git`:

```bash
fl config set ignore_git true
```

or write

```toml
ignore_git = true
```

In `.fl/config.toml` (open with `fl config open`)

## Commands

| Command                    | Aliases       | Description                                              |
| -------------------------- | ------------- | -------------------------------------------------------- |
| `fl init`                  | `i`           | Initialize a new fl repo in the current directory        |
| `fl update`                | `u`           | Snapshot the current state of all files                  |
| `fl status`                | `s`, `st`     | Show what has changed since the last commit              |
| `fl commit [MESSAGE]`      | `c`           | Commit the current snapshot                              |
| `fl diff [FIRST] [SECOND]` | `d`           | Show changes between two commits (or a commit and stage) |
| `fl log`                   | `l`           | Print commit history                                     |
| `fl pwd`                   | `p`           | Print the root path of the current fl repo               |
| `fl config`                | `conf`, `cfg` | Manage config options                                    |

### Global Flags

| Flag                 | Description                                   |
| -------------------- | --------------------------------------------- |
| `-u`, `--update`     | Automatically run `update` before the command |
| `-U`, `--no-update`  | Cancel a `-u` flag                            |
| `--no-global-config` | Ignore the global `~/.config/fl/config.toml`  |

### commit

```bash
fl commit "title"                   # commit with a title
fl commit "title\nmulti-line body"  # commit with a title and body
fl commit -e                        # commit with no message
fl commit                           # open $EDITOR to write a message
```

Lines starting with `#` in the commit message are treated as comments and ignored.

### diff

```bash
fl diff           # diff last commit against current stage
fl diff 2         # diff commit 2 against stage
fl diff 0 3       # diff commits 0 and 3
fl diff -1 -2     # negative indexes count from the end
```

`diff` always compares older to newer, so `fl diff 3 0` and `fl diff 0 3` produce the same output.

### config

```bash
fl config get log.max           # print a config value
fl config set log.max 10        # change a config value
fl config reset log.max         # reset a value to its default
fl config path                  # print path to the config file
fl config open                  # open the config file in $EDITOR
fl config default               # print the default config
```

---

## Change Types

When running `status` or `diff`, each changed file is prefixed with a letter:

| Prefix | Meaning                   |
| ------ | ------------------------- |
| `A`    | File was added            |
| `D`    | File was deleted          |
| `M`    | File content changed      |
| `R`    | File was renamed or moved |

Rename detection is automatic. If a file is deleted and another file with the same hash appears, `fl` recognizes it as a rename and picks the best match by path similarity.

```
R  "old_name.txt" -> "new_name.txt"
R  "src/widget.rs" -> "lib/widget.rs"
```

---

## Configuration

fl looks for config in two places, merged in order:

1. **Local**: `.fl/config.toml` in the repo root (takes precedence)
2. **Global**: `~/.config/fl/config.toml` (or path in `$FL_GLOBAL_CONFIG`)

Run `fl config default` to see all available options with their defaults,
or read the [Default Config](default_config.toml)

---

## How It Works

### Snapshots

`fl update` uses [filelist](https://github.com/Barni228/filelist) to walk the repo directory
and compute a SHA-256 hash for every file. The result is saved as a JSON snapshot in `.fl/STAGE.json`.

`fl commit` copies that snapshot into `.fl/history/<index>.json` with an optional message and timestamp.

### Repository Layout

```
your-project/
└── .fl/
    ├── config.toml          # local config
    ├── STAGE.json           # current snapshot (updated by `fl update`)
    ├── FL_COMMIT_MESSAGE    # temporary file for interactive commits
    └── history/
        ├── 00000000.json    # first commit
        ├── 00000001.json    # second commit
        └── ...
```

Snapshots store paths relative to the repo root, so the entire project directory can be moved without breaking history.

---

## License

MIT
