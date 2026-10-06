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
| `test` | herdsman to try out; every push publishes a preview build | fast-forward from `dev` |
| `main` | herdsman as released | fast-forward from `test` |
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
- The major version stays one above herdr's: herdr 0.x is herdsman 1.x, and the merge that brings
  in herdr 1.0 makes herdsman 2.0.0. Otherwise both projects would soon share 1.x numbers.
- Each `CHANGELOG.md` entry names the herdr version it is built on.

`just release` refuses a version whose major is not one above that of the herdr version named by
the newest "Based on herdr" line in `CHANGELOG.md`, and `upstream/changelog` says when a merge
crosses a herdr major version.

`upstream/rename` sets the package version in every import to `0.0.0`, so herdr's version bumps never
conflict with herdsman's. herdr's real version is in each import commit's `Version:` line.

Plugin minimum versions, docs and tests that name herdr's numbers keep working across a major
merge, because herdsman's major stays ahead.

## Licence

herdr is licensed under Apache 2.0, which lets herdsman modify and redistribute it, and herdsman
keeps that licence. Licences can change between releases: herdr moved from AGPL-3.0-or-later to
Apache 2.0 on 2026-07-22. herdsman may take a revision only under terms it can keep.

`upstream/import` compares every licence file in herdr's tree, vendored code included, and the
`license` field of every crate with the previous import, and stops when any differ. Read the
change, then either import again with `--accept-license-change` or stay on the previous herdr
release. A changed licence for vendored code also means updating `NOTICE` and the About page
(`docs/next/website/src/content/docs/about.mdx`).

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

It stops when herdr's licensing changed since the previous import ([Licence](#licence)).

It also stops if the name `herdr` survives the rename anywhere, and prints where: a new casing, say,
or a new URL form, and warns when a `DROP` entry no longer exists, which means herdr moved a file.
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
  the seven stack entries: `keys.focus_stacked`, `keys.next_stacked`, `keys.previous_stacked`,
  `keys.stack_pane`, `keys.close_stacked`, `ui.stack_strip_position` and `ui.stack_strip_separator`.
  `just maintenance-test` fails until they are back.
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
- **`advertised_client_shell_method_shapes_stay_at_the_v1_contract`**: if it fails on one of
  herdsman's methods, a shared parameter type changed; update its pinned digest in
  `src/server/client_commands/fork_methods.rs`. If it fails on a herdr method whose enum carries the
  name, re-pin it in the fixture. Other shape changes are herdr's own.

### 6. Changelog, promote

herdsman skips herdr versions, so its notes carry herdr's for every version since the last merge:

```bash
upstream/changelog 0.9.4 --write
```

This adds a "Based on herdr 0.9.4" line to the `## Unreleased` section of `CHANGELOG.md`, and
herdr's notes for each version after the one the previous "Based on herdr" line names, each under a
"From herdr" heading. The notes are renamed like the code; issue numbers and credits link to herdr's
GitHub, and docs links to herdsman's page of the same name. It prints every herdr.dev link it changed
or kept. Read the result: herdr's notes describe herdr, so edit what herdsman does differently. Drop
`--write` to print the notes instead.

The version is set when releasing, by `just release`. Then:

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

herdsman's features keep their logic in herdsman's own files and touch herdr files only through small
hooks marked `// fork: <feature>` (`# fork:` in Python); the `fork-code` skill in `.agents/skills/`
describes how. `rg '(//|#) fork:'` lists every hook, and `git diff --diff-filter=A --name-only
herdr-import -- src` every herdsman file. herdsman's own state on `AppState` lives in its one field,
`fork` (`src/app/state/fork.rs`). When a merge conflicts in one of these files, keep herdsman's side.

| File | herdsman's change |
| --- | --- |
| `AGENTS.md` | herdr's governance and release-process sections removed; Scope and Docs rewritten |
| `build.rs` | herdr's contributor-policy warning removed |
| `justfile` | herdr's release recipes and their tests removed; herdsman's `release` recipe added |
| `distribution/latest.json` | herdsman's own manifest, written by herdsman's releases |
| `docs/next/website/src/content/docs/install.mdx` | herdsman's install channels and preview channel |
| `docs/next/website/src/content/docs/{persistence-remote,session-state}.mdx` | system packages named with the other package managers |
| `docs/next/website/src/content/docs/plugins.mdx` | example repository not published yet; "Sharing plugins" replaces the marketplace |
| `docs/next/website/src/content/docs/index.mdx` | a Stacked panes card replaces the marketplace card |
| `docs/next/product-announcement.json` | herdsman's own; `null` until a release announces something |
| `src/update.rs` | fake-update notes read herdsman's `1.0.0` changelog section; `/usr/bin` installs count as system packages |
| `src/checksum.rs` | the test's SHA-256 re-pinned |
| `src/api/server.rs` | test socket prefix shortened to `hs-`; the longer name overflowed macOS's 104-byte socket path limit |
| `src/ui/text.rs` | the truncation test's width widened for the longer name |
| `scripts/agent_detection_manifest_check.py` | `STAGED_PUBLISHED_MANIFESTS` digest re-pinned |
| `tests/fixtures/endpoint-method-shapes-v1.json` | `pane.split` and `pane.input.set` re-pinned |
| `skills/herdsman/SKILL.md` | a stacks section |

### Stacks

Stacks live in herdsman's own files: `ui/stack_strip.rs`, `client/shell/stack_marks.rs`,
`client/shell/tests/stacks.rs`, and a `stack.rs` child module beside each herdr module they extend,
such as `layout/stack.rs` and `app/api/panes/stack.rs`. The herdr files below carry only hooks marked
`// fork: stacks`.

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

The lines stacks change rather than add are few: the `split_slot` calls in `layout.rs`, the
visibility checks in `app/actions.rs` and `workspace.rs`, and the press method in
`client/shell/mouse.rs`.

Stacks add five JSON API methods (`pane.stack`, `pane.focus_stacked`, `pane.focus_stacked_at`,
`pane.focus_stack_strip`, `pane.stacks`) and a `stack` destination for `pane.move`, advertised to
client shells with frozen shapes. No bincode struct, frozen codec or `PROTOCOL_VERSION` changed:
`ClientShellSnapshot` also travels as positional bincode, so stack data is fetched through the new
methods instead.

### Worktree groups

Named groups of a repository's linked worktrees live in herdsman's own files:

- `app/state/fork/worktree_groups.rs`: the groups, by checkout path, saved as
  `worktree-groups.json` in the session's data directory rather than in herdr's session format;
- `app/api/workspaces/worktree_groups.rs` and `api/schema/workspaces/worktree_groups.rs`: the
  `workspace.set_worktree_group` method;
- `cli/workspace/worktree_groups.rs` and `cli/spec/worktree_groups.rs`: `herdsman workspace group`;
- `client/shell/sidebar/worktree_groups.rs`: ordering, headers, clicks and the name prompt, with its
  tests in `client/shell/tests/worktree_groups.rs`; `client/shell/fork.rs` holds herdsman's click
  targets.

The herdr files below carry hooks marked `// fork: worktree groups`.

| Area | Files |
| --- | --- |
| Method | `api/schema.rs`, `api/schema/workspaces.rs`, `api/mod.rs`, `api/server.rs`, `app/api.rs` |
| Handler and advertising | `app/api/workspaces.rs`, `server/client_commands.rs` |
| Group token and saving | `app/creation.rs`, `app/mod.rs` |
| CLI | `cli/workspace.rs`, `cli/spec.rs` |
| Sidebar | `client/shell/sidebar.rs`, `client/shell/endpoint_sidebar.rs`, `client/shell/mouse.rs` |
| Hit map | `client/shell.rs`, `client/shell/state.rs` |
| Menu and prompt | `client/shell/state.rs`, `client/shell/context_menu.rs`, `client/shell/overlay_input.rs` |
| Test wiring | `client/shell/tests/mod.rs`, `client/shell/tests/endpoints.rs` |
| Docs | `configuration.mdx`, `cli-reference.mdx`, `socket-api.mdx` |

A group reaches clients as its members' `worktree_group` workspace token, so no snapshot field, wire
format or `PROTOCOL_VERSION` changed. `workspace_entries` still returns one entry per workspace,
ordered by group and without the hidden members of a collapsed group; the header rows exist only in
the two expanded sidebar renderers, drawn as extra lines of a neighbouring entry.

## Releasing

A release is a `vX.Y.Z` tag on `main`. Pushing it starts `.github/workflows/release.yml`, which
builds the five binaries (Linux and macOS on x86_64 and aarch64, and the Windows zip), creates the
GitHub release with the version's `CHANGELOG.md` section as its notes and `LICENSE` and `NOTICE`
beside the binaries, commits the new
`distribution/latest.json` to `main` and `dev`, redeploys GitHub Pages through `pages.yml`, and
updates the Homebrew formula.

1. **Write the notes.** Under `## Unreleased` in `CHANGELOG.md`, or under `## [X.Y.Z] - Unreleased`
   once the version is known. Name the herdr version the release is built on; `upstream/changelog`
   does that when a herdr release is merged.
2. **Check CI** passed on the `dev` commit you will release.
3. **Rehearse, if anything in the build changed.** This builds everything into a private draft
   release, to try the binaries before anyone else can see them:

   ```bash
   gh workflow run release.yml --ref dev -f version=X.Y.Z
   gh release list                       # the draft is vX.Y.Z-rehearsal.<run>
   gh release delete vX.Y.Z-rehearsal.<run> --yes
   ```

   `X.Y.Z` here is the version `Cargo.toml` has now, not the one being released: `just release`
   sets the new version, and the rehearsal checks the two agree. It rehearses the builds and the
   draft release; Linux packages, `latest.json` and the Homebrew formula only run for a real
   release, and `gh workflow run linux-packages.yml -f version=X.Y.Z` tries the packages for an
   existing release.

4. **Release:**

   ```bash
   just release X.Y.Z
   ```

   `scripts/release` dates the notes, sets the version in `Cargo.toml` and `Cargo.lock`, commits
   `release: vX.Y.Z`, tags it, fast-forwards `test` and `main`, and pushes all of them at once.
   Pushing `test` also publishes the release commit as a preview, so the preview channel never
   falls behind stable.
5. **Watch the workflow**, then `git pull` on `dev` to pick up the manifest commit it made.
6. **Check the packages.** The release workflow's last job waits for the release's Pages deploy,
   then starts `packages.yml`, which installs the release through the install script, the Homebrew
   tap, mise, Nix and the apt, dnf and pacman repositories on clean machines. A Pages deploy in the
   middle of a release, such as the one the release's push to `main` starts, skips its run. Run it
   by hand with `gh workflow run packages.yml`.

Pages serves `latest.json`, the install scripts, the agent guide and the agent-detection catalog
from `main`'s `distribution/` folder. A push to `main` that changes that folder redeploys it too.

### Linux packages and repositories

`linux-packages.yml` turns each release's two Linux binaries into `.deb`, `.rpm` and Arch packages
(`packaging/linux/nfpm.yaml`), signs the `.rpm` and Arch ones, and attaches them to the release. The
Release workflow calls it; run it by hand to package an earlier release:
`gh workflow run linux-packages.yml -f version=X.Y.Z`, then `gh workflow run pages.yml`.

Every Pages deploy rebuilds the apt, dnf and pacman repositories from the packages of the last five
releases (`scripts/package-repos`) and signs them. The signing key is the `PACKAGES_SIGNING_KEY`
secret, an RSA key with fingerprint `971C0D3C67E66F3C70BCCBF97E19E7816ABF24EF`; its public half is
`distribution/herdsman.asc`. The maintainer keeps the private key and its revocation certificate
offline. To replace the key, generate a new one, update the secret and `distribution/herdsman.asc`,
and update the fingerprint in the install guide, `packages.yml` and here. Users must then import the
new key, so announce it in the changelog.

### Homebrew

The Homebrew formula lives in [remziduzagac/homebrew-tap](https://github.com/remziduzagac/homebrew-tap).
The release workflow regenerates it with `scripts/homebrew-formula` and pushes it with a deploy key
that can write to that repository only, stored here as the `HOMEBREW_TAP_DEPLOY_KEY` secret.

## Previews

Pushing `test` starts `.github/workflows/preview.yml`. It builds the five binaries stamped as preview
builds (`herdsman --version` prints `X.Y.Z-preview.<build id>`, the build id being the commit date
and short hash), publishes them as the GitHub pre-release `preview-<build id>` with a
`build-info.json`, deletes all but the newest ten previews, and redeploys Pages, where
`scripts/preview-manifest` turns them into `preview.json`.

```bash
git fetch . dev:test && git push origin test       # publish what dev has as a preview
```

Machines installed with the install script follow previews after `herdsman channel set preview`;
`herdsman update` installs the newest, and attaching to a remote machine installs the same build
there. Pre-releases never reach the package repositories, the Homebrew tap or `latest.json`.

## The documentation site

`docs/next/website` is an Astro Starlight project that builds the pages in
`docs/next/website/src/content/docs`. The Pages workflow publishes it at
https://remziduzagac.github.io/herdsman/docs/ on every deploy, and a push to `main` that changes
`docs/next/` triggers one.

```bash
cd docs/next/website
npm ci
npm run dev      # live preview at http://localhost:4321/herdsman/docs/
npm run build    # the site in dist/; fails on any broken internal link
```

The pages link to each other as `/docs/<page>/`, as herdr's site did; `scripts/rebase-links.mjs`
adds the `/herdsman` prefix after the build, and `scripts/check-links.mjs` then fails the build on a
missing page, heading or a path outside the site. A new page needs an entry in the sidebar in
`astro.config.mjs`. Everything in `docs/next/website` except `src/content/docs` and `src/data` is
herdsman's own; herdr keeps its website in a separate, private repository.

## Repository settings

- **Rulesets.** "Long-lived branches" stops `main`, `dev` and `test` from being deleted or
  force-pushed, for everyone; the release flow only fast-forwards them. "Release tags" lets only
  repository admins create, move or delete `v*` tags, so nothing else can start a release. To get
  past either in an emergency, disable it under Settings → Rules, then turn it back on.
- **Dependabot** opens one grouped pull request a month each for Cargo, GitHub Actions and the docs
  site's npm packages, into `dev`; review and merge them like any other. The Cargo and Actions
  ones touch files herdr also updates: when a merge conflicts in `Cargo.lock` or in a herdr
  workflow, take herdr's side and run `cargo check` to settle `Cargo.lock`. Dependabot alerts
  are on; their security fixes arrive through these monthly updates or the next herdr merge.
- **Security reports** arrive through private vulnerability reporting, as `SECURITY.md` describes.
  Fix them in a private advisory fork or on a branch, ship the fix, then publish the advisory.
- **Test retries.** `.config/nextest.toml` retries a failing integration test in `tests/` once;
  nextest reports a test that passed on retry as FLAKY. Unit tests are never retried.
- **Community files**: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1),
  `SECURITY.md`, the issue forms and pull request template in `.github/`, and `CODEOWNERS`.

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
