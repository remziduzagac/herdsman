# Maintaining herdsman

herdsman is built on [herdr](https://github.com/herdrdev/herdr) and keeps taking in its releases.
herdr's code arrives in two steps: `upstream/import` copies a herdr revision out of a plain clone
and renames it with `upstream/rename`, committing the result on the `herdr-import` branch; a normal
merge then brings it into `dev`. Each import sits on top of the previous one, so git always has a
merge base, and a merge conflicts only where both sides changed the same lines.

## Branches and folders

| Branch | Holds | Written by |
| --- | --- | --- |
| `dev` | herdsman: where work comes together | merges and small commits |
| `test`, `main` | herdsman, promoted from `dev` | fast-forward only |
| `herdr-import` | herdr's code, renamed, one commit per imported revision | `upstream/import` only |
| `<feature>` | one feature or fix, branched from `dev` | you; deleted once merged |
| `merge-v<version>` | one herdr release being merged | you; deleted once merged |

The main checkout stays on `dev`. Feature and merge branches each get a worktree from
`scripts/worktree <name>`, under `~/Projects/worktrees/herdsman/` by default (`WORKTREES_DIR`
overrides it). The helper copies the main checkout's git-excluded `mise.local.toml` and points cargo
at the main checkout's `target/`, so dependencies build once.

herdr itself is a plain clone, `~/Projects/herdr` by default (`UPSTREAM_DIR` overrides it). Make its
`origin` fetch-only with `git remote set-url --push origin no-push`.

The commit-msg hook from `just install-hooks` checks every commit subject, merges included, and CI
checks pull request titles. Give merges a conventional subject:
`git merge -m "feat: <what the branch adds>" <branch>`.

On GitHub, `main` is the default branch, so visitors see the released state, and pull requests
target `dev`. CI runs on pushes to `main` and `dev` and on every pull request.

## Versions

herdsman versions are its own and plain `X.Y.Z`; the updater parses nothing else. They started at
`1.0.0` so that they stay above herdr's 0.x numbers, which plugin minimum versions, docs and tests
still use.

- Merging a herdr release or adding a feature bumps the minor version.
- A fix bumps the patch version.
- Each `CHANGELOG.md` entry names the herdr version it is built on.

`upstream/rename` sets the package version in every import to `0.0.0`, so herdr's version bumps never
conflict with herdsman's. herdr's real version is in each import commit's `Version:` line.

When herdr reaches 1.x, its minimums can overtake herdsman's numbers. The plugin tests fail at that
merge; bump herdsman's major version then.

## Taking a herdr release

Commands without a `cd` run in the main checkout, on `dev`.

### 1. See what changed

```bash
git log -1 --format=%b herdr-import          # Upstream: the last imported commit
git -C ~/Projects/herdr fetch --tags origin
git -C ~/Projects/herdr log --oneline <last>..v0.9.4
git -C ~/Projects/herdr diff --stat <last>..v0.9.4 -- src/layout.rs src/persist \
  src/ui/panes.rs src/app/api src/client/shell src/config src/input src/cli src/api
```

Upstream commits touching `src/layout.rs`, `src/persist/` or `src/ui/panes.rs` deserve a read first:
those carry the core of stacks. Elsewhere herdsman mostly adds lines to lists.

The import drops some herdr files (`DROP` in `upstream/rename`). Anything worth taking from them,
such as a CI improvement in a dropped workflow, is a hand edit:
`git -C ~/Projects/herdr diff <last>..v0.9.4 -- .github CHANGELOG.md`.

### 2. Import

```bash
upstream/import v0.9.4
```

It changes no files in the checkout, only the `herdr-import` branch, and does nothing if that
revision is already there.

It stops if the name `herdr` survives the rename anywhere, and prints where: a new casing, say, or a
new URL form. It also warns when a `DROP` entry no longer exists, which means herdr moved a file.
Teach `upstream/rename` the case, commit that on `dev`, and import again.

### 3. Merge

```bash
scripts/worktree merge-v0.9.4
cd ~/Projects/worktrees/herdsman/merge-v0.9.4
git merge -m "chore: merge herdr 0.9.4" herdr-import
```

Steps 3 to 5 run in that worktree. `git merge --abort` returns to the start. During a merge
`--ours` is herdsman and `--theirs` is the import. Resolve conflicts by kind:

- **Generated schema**, `docs/next/api/herdsman-api.schema.json`: take either side and regenerate it
  in step 4. Never merge it by hand.
- **Config reference**, `docs/next/website/src/data/config-reference.json`: take theirs, then re-add
  the six stack entries: `keys.focus_stacked`, `keys.next_stacked`, `keys.previous_stacked`,
  `keys.stack_pane`, `keys.close_stacked` and `ui.stack_strip_position`. `just maintenance-test`
  fails until they are back.
- **Lists both sides extend**, such as the `Method` enum, method names, `CLIENT_SHELL_METHODS`,
  keybinding tables and defaults: keep both. `CLIENT_SHELL_METHODS` must stay sorted.
- **herdr edits a line stacks changed**: keep herdr's change and re-apply the stack logic.
- **herdsman's own edits to herdr files**, listed below: keep herdsman's side.

Then check that no herdr name came back in through a resolution:

```bash
upstream/rename check .
```

### 4. Rebuild the generated parts

```bash
ZIG=true cargo check --locked --all-targets   # fast; the compiler lists every new match site
HERDSMAN_UPDATE_API_SCHEMA=1 just test-one generated_protocol_schema_artifact_is_current
```

A new upstream `match` on `Node`, `LayoutSnapshot`, `LayoutNode`, `PaneMoveDestination` or
`PendingEndpointKind` shows up as a compile error; add the stack arm as the neighbouring code does.

### 5. Check

```bash
just lint
just test
```

Failures that come from the rename rather than from real bugs:

- **A pinned hash or width** of text that contained the old name: a new herdr test, or one of the
  re-pinned values below. The assertion prints the new value; pin it.
- **`advertised_client_shell_method_shapes_stay_at_the_v1_contract`**: if it fails on a stack
  method, a shared parameter type changed; update the four pinned `pane.*` digests in
  `src/server/client_commands.rs`. If it fails on a herdr method whose enum carries the name,
  re-pin it in the fixture. Other shape changes are herdr's own.

### 6. Version, changelog, promote

Bump the minor version in `Cargo.toml` (`cargo update -p herdsman --offline` follows in
`Cargo.lock`) and add a `CHANGELOG.md` entry naming the herdr version. Then:

```bash
git commit                                   # in the worktree
cd ~/Projects/herdsman
git merge merge-v0.9.4
git worktree remove ~/Projects/worktrees/herdsman/merge-v0.9.4 && git branch -d merge-v0.9.4
git fetch . dev:test                         # when it has been used for a while
git fetch . test:main
```

`git fetch . dev:test` moves `test` to `dev` without leaving `dev`, and refuses anything but a
fast-forward. If it refuses because a fix landed on `test` or `main` directly, check that branch out
and merge instead.

## What the rename does

`upstream/rename` explains itself in its header. In short, it:

- drops herdr's own project files: its README, contribution policy, sponsors, logo, maintainer lists,
  issue templates, herdr-only workflows, changelogs, release manifests, versioned and translated
  docs, and release tooling;
- turns links to herdr's tracker into plain "base project issue #N" references, because herdsman's
  tracker has no such issues. "Upstream" is avoided there, because the vendored libraries' notes
  already use it for their own projects;
- renames every `HERDR`, `Herdr` and `herdr`, in contents and in paths, and points `herdr.dev` and
  `herdrdev` at herdsman's placeholders;
- sets the package version to `0.0.0` and runs `cargo fmt`.

## herdsman's edits to herdr files

When a merge conflicts in one of these, keep herdsman's side.

| File | herdsman's change |
| --- | --- |
| `AGENTS.md` | herdr's governance and release-process sections removed; Scope and Docs rewritten |
| `build.rs` | herdr's contributor-policy warning removed |
| `justfile` | herdr's release recipes and their tests removed |
| `distribution/latest.json` | herdsman's own manifest, written by herdsman's releases |
| `docs/next/product-announcement.json` | herdsman's own; `null` until a release announces something |
| `src/update.rs` | fake-update notes read herdsman's `1.0.0` changelog section |
| `src/checksum.rs` | the test's SHA-256 re-pinned |
| `src/ui/text.rs` | the truncation test's width widened for the longer name |
| `scripts/agent_detection_manifest_check.py` | `STAGED_PUBLISHED_MANIFESTS` digest re-pinned |
| `tests/fixtures/endpoint-method-shapes-v1.json` | `pane.split` and `pane.input.set` re-pinned |
| `skills/herdsman/SKILL.md` | a stacks section |

### Stacks

Stacks change 37 herdr files, almost all additively. The new files are
`src/ui/stack_strip.rs`, `src/client/shell/stack_marks.rs` and `src/client/shell/tests/stacks.rs`.

| Area | Files |
| --- | --- |
| Layout core | `layout.rs` |
| Persistence | `persist/snapshot.rs`, `persist/restore.rs` |
| Layout API | `api/schema/panes.rs`, `app/api/layouts.rs` |
| Methods | `api/schema.rs`, `api/schema/response.rs`, `api/mod.rs`, `api/server.rs` |
| Handlers | `app/api.rs`, `app/api/panes.rs` |
| Advertising | `server/client_commands.rs` |
| Rendering | `ui.rs`, `ui/panes.rs` |
| Done and idle | `app/actions.rs`, `workspace.rs` |
| Config | `config/model.rs`, `config/keybinds.rs`, `app/state.rs`, `app/mod.rs` |
| Input | `input/keybindings.rs`, `input/keybind_help.rs`, `client/shell/actions.rs` |
| Strip clicks | `client/shell/mouse.rs` |
| Navigator | `client/shell.rs`, `client/shell/state.rs`, `client/shell/overlay_input.rs` |
| Navigator rows | `client/shell/aggregate_navigation.rs`, `client/shell/worktrees.rs` |
| CLI | `cli/pane.rs`, `cli/runtime.rs`, `cli/spec.rs` |
| Test wiring | `client/shell/tests/mod.rs`, `…/tests/graphics.rs`, `…/tests/popup_focus_projection.rs` |
| Generated | `docs/next/api/herdsman-api.schema.json` |
| Docs data | `docs/next/website/src/data/config-reference.json` |

The lines stacks changed rather than added are few: the `split_slot` calls in `layout.rs`, the
visibility checks in `app/actions.rs` and `workspace.rs`, the strip calls in `ui/panes.rs`, the click
method in `client/shell/mouse.rs`, and imports.

Stacks add four JSON API methods (`pane.stack`, `pane.focus_stacked`, `pane.focus_stacked_at`,
`pane.stacks`) and a `stack` destination for `pane.move`, advertised to client shells with frozen
shapes. No bincode struct, frozen codec or `PROTOCOL_VERSION` changed: `ClientShellSnapshot` also
travels as positional bincode, so stack data is fetched through the new methods instead.

## Releasing

Not set up yet. A release needs herdsman's GitHub home, a release workflow that builds the five
binaries (Linux and macOS on x86_64 and aarch64, and Windows), and GitHub Pages serving
`distribution/latest.json`, `install.sh` and the agent-detection catalog.

## Where herdsman lives

`upstream/rename` sets the repository, `remziduzagac/herdsman`, and the site,
`remziduzagac.github.io/herdsman` on GitHub Pages, for everything taken from herdr. The site serves
`latest.json`, the installers and the agent-detection catalog. Until a release publishes them,
`herdsman update` and remote installs find nothing to fetch.

To move herdsman elsewhere:

1. Set `SITE` and `OWNER` in `upstream/rename` and commit on `dev`.
2. Re-import the revision `herdr-import` already holds. The new commit changes only those values;
   merge it as in step 3.
3. Update herdsman's own files, which the rename never touches: `git grep -n remziduzagac`.
