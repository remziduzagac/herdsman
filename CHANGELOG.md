# Changelog

Each herdsman release names the herdr release it is built on. Versions are herdsman's own;
`MAINTAINING.md` explains how they are numbered.

## Unreleased

### Added

- Equal pane sizes: `prefix+|` gives every column in the tab the same width and `prefix+_` every
  row the same height, each leaving the other dimension alone. `herdsman tab equalize width|height`
  and the `layout.equalize` API method do the same from scripts.
- A pane's right-click menu offers Stack here, which opens a new pane in that pane's slot, and Equal
  widths and Equal heights when the tab is split that way.
- `herdsman integration install claude`, `codex` and `opencode` also install herdsman's agent skill
  for that agent, and `herdsman update` keeps those copies matched to the new binary. A skill
  herdsman did not write, such as a link into a dotfiles checkout, is left alone.

## [1.1.0] - 2026-10-06

Based on herdr 0.9.3 (`347f9c99`).

### Added

- A line separates a stack's strip from the content of its visible member, so the strip no longer
  runs into the program above or below it. `ui.stack_strip_separator = false` turns it off.
- Worktree groups: sort a repository's worktrees into named groups from a worktree's right-click
  menu or with `herdsman workspace group`. Each group sits under a header in the repository's
  sidebar tree and collapses on a click.

### Changed

- `herdsman update` and the in-app update notice point installs from the apt, dnf and pacman
  packages at their package manager, instead of trying to replace a file it owns.

### Fixed

- Homebrew installs, which come from herdsman's tap, receive update notices.
- Remote machines with herdsman installed through mise are recognised when attaching.

## [1.0.0] - 2026-10-01

Based on herdr 0.9.3 (`347f9c99`).

### Added

- Stacked panes: one layout slot holds several panes and shows one at a time, while the hidden
  ones keep running with their scrollback, working directory and agent state. `prefix+alt+c` adds
  a member behind the current pane, `prefix+alt+n`, `prefix+alt+p` and `prefix+alt+1..9` switch,
  and a strip along the slot names the members; clicking a segment shows it.
- `herdsman pane stack`, `pane stacks`, `pane focus-stacked`, `pane focus <id>` and
  `pane move --stack` drive stacks from the CLI, and the `prefix+g` navigator tags stacked and
  hidden panes.

### Changed

- herdsman is its own app. The binary, `~/.config/herdsman`, the socket, the `HERDSMAN_*`
  environment variables and the agent hooks all carry its name, so it runs beside herdr without
  sharing anything.
