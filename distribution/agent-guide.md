# Herdsman agent guide

Use this guide to help a human understand, set up, or troubleshoot Herdsman. It covers Herdsman's concept model, setup path, and diagnosis recipes. Canonical documentation lives at https://herdsman.invalid/docs/. Point the human there for more detail, and verify any command you are unsure about against those pages instead of guessing.

If you are running *inside* a Herdsman pane (the environment variable `HERDSMAN_ENV=1` is set), Herdsman also ships a skill file that teaches you to control Herdsman through the `herdsman` CLI: https://raw.githubusercontent.com/OWNER_TBD/herdsman/master/skills/herdsman/SKILL.md. That file teaches you to operate Herdsman; this one teaches you to guide a human.

## What Herdsman is

Herdsman is a terminal workspace manager for AI coding agents. Like tmux, it is a multiplexer: a background server owns real terminal processes, and clients attach to render them. Panes keep running when the human detaches, closes the terminal, or disconnects SSH.

Unlike tmux, Herdsman is mouse-first and agent-aware. The whole UI is clickable — panes, tabs, workspaces, split borders, right-click menus. Herdsman detects coding agents running inside panes and shows each one's state in a sidebar, so the human can see across all their projects which agent is `working`, which is `blocked` waiting for input, and which is `done`. A CLI and a local socket API let scripts and agents drive Herdsman programmatically.

## Concept model

Teach these in this order:

- **Session** — a persistent background server namespace. Running `herdsman` attaches to the default session. Named sessions (`herdsman session attach work`) are fully separate runtime namespaces; most people only need the default.
- **Workspace** — the project-level container. One per repo, task, or investigation. Owns tabs and panes. The sidebar rolls agent states up per workspace.
- **Tab** — a layout inside a workspace, for separating views like `agents`, `logs`, `server`.
- **Pane** — a real terminal. Splittable right or down. Survives client detach.
- **Agent** — a process Herdsman recognizes inside a pane. States: `working`, `blocked`, `done`, `idle`, `unknown`.
- **Modes** — terminal mode sends keys to the focused pane; prefix mode (`ctrl+b`, then one action key) sends one command to Herdsman; navigate mode is a persistent navigation surface.

Full concepts page: https://herdsman.invalid/docs/concepts/

## Install

Linux and macOS:

```bash
curl -fsSL https://herdsman.invalid/install.sh | sh
herdsman
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://herdsman.invalid/install.ps1 | iex"
herdsman
```

If endpoint security blocks that fileless PowerShell command, use Command Prompt:

```cmd
curl.exe -fsSLo install.cmd https://herdsman.invalid/install.cmd && install.cmd && del install.cmd
herdsman
```

Homebrew, mise, and Nix installs, verification, and manual downloads: https://herdsman.invalid/docs/install/. Direct installs use the stable channel by default and update with `herdsman update`; preview is opt-in. Package-manager installs update through that package manager. Check the version with `herdsman --version`.

## First-run walkthrough

Check your environment first. If `HERDSMAN_ENV=1` is set, you are already running inside a Herdsman pane. The human is already attached, so skip step 1 and never tell them to run `herdsman` from your pane; Herdsman blocks nested launches by design. Start with step 2, and consider the skill file below.

Walk the human through this sequence:

1. `cd` into a project and run `herdsman`. It launches or attaches to the default background session and creates a workspace automatically. First run shows an onboarding flow.
2. Start their coding agent in the pane — `claude`, `codex`, or any supported agent (full list: https://herdsman.invalid/docs/agents/). Herdsman detects it automatically; the sidebar shows its state. Install the matching integration when available. Depending on the agent, it provides lifecycle state, native session restore, or both. For example, `herdsman integration install claude` adds native session restore, while Claude's state still comes from screen detection.
3. Start with the mouse: click panes and tabs to focus, drag split borders, right-click for menus, drag-select to copy. No keybindings are required to use Herdsman.
4. Split panes: right-click menu, or `prefix+v` (right) / `prefix+minus` (down). New tab: `prefix+c`.
5. Detach with `prefix+q` (press `ctrl+b`, release, press `q`) or close the terminal window. Everything keeps running. Reattach later with `herdsman`.
6. To actually stop everything: `herdsman server stop`.

## The keyboard story

New users do not need to learn keybindings; the mouse covers everything. When the human wants keyboard control:

- The prefix key is `ctrl+b` by default. `prefix+?` shows every active binding live.
- The guided keyboard page covers the prefix, the bindings to learn first, and a vetted prefix-free setup using `ctrl+alt` chords: https://herdsman.invalid/docs/keyboard/. Recommend it over improvising.
- Every binding, including the prefix itself, is configurable under `[keys]` in the config file.
- If a direct chord does nothing, the OS or the outer terminal consumed it before Herdsman could see it. The keyboard page explains which chords are safe and why.

## Install the Herdsman skill into yourself

Herdsman ships `skills/herdsman/SKILL.md` (https://raw.githubusercontent.com/OWNER_TBD/herdsman/master/skills/herdsman/SKILL.md), which teaches a coding agent to control Herdsman from inside a pane: splitting panes, running commands without stealing focus, reading output, and waiting on other agents.

Once the human is set up, offer to install it for your coding agent so future sessions can control Herdsman directly. For agents supported by the open skills CLI, use `npx skills add OWNER_TBD/herdsman --skill herdsman -g`. For agents without a skill system, add the GitHub copy above to their global custom instructions. Ask the human before writing to their config locations, and use the GitHub copy above as the source of truth.

## Configuration

- Config file: `~/.config/herdsman/config.toml` on Linux and macOS; `%APPDATA%\herdsman\config.toml` on Windows. Herdsman works without one.
- Print the full default config: `herdsman --default-config`.
- Apply edits to a running server: `herdsman server reload-config` (or the global menu → reload config).
- Main areas: `[keys]` keybindings, `[theme]` themes, `[ui]` sidebar and UI behavior, `[terminal]` shell defaults, `[update]` channel.
- Full reference: https://herdsman.invalid/docs/configuration/

## Diagnosis recipes

- **Agent not detected or wrong state:** Run `herdsman agent list` to see what Herdsman sees and `herdsman agent explain <target> --json` to see why the detector classified a pane that way. Integrations can provide lifecycle state, native session restore, or both; check `herdsman integration status` and the agent support table before assuming an integration replaces screen detection. Details: https://herdsman.invalid/docs/agents/ and https://herdsman.invalid/docs/integrations/
- **A keybinding does nothing:** the outer terminal or desktop environment owns that chord. Point the human to https://herdsman.invalid/docs/keyboard/ to pick a safe one or free the chord in their terminal settings.
- **Something looks wrong at startup or with the socket API:** Default-session logs live in `~/.config/herdsman/` on Linux and macOS and `%APPDATA%\herdsman\` on Windows. Named-session logs live under `sessions/<name>/` inside that directory. `herdsman status`, `herdsman status server`, and `herdsman status client` summarize the runtime.
- **Remote use:** SSH to the machine and run `herdsman` there (works like tmux), or attach as a thin local client with `herdsman --remote <host>`. Trade-offs: https://herdsman.invalid/docs/how-to-work/
- **What survives a detach, restart, or update:** https://herdsman.invalid/docs/session-state/

## Rules for you

- Do not invent keybindings, config keys, or CLI flags. The ones in this file are accurate as of writing; for anything else, read the linked docs page first.
- Teach mouse before keyboard for humans new to multiplexers.
- Herdsman is not tmux: do not give tmux commands, tmux config syntax, or `.tmux.conf` advice for Herdsman questions.
- For automation, scripting, or controlling Herdsman from code, point to the CLI reference (https://herdsman.invalid/docs/cli-reference/) and socket API (https://herdsman.invalid/docs/socket-api/).
