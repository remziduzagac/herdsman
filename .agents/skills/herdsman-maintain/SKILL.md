---
name: herdsman-maintain
description: "Maintaining herdsman, a fork that keeps merging herdr's releases. Use when adding or changing herdsman behaviour, above all in files that come from herdr, when moving herdsman code out of herdr files, or when taking a herdr release into herdsman."
---

# Maintaining herdsman

herdsman is a fork of herdr that keeps taking in herdr's releases with `git merge`. Two kinds of work
follow: writing herdsman's code so that those merges stay cheap, and doing the merges.
`MAINTAINING.md` is the reference for people and holds every command; this skill is the agent's way
through it.

## Whose file is it?

A file is herdr's when it exists on the `herdr-import` branch, renamed:

```bash
git cat-file -e herdr-import:src/layout.rs && echo herdr || echo herdsman
```

herdr's files take only small hooks marked `// fork: <feature>` (`# fork:` in Python); herdsman's
files hold its logic and its tests. `rg '(//|#) fork:'` lists every hook, and `MAINTAINING.md`,
"herdsman's edits to herdr files", lists the herdr files herdsman changed beyond hooks.

## Branches

- `dev`: where work comes together. The main checkout stays on it.
- `herdr-import`: herdr's code, renamed, one commit per imported revision. Only `upstream/import`
  writes it.
- `test` and `main`: previews and releases, fast-forwarded from `dev` by the person.
- Feature and merge branches each get a worktree from `scripts/worktree <name>`, under
  `~/Projects/worktrees/herdsman/`.

## What are you doing?

- **Adding or changing herdsman behaviour,** or moving herdsman code out of a herdr file: read
  [FORK-CODE.md](FORK-CODE.md) and follow it.
- **Taking a herdr release, or a herdr revision, into herdsman:** read
  [MERGE-HERDR.md](MERGE-HERDR.md) and follow it.
