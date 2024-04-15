<div align="center">

# rip2

### A safer, rust-based `rm`

[![crates](https://img.shields.io/crates/v/rip2.svg)](https://crates.io/crates/rip2)
[![Test](https://github.com/MilesCranmer/rip2/actions/workflows/test.yml/badge.svg)](https://github.com/MilesCranmer/rip2/actions/workflows/test.yml)
[![Coverage Status](https://coveralls.io/repos/github/MilesCranmer/rip2/badge.svg?branch=master)](https://coveralls.io/github/MilesCranmer/rip2?branch=master)

</div>

`rip` is a rust-based `rm` with a focus on safety, ergonomics, and performance.  It favors a simple interface, and does *not* implement the xdg-trash spec or attempt to achieve the same goals.

Deleted files get sent to the graveyard 🪦 (Usually `/tmp/graveyard-$USER`, see [notes](#notes) on changing this) under their absolute path, giving you a chance to recover them 🧟. No data is overwritten. If files that share the same path are deleted, they will be renamed as numbered backups.

This version, "rip2", is a fork-of-a-fork:

1. [nivekuil/rip](https://github.com/nivekuil/rip), the original, which has been unmaintained since 2020.
2. [StandingPadAnimation/rip](https://github.com/StandingPadAnimations/rip) who added a few features.
3. Finally, that repo was forked [@here](https://github.com/MilesCranmer/rip2) with ongoing maintenance:
    - Windows support
    - Refactoring to modern rust
    - Test suite and coverage
    - Shell completions
    - Added a few unmerged PRs from the original repo
    - Bug fix for FIFO files
    - Bug fix for seance

## ⚰️ Installation

This package is supported on Linux, macOS, and Windows.

To install it, first [install Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html).

Then, install this package with cargo:

```bash
$ cargo install --locked rip2
```

No binaries are made available at this time.

## Usage

```text
Usage: rip [OPTIONS] [TARGETS]... [COMMAND]

Commands:
  completions  Generate shell completions file for the specified shell
  help         Print this message or the help of the given subcommand(s)

Arguments:
  [TARGETS]...  File or directory to remove

Options:
      --graveyard <GRAVEYARD>  Directory where deleted files rest
  -d, --decompose              Permanently deletes the graveyard
  -s, --seance                 Prints files that were deleted in the current working directory
  -u, --unbury                 Restore the specified files or the last file if none are specified
  -i, --inspect                Print some info about TARGET before burying
  -h, --help                   Print help
  -V, --version                Print version
```

Basic usage -- easier than rm

```bash
$ rip dir1/ file1
```

Undo the last deletion

```bash
$ rip -u
Returned /tmp/graveyard-jack/home/jack/file1 to /home/jack/file1
```

Print some info (size and first few lines in a file, total size and first few files in a directory) about the target and then prompt for deletion

```bash
$ rip -i file1
dir1: file, 1337 bytes including:
> Position: Shooting Guard and Small Forward ▪ Shoots: Right
> 6-6, 185lb (198cm, 83kg)
Send file1 to the graveyard? (y/n) y
```

Print files that were deleted from under the current directory

```bash
$ rip -s
/tmp/graveyard-jack/home/jack/file1
/tmp/graveyard-jack/home/jack/dir1
```

Name conflicts are resolved

```bash
$ touch file1
$ rip file1
$ rip -s
/tmp/graveyard-jack/home/jack/dir1
/tmp/graveyard-jack/home/jack/file1
/tmp/graveyard-jack/home/jack/file1~1
```

-u also takes the path of a file in the graveyard

```bash
$ rip -u /tmp/graveyard-jack/home/jack/file1
Returned /tmp/graveyard-jack/home/jack/file1 to /home/jack/file1
```

Combine -u and -s to restore everything printed by -s

```bash
$ rip -su
Returned /tmp/graveyard-jack/home/jack/dir1 to /home/jack/dir1
Returned /tmp/graveyard-jack/home/jack/file1~1 to /home/jack/file1~1
```

## Notes

**Aliases.**

You probably shouldn't alias `rm` to `rip`.  Unlearning muscle memory is hard, but it's harder to ensure that every `rm` you make (as different users, from different machines and application environments) is the aliased one.

What I instead recommend is aliasing `rm` to an echo statement that simply reminds you to use `rip`:

```bash
alias rm="echo Use 'rip' instead of rm."
```

**Graveyard location.**

If you have `$XDG_DATA_HOME` environment variable set, `rip` will use `$XDG_DATA_HOME/graveyard` instead of the `$TMPDIR/graveyard-$USER`.

If you want to put the graveyard somewhere else (like `~/.local/share/Trash`), you have two options, in order of precedence:

  1. Alias `rip` to `rip --graveyard ~/.local/share/Trash`
  2. Set the environment variable `$RIP_GRAVEYARD` to `~/.local/share/Trash`.

This can be a good idea because if the graveyard is mounted on an in-memory file system (as `/tmp` is in Arch Linux), deleting large files can quickly fill up your RAM. It's also much slower to move files across file systems, although the delay should be minimal with an SSD.

**Miscellaneous.**

In general, a deletion followed by a `--unbury` should be idempotent.

The deletion log is kept in `.record`, found in the top level of the graveyard.
