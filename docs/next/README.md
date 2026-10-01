# herdsman


<p align="center">
  <img src="assets/logo.png" alt="herdsman" width="100" />
</p>

<p align="center">
  <a href="https://herdsman.invalid">herdsman.invalid</a> · <a href="#install">install</a> · <a href="https://herdsman.invalid/docs/quick-start/">quick start</a> · <a href="https://herdsman.invalid/docs/">docs</a>
</p>

<p align="center">
  English · <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-666666?labelColor=333333" alt="Apache 2.0 license" /></a>
  <a href="https://github.com/OWNER_TBD/herdsman/releases"><img src="https://img.shields.io/github/downloads/OWNER_TBD/herdsman/total?labelColor=333333&color=666666" alt="total GitHub release downloads" /></a>
  <a href="https://github.com/OWNER_TBD/herdsman/stargazers"><img src="https://img.shields.io/github/stars/OWNER_TBD/herdsman?labelColor=333333&color=666666&logo=github" alt="GitHub stars" /></a>
  <a href="https://github.com/OWNER_TBD/herdsman/releases/latest"><img src="https://img.shields.io/github/v/release/OWNER_TBD/herdsman?label=release&labelColor=333333&color=666666" alt="latest stable release" /></a>
  <a href="https://formulae.brew.sh/formula/herdsman"><img src="https://img.shields.io/homebrew/v/herdsman?label=homebrew&labelColor=333333&color=666666" alt="Homebrew version" /></a>
  <a href="https://x.com/OWNER_TBD"><img src="https://img.shields.io/badge/follow-%40OWNER_TBD-000000?logo=x&logoColor=white" alt="follow @OWNER_TBD on X" /></a>
</p>

---

https://github.com/user-attachments/assets/043ec09f-4bdd-41d5-aee0-8fda6b83e267

**the runtime your coding agents live on.**

- **detach without stopping work** — herdsman keeps terminals running in a background server when you close the client or lose your SSH connection. after a server or machine restart, herdsman restores the saved layout and can resume supported agent sessions; the original processes do not survive. [session state →](https://herdsman.invalid/docs/session-state/)
- **several machines, one window** — keep local work and saved ssh machines together, with a combined agent list and independent reconnects. [remote machines →](https://herdsman.invalid/docs/connecting-machines/)
- **never hunt for the stuck one** — every pane is marked working, blocked, or idle. when an agent stops and needs an answer, herdsman says so.
- **agent-native** — agents drive herdsman through the cli and socket api: they can spawn panes, prompt each other, and wait until another agent is genuinely blocked. [agent skill →](https://herdsman.invalid/docs/agent-skill/)
- **runs what you already run** — claude code, codex, cursor, opencode, grok and the rest. herdsman doesn't wrap or replace them; it owns their terminals. building an agent? [add herdsman support →](https://herdsman.invalid/docs/add-herdsman-support/)
- **keyboard and mouse, both first-class** — tmux-style prefix keys *and* click, drag, split. pick per moment, not per tool.
- **plugins** — extend panes and workflows. [browse the marketplace →](https://herdsman.invalid/plugins/)
- **one rust binary, no electron** — runs in whatever terminal you already use.

---

## install

```bash
curl -fsSL https://herdsman.invalid/install.sh | sh
```

or `brew install herdsman` · `mise use -g herdsman` · windows: `powershell -ExecutionPolicy Bypass -c "irm https://herdsman.invalid/install.ps1 | iex"` · [endpoint-protected Windows](https://herdsman.invalid/docs/windows-beta/) · [binaries](https://github.com/OWNER_TBD/herdsman/releases)

then start it where the work lives:

```bash
herdsman
```

run your agents, split panes, walk away. `ctrl+b q` detaches, `herdsman` reattaches. [quick start →](https://herdsman.invalid/docs/quick-start/)

## docs

everything lives at [herdsman.invalid/docs](https://herdsman.invalid/docs/): [quick start](https://herdsman.invalid/docs/quick-start/) · [concepts](https://herdsman.invalid/docs/concepts/) · [supported agents](https://herdsman.invalid/docs/agents/) · [keyboard](https://herdsman.invalid/docs/keyboard/) · [configuration](https://herdsman.invalid/docs/configuration/) · [session state](https://herdsman.invalid/docs/session-state/) · [connecting machines](https://herdsman.invalid/docs/connecting-machines/) · [remote](https://herdsman.invalid/docs/persistence-remote/) · [integrations](https://herdsman.invalid/docs/integrations/) · [add herdsman support to your agent](https://herdsman.invalid/docs/add-herdsman-support/) · [plugins](https://herdsman.invalid/docs/plugins/) · [socket api](https://herdsman.invalid/docs/socket-api/)

## thanks

every past sponsor and backer is listed in [SPONSORS.md](./SPONSORS.md) — thank you 🐑

enterprise / partnership: hey@herdsman.invalid

## agent instructions

if you are an ai agent helping with this repository, read [`AGENTS.md`](./AGENTS.md) before making changes and read [`CONTRIBUTING.md`](./CONTRIBUTING.md) before opening issues or PRs.

## development

```bash
git clone https://github.com/OWNER_TBD/herdsman
cd herdsman
cargo build --release

just test        # unit tests
just check       # formatting, tests, and maintenance checks
```

## license

Herdsman is licensed under the [Apache License 2.0](LICENSE).
