//! API types for worktree groups: named groups of a repository's linked
//! worktrees, which the sidebar nests under the repository.
//!
//! herdsman's own module. `workspaces.rs` re-exports it through hooks marked
//! `// fork: worktree groups`.

use super::*;

/// The workspace token naming the group a linked worktree belongs to.
pub const WORKTREE_GROUP_TOKEN: &str = "worktree_group";

/// Put a linked worktree's workspace into a named group within its
/// repository, or take it out when `group` is absent or empty. A group exists
/// while it has members; each member carries its name as the
/// `worktree_group` token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceSetWorktreeGroupParams {
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}
