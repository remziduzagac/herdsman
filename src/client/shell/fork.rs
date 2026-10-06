//! herdsman's own client-shell state, kept in single fields of the base project's types.
//!
//! herdsman's own module. `shell.rs` and `state.rs` reach it through hooks
//! marked `// fork: worktree groups`.

/// herdsman's click targets, rebuilt with the rest of the hit map each frame.
#[derive(Debug, Default)]
pub(super) struct ForkHits {
    pub(super) worktree_group_headers: Vec<super::sidebar::worktree_groups::GroupHeaderHit>,
}
