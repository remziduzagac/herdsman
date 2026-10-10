# Merging a herdr release

Assumes [SKILL.md](SKILL.md): whose file is which, and the branches. The commands are in
`MAINTAINING.md`, "Taking a herdr release"; this file is the order of the work, the decisions on the
way, and when each step is done. Read the matching `MAINTAINING.md` section as each step starts.

The person chooses the herdr revision: normally a release tag, sometimes a commit on herdr's
`master` for fixes that cannot wait.

## 1. See what changed

Follow "1. See what changed". Report to the person:

- the revision last imported and the one proposed, with herdr's version for each;
- how many commits lie between them, and the headline changes from herdr's `CHANGELOG.md`;
- how much changed in `src/layout.rs`, `src/persist/` and `src/ui/panes.rs`, which carry stacks;
- what changed in files the import drops, such as herdr's workflows in `.github/`.

Done when the person has agreed to the revision.

## 2. Import

Follow "2. Import". When it stops:

- **A licence changed:** show the person the licence diff and wait. Only the person accepts a
  licence change (`--accept-license-change`), or decides to stay on the previous release.
- **The name `herdr` survives somewhere:** teach `upstream/rename` the case on `dev`, propose that
  commit, and import again once it has landed.
- **A `DROP` entry no longer exists:** herdr moved a file. Find where it went and propose the
  `DROP` update the same way.

Done when `git log -1 --format=%b herdr-import` names the chosen revision.

## 3. Merge

Follow "3. Merge": create the worktree, merge, and resolve each conflict by its kind in that
section's list. Where herdr changed code that holds a herdsman hook, keep herdr's change and put the
hook back, as small as before; read [FORK-CODE.md](FORK-CODE.md) for how. Never drop herdr's side to
make a hook fit, and never add a `_ =>` arm to quiet a match.

Done when `git diff --name-only --diff-filter=U` and `git diff --cached --check` print nothing, and
`upstream/rename check .` is clean.

## 4. Rebuild the generated parts

Follow "4. Rebuild the generated parts". Every compile error from a new herdr `match` gets the stack
arm its neighbours have.

Done when `ZIG=true cargo check --locked --all-targets` passes and the API schema is regenerated.

## 5. Check

Run `just check`. Sort each failure: one caused by the rename, as "5. Check" describes (a pinned
hash, width or method-shape digest), is re-pinned; anything else is a real failure, to fix or to
report.

Done when `just check` passes, or the person has the list of failures that remain, and every re-pin
is reported with its reason.

## 6. Changelog

Run `upstream/changelog <version> --write`, as "6. Changelog, promote" describes, then read the
result and edit what herdsman does differently, such as a herdr feature herdsman drops or replaces.

Done when the `## Unreleased` section names the new herdr version and reads true for herdsman.

## 7. Hand back

Update `MAINTAINING.md`, "herdsman's edits to herdr files", when the merge moved or added a hook.
Propose the commit message (`chore: merge herdr <version>`) and commit in the worktree once it is
agreed. Merging into `dev`, removing the worktree and moving `test` or `main` are the person's: offer
the commands from "6. Changelog, promote".

Then go through the changes in the files the import drops (step 1). Propose each one worth taking,
such as a CI fix, as its own commit on `dev` after the merge.

Done when the merge commit exists and the person has the report: what came in, what conflicted and
how it was resolved, what was re-pinned, and which changes in dropped files were proposed or left
out.
