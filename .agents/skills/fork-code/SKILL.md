---
name: fork-code
description: Write herdsman's own features so they stay cheap to merge with herdr. Use whenever you add or change herdsman-specific behaviour in this repository, especially in files that come from herdr (anything that exists on the herdr-import branch), and when moving existing herdsman code out of herdr files.
---

# Fork code: hooks in herdr files, logic in ours

herdsman takes in every herdr release with `git merge` (see `MAINTAINING.md`). Each line herdsman
changes in a herdr file is a line that can conflict at the next merge, or break silently when herdr
rewrites the code around it. So herdsman code follows one rule:

**herdr files get hooks, herdsman files get logic.** A hook is the smallest edit that lets herdr's
code reach ours: a `mod` line, an enum variant, a one-line match arm, a single struct field, a call
site. Everything else, including tests, lives in files herdr does not have.

## Whose file is it?

A file is herdr's when it exists on the import branch, renamed:

```bash
git cat-file -e herdr-import:src/layout.rs && echo herdr || echo herdsman
```

## Where herdsman code goes

Put a feature in new files next to the herdr module it extends, as a child module. A child module
sees its parent's private items, so moving code there needs no visibility changes in herdr files.

```rust
// src/layout.rs (herdr's): one hook line
mod stack; // fork: stacks

// src/layout/stack.rs (herdsman's): the logic and its tests
use super::*;

impl TileLayout {
    pub fn stack_members(&self, pane: PaneId) -> Option<(&[PaneId], usize)> { /* ... */ }
}

#[cfg(test)]
mod tests { /* ... */ }
```

- Name files after the feature: `stack.rs`, `stack_strip.rs`, `worktree_groups.rs`.
- Methods on herdr types go in a separate `impl` block in the herdsman file. Rust allows any number of
  inherent `impl` blocks for a type anywhere in the crate.
- Use `#[path = "..."]` only when a natural child location does not exist. Plain child modules keep
  rustfmt and rust-analyzer happy.
- Tests for herdsman behaviour go in the herdsman file's own `mod tests`, never in herdr's.

## The hooks, one by one

Mark every hook in a herdr file with a `// fork: <feature>` comment (`# fork:` in Python and TOML),
on the line or just above a multi-line hook, so `rg '(//|#) fork:'` lists every hook point and a
conflict shows at once which side is ours. herdr's code never uses this marker. JSON cannot carry it;
list a JSON hook in `MAINTAINING.md` instead.

Place a hook next to a related, long-standing line, not at the end of a struct, enum or list: the end
is where herdr appends its own additions, and two insertions at the same spot conflict.

### Enum variants and match arms

A variant is unavoidable when herdsman extends a herdr enum. Keep each arm a single delegation,
even when the arm needs real logic: move the logic into a function in the herdsman module.

```rust
// src/app/api.rs (herdr's)
Method::PaneStack(params) => return self.handle_pane_stack(request.id, params), // fork: stacks
```

The handler itself lives in `src/app/api/stack.rs` as `impl App { pub(super) fn handle_pane_stack(..) }`.

Never add a `_ =>` catch-all to a match over an enum herdsman extends, and never remove one herdr
has. When herdr adds a new `match`, the compiler must flag the missing herdsman arm.

### Struct fields

Add at most one field per herdr struct: a herdsman type holding everything herdsman keeps there, so
later features change only herdsman's type:

```rust
pub struct AppState {
    // ...herdr's fields...
    pub(crate) fork: fork::ForkState, // fork: state
}
```

`AppState`'s is `ForkState` in `src/app/state/fork.rs`; put new herdsman state there.

Prefer a side table keyed by a stable id (a workspace id, a checkout path) over fields on many herdr
types. Persisted and JSON structs need `#[serde(default)]` on the new field so older files and peers
still load.

Wire formats follow `AGENTS.md`, "Stable client endpoint contract": binary codecs and anything
pinned in `tests/fixtures` never change shape, and JSON messages such as the client snapshot and API
results only gain optional fields. A new capability is a new advertised method, not a new meaning for
an old one.

### Lists both sides extend

Method name tables, `CLIENT_SHELL_METHODS`, keybinding tables and defaults: add the entries, keep the
list's own order (some lists must stay sorted), and pin anything herdsman-specific, such as method
shape digests, in the herdsman block of the test that checks them.

### Call sites

When herdr code must do something new, call one herdsman function and keep the decision inside it.
Prefer adjusting herdr's result over editing herdr's line: keep the line as herdr wrote it and add
one after it.

```rust
let pane_inner = pane_inner_rect(info.rect, info.borders); // herdr's line, untouched
let pane_inner = stack::content_rect(app, tab, info.id, pane_inner); // fork: stacks
```

When a herdsman feature needs a herdr function to behave differently, keep the function's signature
and let it consult herdsman state through a hook. `pane.stack` reuses `handle_pane_split` this way:
it sets `ForkState::split_into_stack`, and one hook in the split folds the new pane into the stack.

## What not to do in herdr files

- No reformatting, reordering, renaming or moving herdr code, and no drive-by fixes. Each of these
  turns a clean merge into a conflict.
- Do not change a herdr function's signature or meaning; add a herdsman function instead.
- Do not edit herdr's tests. Add herdsman's in herdsman files: the herdsman module's own
  `mod tests`, or a test-only child module (`#[cfg(test)] mod stack;`) for tests of hooks in a herdr
  file. Widening a herdr test helper to `pub(super)` so herdsman's tests can reuse it is a hook; mark
  it `// fork: <feature> tests`.
- Do not copy a herdr function to change it slightly; call it, or hook the one place that differs.

When a hook would have to be large, stop and look for a seam: a function herdr calls that can be
wrapped, a value that can be adjusted after herdr computes it, or a child module that can take the
whole block.

## Before committing

1. Show this branch's changes to herdr files:

   ```bash
   for f in $(git diff --name-only dev); do
       git cat-file -e "herdr-import:$f" 2>/dev/null && git diff dev -- "$f"
   done
   ```

   Every hunk should be a hook carrying `// fork:`. Anything else needs a reason.
2. Update the feature's row in `MAINTAINING.md`, "herdsman's edits to herdr files", with any herdr
   file that gained a hook.
3. Run the checks in `AGENTS.md`, "Testing".
