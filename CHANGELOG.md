# Changelog

Each herdsman release names the herdr release it is built on. Versions are herdsman's own;
`MAINTAINING.md` explains how they are numbered.

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
