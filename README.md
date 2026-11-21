<div align="center">

<img src="assets/logo-512.png" width="96" alt="fileforge">

# fileforge

**A Rust CLI that tidies a folder by moving files into per-extension subfolders.**

[![CI](https://github.com/marcus-beauregard/fileforge/actions/workflows/ci.yml/badge.svg)](https://github.com/marcus-beauregard/fileforge/actions/workflows/ci.yml)
[![License: GPL-2.0-or-later](https://img.shields.io/badge/license-GPL--2.0--or--later-blue.svg)](LICENSE)

</div>

## Quick start

```bash
cargo build --release
./target/release/fileforge ~/Downloads --dry-run    # print the plan, move nothing
./target/release/fileforge ~/Downloads              # do it
```

Or install it on your PATH:

```bash
cargo install --path .
fileforge ~/Downloads --dry-run
```

## What it does

Reads the directory, groups files by extension, and moves each into a subfolder named after that extension — ```.png``` becomes ```png/```. ```--dry-run``` prints the plan and exits without touching anything.

Destination collisions are checked before the rename, because ```rename(2)``` would otherwise replace an existing file without a word.

## Develop

```bash
cargo test
cargo fmt --check
```

## Known gaps

Nested subdirectories are ignored (#5), and the collision check and the rename are separate calls, leaving a window between them (#6).

## Contributing

Humans and automated agents are both welcome. There is no CLA — every commit carries a [DCO](https://developercertificate.org/) sign-off (**git commit -s**).

Start with a [good first issue](https://github.com/marcus-beauregard/fileforge/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22), or read [CONTRIBUTING.md](CONTRIBUTING.md).

## Licence

GPL-2.0-or-later — see [LICENSE](LICENSE). Copyright (C) 2026 Marcus Beauregard.
