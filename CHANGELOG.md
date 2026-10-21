# Changelog

Each herdsman release names the herdr release it is built on. Versions are herdsman's own;
`MAINTAINING.md` explains how they are numbered.

## Unreleased

## [0.3.0] - 2026-10-21

Based on herdr 0.9.5 (`2ecf19a9`), as 0.2.0 is.

### Added

- Tab presets: save a tab's layout under a name with Save tab as preset... in its right-click menu or
  `herdsman preset save`, and open it again as a new tab in any workspace from the `prefix+t` list or
  `herdsman preset load`. Each pane starts in the workspace's folder and runs its saved command, such as
  an agent. A preset is a TOML file in `~/.config/herdsman/presets/`, editable from the list with `e` or
  with `herdsman preset edit`.
- Preset parameters: a preset's `[params]` table declares values it takes when it opens, given with
  `herdsman preset load <name> --param <name>=<value>` or asked for by the presets list. Every pane gets them
  as environment variables, such as `$PR`, for its command to use.
- Tokyo Night Storm and Moon themes, `tokyo-night-storm` and `tokyo-night-moon`, which switch to Tokyo
  Night Day in light mode. All four Tokyo Night themes now take their colours from the official
  tokyonight.nvim palettes.
- The agents panel collapses to its header, so the spaces list gets the sidebar's height: click its title or
  press `prefix+a` (`keys.toggle_agent_panel`) to collapse or open it, and set
  `[ui] agent_panel_start_collapsed = true` to start collapsed.

## [0.2.0] - 2026-10-15

Based on herdr 0.9.5 (`2ecf19a9`).

### From herdr 0.9.5

A hotfix release for herdr 0.9.4, which herdsman 0.1.0 is already based on.

#### Fixed

- Prefix shortcuts work again when the terminal sends the next key as plain text. This affected
  Tern, Chinese, Japanese and Korean input methods on macOS, and Ghostty `text:` keybinds that send
  the prefix and a key together. Workspace navigation had the same problem.
  ([herdr#5255](https://github.com/herdrdev/herdr/issues/5255))
- Codex sessions no longer start with a "clamping Interrupt hook timeout" warning. Herdsman no
  longer installs Codex turn hooks, and Codex status comes from screen detection again. Run
  `herdsman integration install codex` to update.
  ([herdr#5246](https://github.com/herdrdev/herdr/issues/5246),
  [herdr#5278](https://github.com/herdrdev/herdr/issues/5278))
- Claude Code questions show as blocked in named sessions.
  ([herdr#4573](https://github.com/herdrdev/herdr/issues/4573),
  [herdr#5192](https://github.com/herdrdev/herdr/issues/5192))
- On Windows, plugin panes start even when a folder on `PATH` has the same name as the plugin's
  program. ([herdr#5267](https://github.com/herdrdev/herdr/issues/5267),
  [herdr#5269](https://github.com/herdrdev/herdr/issues/5269), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- On Windows, Claude Code session tracking works when the user folder name contains non-ASCII
  characters, such as Korean. ([herdr#5253](https://github.com/herdrdev/herdr/issues/5253),
  [herdr#5270](https://github.com/herdrdev/herdr/issues/5270), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Adding a Windows machine no longer prints raw PowerShell progress output.
  ([herdr#4639](https://github.com/herdrdev/herdr/issues/4639),
  [herdr#5282](https://github.com/herdrdev/herdr/issues/5282), thanks
  [@JJLiebig](https://github.com/JJLiebig))

## [0.1.0] - 2026-10-14

Based on herdr 0.9.4 (`7e26bad9`).

herdsman's first release numbered 0.x. It replaces 1.0.0 and 1.1.0, which were withdrawn when
herdsman moved to 0.x, and carries everything they had. What herdr itself changed up to this release
is in [herdr's v0.9.4 release notes](https://github.com/herdrdev/herdr/releases/tag/v0.9.4) and the
releases before it.

### Added

- herdsman is its own app. The binary, `~/.config/herdsman`, the socket, the `HERDSMAN_*`
  environment variables and the agent hooks all carry its name, so it runs beside herdr without
  sharing anything.
- Stacked panes: one layout slot holds several panes and shows one at a time, while the hidden
  ones keep running with their scrollback, working directory and agent state. `prefix+alt+c` adds
  a member behind the current pane, `prefix+alt+n`, `prefix+alt+p` and `prefix+alt+1..9` switch,
  and a strip along the slot names the members; clicking a segment shows it. A line separates the
  strip from the content; `ui.stack_strip_separator = false` turns it off.
- `herdsman pane stack`, `pane stacks`, `pane focus-stacked`, `pane focus <id>` and
  `pane move --stack` drive stacks from the CLI, and the `prefix+g` navigator tags stacked and
  hidden panes.
- Equal pane sizes: `prefix+|` gives every column in the tab the same width and `prefix+_` every
  row the same height, each leaving the other dimension alone. `herdsman tab equalize width|height`
  and the `layout.equalize` API method do the same from scripts.
- A pane's right-click menu offers Stack here, which opens a new pane in that pane's slot, and Equal
  widths and Equal heights when the tab is split that way.
- Worktree sections: sort a repository's worktrees into named sections with Section... in a
  worktree's right-click menu or with `herdsman workspace section`. Each section sits under a
  header in the repository's sidebar tree and collapses on a click.
- `herdsman integration install claude`, `codex` and `opencode` also install herdsman's agent skill
  for that agent, and `herdsman update` keeps those copies matched to the new binary. A skill
  herdsman did not write, such as a link into a dotfiles checkout, is left alone.
- A plugin manifest may give `min_herdr_version`, the herdr release the plugin needs, instead of
  `min_herdsman_version` or beside it. herdsman checks it against the herdr release it is built on,
  which `herdsman --version` prints on a second line, so plugins written for herdr name the version
  they mean.
- herdsman's own releases: an install script, a Homebrew tap, signed apt, dnf and pacman
  repositories, mise, Nix, and a preview channel fed by every push to `test`. `herdsman update`
  and the in-app update notice point apt, dnf and pacman installs at their package manager, and
  Homebrew installs receive update notices. Remote machines with herdsman installed through mise
  are recognised when attaching.
