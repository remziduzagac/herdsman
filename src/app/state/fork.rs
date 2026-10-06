//! herdsman's own state on `AppState`, kept in one field so herdr's struct
//! gains a single line. `state.rs` and `app/mod.rs` reach it through hooks
//! marked `// fork: state`.

use std::collections::HashMap;

use crate::config::Config;
use crate::ui::StripPlacement;
use crate::workspace::Workspace;

mod worktree_groups;

pub(crate) use worktree_groups::{normalize_group_name, WorktreeGroups};

/// Everything herdsman keeps on `AppState`.
#[derive(Debug, Default)]
pub(crate) struct ForkState {
    /// Where stacked slots draw their strip, and whether a line separates it.
    pub stack_strip: StripPlacement,
    /// Set by `pane.stack` while it reuses `pane.split`, which then folds the
    /// new pane into the target's slot instead of leaving it beside it.
    pub(crate) split_into_stack: bool,
    /// Named groups of linked worktrees.
    pub(crate) worktree_groups: WorktreeGroups,
}

impl ForkState {
    /// The state for a new app: settings from `config`, and worktree groups
    /// restored and saved as `policy` says for the session.
    pub(crate) fn new(config: &Config, policy: &crate::app::AppPolicy) -> Self {
        let mut state = Self {
            worktree_groups: WorktreeGroups::open(
                crate::session::data_dir().join("worktree-groups.json"),
                policy.restore_session,
                policy.persist_session,
            ),
            ..Self::default()
        };
        state.apply_config(config);
        state
    }

    /// Take the settings herdsman reads from a (re)loaded config.
    pub fn apply_config(&mut self, config: &Config) {
        self.stack_strip = StripPlacement {
            position: config.ui.stack_strip_position,
            separator: config.ui.stack_strip_separator,
        };
    }

    /// A workspace's metadata tokens, with `worktree_group` added for a linked
    /// worktree in a group.
    pub(crate) fn workspace_tokens(&self, workspace: &Workspace) -> HashMap<String, String> {
        let mut tokens = workspace.metadata_tokens.values();
        if let Some(group) = workspace
            .worktree_space()
            .filter(|space| space.is_linked_worktree)
            .and_then(|space| self.worktree_groups.group(&space.checkout_path))
        {
            tokens.insert(
                crate::api::schema::WORKTREE_GROUP_TOKEN.to_owned(),
                group.to_owned(),
            );
        }
        tokens
    }
}
