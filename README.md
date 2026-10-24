<p align="center"><img src="assets/logo.svg" alt="" width="96"></p>
<h1 align="center">herdsman</h1>
<p align="center"><a href="https://github.com/remziduzagac/herdsman/actions/workflows/ci.yml"><img src="https://github.com/remziduzagac/herdsman/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a></p>

A terminal workspace for running coding agents: workspaces, tabs and panes, SSH-connected
machines, and agents recognised by state in the sidebar. On top of that, **stacked panes**:
one layout slot holds several panes and shows one at a time, while the hidden ones keep running.

![herdsman: a repository's worktrees in sections in the sidebar, and an editor beside a stack of agent panes][shot]

[shot]: docs/next/website/src/assets/screenshots/overview.webp

herdsman is built on [herdr](https://github.com/herdrdev/herdr): a modified version, renamed into
a separate app, that keeps taking in herdr's releases. It is not affiliated with or endorsed by
herdr. See [NOTICE](NOTICE).

## Status

Early. [CHANGELOG.md](CHANGELOG.md) lists each release and the herdr release it is built on.
Preview builds of upcoming changes are available to installs made with the install script, not
to Homebrew, mise, Nix or the package repositories: `herdsman channel set preview`.

## Use at your own risk

herdsman runs shells, commands and coding agents on your machine and on the machines you connect
it to. Agents running in it can use its CLI to type into, start and close other panes, and
features such as live handoff, worktree removal and integration installs change running
processes and files. herdsman makes no guarantee about what the agents it hosts do, or that it is
free of bugs. You use it at your own risk and are responsible for what happens on your machines,
so:

- review what an agent proposes and does before you accept it;
- keep backups, and keep your work under version control;
- use your agent's own permission settings;
- treat features marked experimental, such as live handoff, as ones that can fail.

herdsman is provided "as is", without warranty of any kind and without liability for any damage or
loss, including lost work or data, as sections 7 and 8 of the [licence](LICENSE) say.

## Install

```bash
curl -fsSL https://remziduzagac.github.io/herdsman/install.sh | sh    # Linux, macOS
brew install remziduzagac/tap/herdsman                                 # Homebrew
mise use -g github:remziduzagac/herdsman                               # mise
```

Signed apt, dnf and pacman repositories, Windows, Nix and manual downloads are covered in the
[install guide](https://remziduzagac.github.io/herdsman/docs/install/).

## How it differs from herdr

herdsman is its own app: its binary, `~/.config/herdsman`, socket, `HERDSMAN_*` environment
variables and agent hooks carry its name, so it runs beside herdr without sharing anything. On top
of herdr it adds stacked panes, equal pane sizes, tab presets, worktree sections, a collapsible
agents panel and more.
[What herdsman adds](https://remziduzagac.github.io/herdsman/docs/what-herdsman-adds/) lists every
feature with the release it arrived in.

## Building

```bash
cargo build --release --locked     # binary at target/release/herdsman
```

The vendored libghostty-vt needs Zig 0.16.0 on `PATH`. [CONTRIBUTING.md](CONTRIBUTING.md) lists
the rest of the tools and how to run the tests.

## Documentation

- [Documentation](https://remziduzagac.github.io/herdsman/docs/): install, configuration, keys, agents, the CLI and the socket
  API.
- `herdsman --help` for the CLI, and `herdsman --skill` for the agent skill.
- [MAINTAINING.md](MAINTAINING.md): branches, versions, and how herdr releases are merged in.
- [CONTRIBUTING.md](CONTRIBUTING.md): issues, pull requests, building and testing.

## Licence

Apache License 2.0, as herdr. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
