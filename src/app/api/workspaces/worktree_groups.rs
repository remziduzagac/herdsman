//! `workspace.set_worktree_group`: named groups of a repository's linked
//! worktrees.
//!
//! herdsman's own module. `workspaces.rs` reaches it through hooks marked
//! `// fork: worktree groups`.

use super::*;
use crate::api::schema::{ResponseResult, WorkspaceSetWorktreeGroupParams};
use crate::app::state::fork::normalize_group_name;

impl App {
    /// Put a linked worktree's workspace into a named group within its
    /// repository, or take it out. Members carry the group as their
    /// `worktree_group` token, so a change reaches clients like any token.
    pub(in crate::app::api) fn handle_workspace_set_worktree_group(
        &mut self,
        id: String,
        params: WorkspaceSetWorktreeGroupParams,
    ) -> String {
        let Some(index) = self.parse_workspace_id(&params.workspace_id) else {
            return workspace_not_found(id, &params.workspace_id);
        };
        let Some(checkout) = self
            .state
            .workspaces
            .get(index)
            .and_then(|workspace| workspace.worktree_space())
            .filter(|space| space.is_linked_worktree)
            .map(|space| space.checkout_path.clone())
        else {
            return encode_error(
                id,
                "not_a_linked_worktree",
                "only a linked worktree's workspace can join a worktree group",
            );
        };
        let group = match normalize_group_name(params.group) {
            Ok(group) => group,
            Err(message) => return encode_error(id, "invalid_worktree_group", message),
        };
        if self.state.fork.worktree_groups.set(&checkout, group) {
            self.emit_workspace_token_updated(index);
        }
        encode_success(
            id,
            ResponseResult::WorkspaceInfo {
                workspace: self.workspace_info(index),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{ErrorResponse, SuccessResponse, WORKTREE_GROUP_TOKEN};
    use crate::config::Config;
    use crate::workspace::{Workspace, WorktreeSpaceMembership};

    fn membership(checkout: &str, linked: bool) -> WorktreeSpaceMembership {
        WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdsman".into(),
            repo_root: "/repo/herdsman".into(),
            checkout_path: checkout.into(),
            is_linked_worktree: linked,
        }
    }

    /// The repository's main checkout and one linked worktree.
    fn app_with_repo() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("main"), Workspace::test_new("feature")];
        app.state.ensure_test_terminals();
        app.state.workspaces[0].worktree_space = Some(membership("/repo/herdsman", false));
        app.state.workspaces[1].worktree_space = Some(membership("/repo/herdsman-feature", true));
        app
    }

    fn set_group(app: &mut App, index: usize, group: Option<&str>) -> String {
        let workspace_id = app.public_workspace_id(index);
        app.handle_workspace_set_worktree_group(
            "req".into(),
            WorkspaceSetWorktreeGroupParams {
                workspace_id,
                group: group.map(str::to_owned),
            },
        )
    }

    fn group_token(app: &App, index: usize) -> Option<String> {
        app.workspace_info(index)
            .tokens
            .get(WORKTREE_GROUP_TOKEN)
            .cloned()
    }

    #[test]
    fn a_linked_worktree_joins_and_leaves_a_group_as_a_token() {
        let mut app = app_with_repo();

        let response = set_group(&mut app, 1, Some(" features "));

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::WorkspaceInfo { workspace } = success.result else {
            panic!("expected workspace info");
        };
        assert_eq!(
            workspace
                .tokens
                .get(WORKTREE_GROUP_TOKEN)
                .map(String::as_str),
            Some("features")
        );
        assert_eq!(group_token(&app, 1).as_deref(), Some("features"));

        set_group(&mut app, 1, Some(""));
        assert_eq!(group_token(&app, 1), None);
    }

    #[test]
    fn a_worktree_keeps_its_group_when_its_workspace_opens_again() {
        let mut app = app_with_repo();
        set_group(&mut app, 1, Some("features"));

        let mut reopened = Workspace::test_new("feature again");
        reopened.worktree_space = Some(membership("/repo/herdsman-feature", true));
        app.state.workspaces[1] = reopened;

        assert_eq!(group_token(&app, 1).as_deref(), Some("features"));
    }

    #[test]
    fn only_linked_worktrees_join_groups_and_names_are_checked() {
        let mut app = app_with_repo();

        for (index, group, code) in [
            (0, "features".to_owned(), "not_a_linked_worktree"),
            (1, "x".repeat(65), "invalid_worktree_group"),
        ] {
            let response = set_group(&mut app, index, Some(&group));
            let error: ErrorResponse = serde_json::from_str(&response).unwrap();
            assert_eq!(error.error.code, code);
        }
        assert_eq!(group_token(&app, 0), None);
        assert_eq!(group_token(&app, 1), None);
    }
}
