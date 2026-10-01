# Contributing to herdsman

herdsman has a single maintainer, so this page is about making your effort count.

## Issues

Bug reports and ideas are welcome as GitHub issues. For a bug, include `herdsman --version`, your
operating system and terminal, and the shortest steps that reproduce it.

herdsman is built on [herdr](https://github.com/herdrdev/herdr) and takes in its releases. If a bug
also happens in herdr, report it there too: a fix in herdr reaches herdsman with the next merge.

## Pull requests

- **Small fixes**, such as a bug, a typo or a missing test: open a pull request directly.
- **Features and larger changes**: open an issue first, so we agree on the approach before you
  write code. A pull request for a feature nobody discussed may be declined, however good it is.
- Most of the code is merged in from herdr releases. Keep changes to that code small and focused,
  so later merges stay cheap; [MAINTAINING.md](MAINTAINING.md) explains how merges work.

## Building and testing

You need rustup, which installs the toolchain pinned in `rust-toolchain.toml`, plus Zig 0.16.0,
[just](https://github.com/casey/just) and [bun](https://bun.sh) on `PATH`.

```bash
cargo build --release --locked    # binary at target/release/herdsman
just lint                         # formatting and clippy
just test                         # unit, integration and maintenance tests
just install-hooks                # commit message and formatting hooks
```

## Commits

Use lowercase [conventional commit](https://www.conventionalcommits.org) subjects, such as
`fix: handle pane focus`; CI checks them. When a commit relates to an issue, add
`refs #<number>` in the body.

## Coding agents

Agents working on the code read [AGENTS.md](AGENTS.md); `CLAUDE.md` links to it.

## Licence

herdsman is licensed under Apache-2.0. Unless you state otherwise, your contribution is licensed
the same way, as section 5 of [LICENSE](LICENSE) says, so no separate agreement is needed.
