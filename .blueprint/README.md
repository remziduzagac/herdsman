---
synced_with: dev@e2622d52
---

# herdsman blueprint

herdsman's design: what it must do, how it works, the decisions behind it, and the work in progress. herdsman is a fork of herdr that takes in herdr's releases and adds its own features; `MAINTAINING.md` and `AGENTS.md` say how. The user documentation lives in `docs/next/website/` and is written for users, not copied from here.

Status on 2026-11-01: not started. The initial design comes first.

## Where to start

1. [roadmap.md](roadmap.md): where the work stands.

## The strands

- [roadmap.md](roadmap.md): the order of work.

Requirements, design, ideas, notes, research, decisions, tasks and workstreams get their files when they first have content.

## How things are recorded

- Requirements and design are the current truth; decisions say why.
- Until the initial design is declared settled, changes go straight into these files. After that, every change to requirements or design goes through a workstream, which writes only inside its own folder and lists its changes to the root (ADDED, MODIFIED, REMOVED by path and section) until it is consolidated.
- Decisions are append-only: a change is a new entry, with a dated note under each earlier entry it changes.
- A change to the code that alters what this blueprint describes updates it in the same piece of work. `synced_with` above names the last commit on `dev` the blueprint was reconciled with.
- This folder is public, like the code: it never names anyone's machines, private repositories or personal details.

## Conventions

- **Markdown:** prose not wrapped; table rows, code lines and headings at most 120 characters.
- **Issues:** GitHub issues are linked in full, `[#30](https://github.com/remziduzagac/herdsman/issues/30)`.
