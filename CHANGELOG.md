# Changelog

Each herdsman release names the herdr release it is built on. Versions are herdsman's own;
`MAINTAINING.md` explains how they are numbered.

## Unreleased

Based on herdr 0.9.4 (`7e26bad9`).

### Added

- Equal pane sizes: `prefix+|` gives every column in the tab the same width and `prefix+_` every
  row the same height, each leaving the other dimension alone. `herdsman tab equalize width|height`
  and the `layout.equalize` API method do the same from scripts.
- A pane's right-click menu offers Stack here, which opens a new pane in that pane's slot, and Equal
  widths and Equal heights when the tab is split that way.
- `herdsman integration install claude`, `codex` and `opencode` also install herdsman's agent skill
  for that agent, and `herdsman update` keeps those copies matched to the new binary. A skill
  herdsman did not write, such as a link into a dotfiles checkout, is left alone.
- A plugin manifest may give `min_herdr_version`, the herdr release the plugin needs, instead of
  `min_herdsman_version` or beside it. herdsman checks it against the herdr release it is built on,
  which `herdsman --version` now prints on a second line, so plugins written for herdr name the
  version they mean.

### Changed

- Worktree groups are now called sections, so they are not mistaken for herdr's worktree groups, a
  repository together with its worktrees. A worktree's right-click menu offers Section..., and
  `herdsman workspace section` replaces `herdsman workspace group`, which still works.

### Fixed

- A pane added to a stack fits its slot at once. Until the layout next changed, it could keep the
  size it started with, so its bottom rows hid behind the stack's strip.

### From herdr 0.9.4

#### Added

- `herdsman plugin update` updates plugins installed from GitHub, all at once or by name. Running
  plugin panes keep working on their old files, and new runs pick up the update without a config
  reload. Locally linked plugins are left alone.
  ([herdr#4245](https://github.com/herdrdev/herdr/issues/4245), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Windows notifications stay in Notification Center. Clicking one jumps to its pane and brings the
  terminal window forward. ([herdr#4773](https://github.com/herdrdev/herdr/issues/4773),
  [herdr#4803](https://github.com/herdrdev/herdr/issues/4803), thanks
  [@JJLiebig](https://github.com/JJLiebig))

#### Changed

- An elevated Windows server now only accepts administrator clients by default. To let normal
  clients from the same account in, set `[server] allow_unelevated_clients = true` or start the
  server with `--allow-unelevated-clients`. Permission errors now say how to fix them, and sessions
  you can't reach show as unavailable instead of stopped.
  ([herdr#4717](https://github.com/herdrdev/herdr/issues/4717),
  [herdr#4769](https://github.com/herdrdev/herdr/issues/4769),
  [herdr#4777](https://github.com/herdrdev/herdr/issues/4777),
  [herdr#4779](https://github.com/herdrdev/herdr/issues/4779), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- On Linux and macOS, closing the shell that started Herdsman no longer stops the server. Scripts
  that stopped Herdsman with `SIGHUP` should use `herdsman server stop` or `SIGTERM`. The server log
  now says why the server stopped. ([herdr#4920](https://github.com/herdrdev/herdr/issues/4920),
  [herdr#4965](https://github.com/herdrdev/herdr/issues/4965))
- With mouse capture on, plugins that handle a link get first pick when you click it. Turn capture
  off to use your terminal's own link shortcuts, like Command-click on macOS.
  ([herdr#4874](https://github.com/herdrdev/herdr/issues/4874),
  [herdr#5064](https://github.com/herdrdev/herdr/issues/5064), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Antigravity and Grok count as idle once they are back at their prompt, even if their own
  background tasks or subagents are still running.
  ([herdr#3530](https://github.com/herdrdev/herdr/issues/3530),
  [herdr#4735](https://github.com/herdrdev/herdr/issues/4735),
  [herdr#4959](https://github.com/herdrdev/herdr/issues/4959))
- Ctrl+Shift+letter reaches plain shells as the Ctrl key, like in Alacritty and WezTerm. Super,
  Hyper and Meta shortcuts no longer type escape-code junk into apps that can't read them.
  ([herdr#4356](https://github.com/herdrdev/herdr/issues/4356),
  [herdr#5017](https://github.com/herdrdev/herdr/issues/5017))

#### Fixed

- Restarting a Mac or shutting down Windows keeps your agents' sessions. Herdsman saves before the
  system starts closing apps, so agents come back where they were.
  ([herdr#4414](https://github.com/herdrdev/herdr/issues/4414),
  [herdr#5013](https://github.com/herdrdev/herdr/issues/5013),
  [herdr#5057](https://github.com/herdrdev/herdr/issues/5057),
  [herdr#5092](https://github.com/herdrdev/herdr/issues/5092), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Session saves are written to disk before they replace the old file, so a crash or power cut can't
  leave an unreadable save. ([herdr#5150](https://github.com/herdrdev/herdr/issues/5150),
  [herdr#5152](https://github.com/herdrdev/herdr/issues/5152), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- The server keeps running when the machine runs out of processes or threads, instead of taking
  every pane down with it. ([herdr#3324](https://github.com/herdrdev/herdr/issues/3324),
  [herdr#4958](https://github.com/herdrdev/herdr/issues/4958))
- An out-of-range workspace number returns `workspace_not_found` instead of crashing the server.
  ([herdr#5096](https://github.com/herdrdev/herdr/issues/5096),
  [herdr#5171](https://github.com/herdrdev/herdr/issues/5171))
- Suspending an agent with Ctrl+Z and bringing it back with `fg` keeps its name and saved
  conversation. ([herdr#1647](https://github.com/herdrdev/herdr/issues/1647),
  [herdr#4964](https://github.com/herdrdev/herdr/issues/4964))
- Restoring a session stays responsive when a checkout's Git data is slow or missing. Worktree
  groups survive checkouts that are temporarily unavailable, and opening the same repository twice
  no longer nests one workspace inside the other.
  ([herdr#4926](https://github.com/herdrdev/herdr/issues/4926),
  [herdr#4736](https://github.com/herdrdev/herdr/issues/4736),
  [herdr#4945](https://github.com/herdrdev/herdr/issues/4945),
  [herdr#4770](https://github.com/herdrdev/herdr/issues/4770),
  [herdr#4772](https://github.com/herdrdev/herdr/issues/4772), thanks
  [@minatoaquaMK2](https://github.com/minatoaquaMK2) and [@JJLiebig](https://github.com/JJLiebig))
- Closing a workspace's only pane asks first, like closing its last tab already does. Turn this off
  with `ui.confirm_close = false`. ([herdr#5143](https://github.com/herdrdev/herdr/issues/5143),
  [herdr#5173](https://github.com/herdrdev/herdr/issues/5173))
- Ctrl shortcuts like Ctrl+W work on non-Latin keyboard layouts such as Russian, Greek or Hebrew.
  Latin layouts and AltGr characters keep their normal meaning.
  ([herdr#1079](https://github.com/herdrdev/herdr/issues/1079),
  [herdr#4961](https://github.com/herdrdev/herdr/issues/4961))
- Accent keys and AltGr shortcuts work on Windows.
  ([herdr#4688](https://github.com/herdrdev/herdr/issues/4688),
  [herdr#5021](https://github.com/herdrdev/herdr/issues/5021), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Shift+Enter and other enhanced keys keep working after the outer terminal resets, for example
  after VS Code's Reload Window. ([herdr#4985](https://github.com/herdrdev/herdr/issues/4985),
  [herdr#5088](https://github.com/herdrdev/herdr/issues/5088))
- Quick Escape taps work on macOS with WezTerm's Kitty keyboard mode.
  ([herdr#1266](https://github.com/herdrdev/herdr/issues/1266),
  [herdr#5089](https://github.com/herdrdev/herdr/issues/5089))
- Kitty keyboard apps get key releases and F1, F2 and F4 correctly, and Backspace works when the
  terminal sends Ctrl+H for it. ([herdr#4184](https://github.com/herdrdev/herdr/issues/4184),
  [herdr#4403](https://github.com/herdrdev/herdr/issues/4403),
  [herdr#3244](https://github.com/herdrdev/herdr/issues/3244),
  [herdr#5017](https://github.com/herdrdev/herdr/issues/5017))
- Keyboard and terminal color replies no longer leak stray characters into pane input, and a
  half-finished color reply no longer freezes typing.
  ([herdr#4856](https://github.com/herdrdev/herdr/issues/4856),
  [herdr#4025](https://github.com/herdrdev/herdr/issues/4025),
  [herdr#5052](https://github.com/herdrdev/herdr/issues/5052),
  [herdr#5017](https://github.com/herdrdev/herdr/issues/5017),
  [herdr#5087](https://github.com/herdrdev/herdr/issues/5087))
- Fish no longer repeats prompt fragments while you resize a pane. The same problem with zsh and
  Powerlevel10k is still open, waiting on a Ghostty fix.
  ([herdr#3672](https://github.com/herdrdev/herdr/issues/3672),
  [herdr#4977](https://github.com/herdrdev/herdr/issues/4977))
- Apps that ask for pixel mouse coordinates get them, and clicks land in the right place on
  terminals that only report cells. ([herdr#4750](https://github.com/herdrdev/herdr/issues/4750),
  [herdr#3675](https://github.com/herdrdev/herdr/issues/3675),
  [herdr#5017](https://github.com/herdrdev/herdr/issues/5017),
  [herdr#4971](https://github.com/herdrdev/herdr/issues/4971))
- Turning off Kitty graphics makes panes stop advertising image support, so apps fall back to text.
  ([herdr#4978](https://github.com/herdrdev/herdr/issues/4978),
  [herdr#4979](https://github.com/herdrdev/herdr/issues/4979))
- Typing shows up faster, especially in PowerShell.
  ([herdr#4884](https://github.com/herdrdev/herdr/issues/4884),
  [herdr#5149](https://github.com/herdrdev/herdr/issues/5149), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Big sessions use less CPU on busy agents and idle shells, hidden panes use less memory, and
  Windows panes need fewer threads. ([herdr#4506](https://github.com/herdrdev/herdr/issues/4506),
  [herdr#5083](https://github.com/herdrdev/herdr/issues/5083),
  [herdr#4845](https://github.com/herdrdev/herdr/issues/4845),
  [herdr#4847](https://github.com/herdrdev/herdr/issues/4847),
  [herdr#4849](https://github.com/herdrdev/herdr/issues/4849),
  [herdr#4850](https://github.com/herdrdev/herdr/issues/4850),
  [herdr#4873](https://github.com/herdrdev/herdr/issues/4873),
  [herdr#5085](https://github.com/herdrdev/herdr/issues/5085),
  [herdr#5091](https://github.com/herdrdev/herdr/issues/5091), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- New workspaces, tabs and splits start their shell at the right size instead of borrowing another
  pane's size. ([herdr#4419](https://github.com/herdrdev/herdr/issues/4419),
  [herdr#4873](https://github.com/herdrdev/herdr/issues/4873))
- Forwarded SSH agents are no longer contacted every second, which could eventually break SSH and
  Git authentication. ([herdr#4931](https://github.com/herdrdev/herdr/issues/4931),
  [herdr#4962](https://github.com/herdrdev/herdr/issues/4962))
- Sidebar width, collapsed groups and agent sorting on remote machines survive reconnects. Switching
  workspaces quickly while changing machines no longer gets canceled.
  ([herdr#4364](https://github.com/herdrdev/herdr/issues/4364),
  [herdr#5019](https://github.com/herdrdev/herdr/issues/5019),
  [herdr#4536](https://github.com/herdrdev/herdr/issues/4536),
  [herdr#5061](https://github.com/herdrdev/herdr/issues/5061), thanks
  [@minatoaquaMK2](https://github.com/minatoaquaMK2) and [@JJLiebig](https://github.com/JJLiebig))
- The sidebar collapse button stays clickable when the agent list has a scrollbar.
  ([herdr#4692](https://github.com/herdrdev/herdr/issues/4692),
  [herdr#4734](https://github.com/herdrdev/herdr/issues/4734), thanks
  [@minatoaquaMK2](https://github.com/minatoaquaMK2))
- The welcome screen shows your configured prefix.
  ([herdr#4745](https://github.com/herdrdev/herdr/issues/4745),
  [herdr#4747](https://github.com/herdrdev/herdr/issues/4747), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Ctrl+[ leaves workspace navigation. ([herdr#1431](https://github.com/herdrdev/herdr/issues/1431),
  [herdr#2340](https://github.com/herdrdev/herdr/issues/2340), thanks
  [@qaz74107410](https://github.com/qaz74107410))
- Keybindings that use `+` survive config reloads, and `/` opens Go To search on layouts where it
  needs Shift. ([herdr#4960](https://github.com/herdrdev/herdr/issues/4960),
  [herdr#4963](https://github.com/herdrdev/herdr/issues/4963),
  [herdr#4983](https://github.com/herdrdev/herdr/issues/4983),
  [herdr#5090](https://github.com/herdrdev/herdr/issues/5090), thanks
  [@minatoaquaMK2](https://github.com/minatoaquaMK2))
- Pasting from WezTerm on Windows into remote panes no longer types encoded key data. Copying no
  longer adds duplicate clipboard history entries, and closing a pane no longer hangs on a pending
  Enter. ([herdr#4725](https://github.com/herdrdev/herdr/issues/4725),
  [herdr#4730](https://github.com/herdrdev/herdr/issues/4730),
  [herdr#4709](https://github.com/herdrdev/herdr/issues/4709),
  [herdr#4872](https://github.com/herdrdev/herdr/issues/4872), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Windows shells started through Scoop or 32-bit wrappers keep their working directory in new tabs
  and after restarts. ([herdr#4472](https://github.com/herdrdev/herdr/issues/4472),
  [herdr#4749](https://github.com/herdrdev/herdr/issues/4749),
  [herdr#5062](https://github.com/herdrdev/herdr/issues/5062), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Windows ARM64 machines can be used as remote hosts through the x64 build, on a best-effort basis.
  ([herdr#4752](https://github.com/herdrdev/herdr/issues/4752))
- Codex reports finished turns through its integration when started with `--no-daemon`, its startup
  update screen shows as blocked, and Codex and Pi status is recognized in more cases.
  ([herdr#4507](https://github.com/herdrdev/herdr/issues/4507),
  [herdr#4811](https://github.com/herdrdev/herdr/issues/4811),
  [herdr#4756](https://github.com/herdrdev/herdr/issues/4756),
  [herdr#5174](https://github.com/herdrdev/herdr/issues/5174), thanks
  [@JJLiebig](https://github.com/JJLiebig))
- Antigravity permission, question and workspace-trust dialogs show as blocked, and its thinking
  spinner shows as working. ([herdr#3871](https://github.com/herdrdev/herdr/issues/3871),
  [herdr#3419](https://github.com/herdrdev/herdr/issues/3419),
  [herdr#3530](https://github.com/herdrdev/herdr/issues/3530),
  [herdr#4959](https://github.com/herdrdev/herdr/issues/4959))
- Hermes started through its Python installer is recognized, and installing or removing its
  integration keeps your YAML plugin list intact.
  ([herdr#4910](https://github.com/herdrdev/herdr/issues/4910),
  [herdr#4909](https://github.com/herdrdev/herdr/issues/4909),
  [herdr#4911](https://github.com/herdrdev/herdr/issues/4911),
  [herdr#4984](https://github.com/herdrdev/herdr/issues/4984), thanks
  [@minatoaquaMK2](https://github.com/minatoaquaMK2))
- On macOS, `pane run` and `agent start` refuse commands too long for a shell that is still
  starting, instead of silently cutting them off.
  ([herdr#4950](https://github.com/herdrdev/herdr/issues/4950),
  [herdr#4988](https://github.com/herdrdev/herdr/issues/4988), thanks
  [@minatoaquaMK2](https://github.com/minatoaquaMK2))

#### Removed

- The experimental `HERDSMAN_WINDOWS_INPUT_BACKEND=crossterm` setting is gone. Windows always uses
  its native console input. ([herdr#5017](https://github.com/herdrdev/herdr/issues/5017))

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
