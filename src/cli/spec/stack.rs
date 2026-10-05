//! `herdsman pane stack`, `pane stacks` and `pane focus-stacked` in the CLI
//! spec.
//!
//! herdsman's own module. `spec.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;

pub(super) fn pane_subcommands() -> Vec<Command> {
    vec![
        Command::new("stacks")
            .about("List stacks: members and the visible one")
            .arg(option("workspace", "ID")),
        Command::new("focus-stacked")
            .about("Show and focus a member of a pane's stack")
            // `[PANE_ID] MEMBER`: an optional positional cannot precede a
            // required one, so both share one argument of one or two values.
            .arg(
                Arg::new("member")
                    .value_name("[PANE_ID] N|next|previous")
                    .num_args(1..=2)
                    .required(true),
            )
            .args(current_pane_args()),
        Command::new("stack")
            .about("Open a pane stacked in a pane's slot")
            .arg(Arg::new("pane_id").value_name("PANE_ID"))
            .args(current_pane_args())
            .arg(path_option("cwd", "PATH"))
            .arg(env_option())
            .arg(option("right-click", "TARGET").value_parser(["herdsman", "pane"]))
            .arg(flag("focus"))
            .arg(flag("no-focus")),
    ]
}
