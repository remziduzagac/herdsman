//! herdsman's own client-shell state, kept in single fields of the base project's types.
//!
//! herdsman's own module. `shell.rs` and `state.rs` reach it through hooks
//! marked `// fork: worktree groups` and `// fork: agents collapse`.

/// herdsman's own client state, beside the base project's in `ClientShellState`.
#[derive(Debug, Default)]
pub(super) struct ForkState {
    /// The agents panel shows only its header; the spaces list takes the rest.
    pub(super) agents_collapsed: bool,
    pub(super) creation_generation: u64,
    pub(super) creation: Option<super::presets::CreationState>,
}

impl ForkState {
    pub(super) fn new(config: &super::ClientShellConfig) -> Self {
        Self {
            agents_collapsed: config.agent_panel_start_collapsed,
            ..Self::default()
        }
    }
}

/// herdsman's click targets, rebuilt with the rest of the hit map each frame.
#[derive(Debug, Default)]
pub(super) struct ForkHits {
    pub(super) creation_preset: ratatui::layout::Rect,
    pub(super) worktree_group_headers: Vec<super::sidebar::worktree_groups::GroupHeaderHit>,
    /// The agents panel's title, which collapses or opens it.
    pub(super) agents_toggle: ratatui::layout::Rect,
}
