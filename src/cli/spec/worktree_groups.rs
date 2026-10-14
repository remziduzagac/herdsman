//! `herdsman workspace section` in the CLI spec. `group`, its name in 1.1.0,
//! still works as a hidden alias.
//!
//! herdsman's own module. `spec.rs` reaches it through hooks marked
//! `// fork: worktree groups`.

use super::*;

pub(super) fn workspace_group_command() -> Command {
    Command::new("section")
        .alias("group")
        .about("Put a linked worktree's workspace into a section of its repository, or take it out")
        .arg(required("workspace_id", "WORKSPACE_ID"))
        .arg(
            Arg::new("name")
                .value_name("NAME|--clear")
                .num_args(1..)
                .required(true)
                .allow_hyphen_values(true),
        )
}
