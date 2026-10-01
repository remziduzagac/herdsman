<p align="center"><img src="assets/logo.svg" alt="" width="96"></p>
<h1 align="center">herdsman</h1>

A terminal workspace for running coding agents: workspaces, tabs and panes, SSH-connected
machines, and agents recognised by state in the sidebar. On top of that, **stacked panes**:
one layout slot holds several panes and shows one at a time, while the hidden ones keep running.

herdsman is built on [herdr](https://github.com/herdrdev/herdr): a modified version, renamed into
a separate app, that keeps taking in herdr's releases. It is not affiliated with or endorsed by
herdr. See [NOTICE](NOTICE).

## Status

Early. There are no releases yet, so `herdsman update` and remote installs have nothing to fetch.
[CHANGELOG.md](CHANGELOG.md) lists what is coming in 1.0.0.

## How it differs from herdr

- **Its own app.** The binary, `~/.config/herdsman`, the socket, the `HERDSMAN_*` environment
  variables and the agent hooks all carry herdsman's name, so herdsman and herdr run side by side
  without sharing anything.
- **Stacked panes.** `prefix+alt+c` puts a new pane behind the current one, and a strip along
  the slot switches between them. [The stacks page](docs/next/website/src/content/docs/stacks.mdx)
  has the keys, CLI and behaviour.

## Building

```bash
cargo build --release --locked     # binary at target/release/herdsman
```

The vendored libghostty-vt needs Zig 0.16.0 on `PATH`. [CONTRIBUTING.md](CONTRIBUTING.md) lists
the rest of the tools and how to run the tests.

## Documentation

- [User docs](docs/next/website/src/content/docs/): install, configuration, keys, agents, the CLI
  and the socket API.
- `herdsman --help` for the CLI, and `herdsman --skill` for the agent skill.
- [MAINTAINING.md](MAINTAINING.md): branches, versions, and how herdr releases are merged in.
- [CONTRIBUTING.md](CONTRIBUTING.md): issues, pull requests, building and testing.

## Licence

Apache License 2.0, as herdr. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
