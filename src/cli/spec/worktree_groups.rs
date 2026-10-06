//! `herdsman workspace group` in the CLI spec.
//!
//! herdsman's own module. `spec.rs` reaches it through hooks marked
//! `// fork: worktree groups`.

use super::*;

pub(super) fn workspace_group_command() -> Command {
    Command::new("group")
        .about("Put a linked worktree's workspace into a named group, or take it out")
        .arg(required("workspace_id", "WORKSPACE_ID"))
        .arg(
            Arg::new("name")
                .value_name("NAME|--clear")
                .num_args(1..)
                .required(true)
                .allow_hyphen_values(true),
        )
}
