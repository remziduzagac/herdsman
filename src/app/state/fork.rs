//! herdsman's own state on `AppState`, kept in one field so the base project's struct
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio::sync::{mpsc, Notify};

    use super::*;
    use crate::api::schema::WORKTREE_GROUP_TOKEN;

    #[tokio::test]
    async fn a_restored_worktree_keeps_its_group_while_its_checkout_is_missing() {
        let mut snapshot: crate::persist::SessionSnapshot = serde_json::from_str(include_str!(
            "../../../tests/fixtures/session/current-herdsman-session.json"
        ))
        .unwrap();
        let cwd = std::env::current_dir().unwrap();
        let missing = cwd.join("__herdsman_missing_worktree_group_checkout__");
        assert!(!missing.exists());
        for workspace in &mut snapshot.workspaces {
            workspace.identity_cwd = cwd.clone();
            for tab in &mut workspace.tabs {
                for pane in tab.panes.values_mut() {
                    pane.cwd = missing.clone();
                }
            }
        }
        snapshot.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "saved-repo-key".into(),
            label: "saved-repo".into(),
            repo_root: cwd.clone(),
            checkout_path: missing.clone(),
            is_linked_worktree: true,
        });

        // A missing shell keeps every pane unavailable, so nothing is spawned.
        let (events, _rx) = mpsc::channel(32);
        let (workspaces, _terminals, _runtimes) = crate::persist::restore(
            &snapshot,
            None,
            24,
            80,
            0,
            "__herdsman_missing_restore_shell__",
            crate::config::ShellModeConfig::NonLogin,
            false,
            events,
            Arc::new(Notify::new()),
            Arc::new(crate::render_signal::RenderSignal::new()),
        );

        let mut fork = ForkState {
            worktree_groups: WorktreeGroups::open(PathBuf::from("unused.json"), false, false),
            ..ForkState::default()
        };
        assert!(fork.worktree_groups.set(&missing, Some("review".into())));
        let tokens = fork.workspace_tokens(&workspaces[0]);
        assert_eq!(
            tokens.get(WORKTREE_GROUP_TOKEN).map(String::as_str),
            Some("review")
        );
    }
}
