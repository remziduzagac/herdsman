//! `herdsman preset` in the CLI spec.
//!
//! herdsman's own module. `spec.rs` reaches it through hooks marked
//! `// fork: presets`.

use super::*;

pub(super) fn preset_command() -> Command {
    Command::new("preset")
        .about("Save a tab as a preset and open presets as new tabs")
        .subcommand(Command::new("list").about("List saved presets"))
        .subcommand(
            Command::new("show")
                .about("Show a preset's layout")
                .arg(required("name", "NAME")),
        )
        .subcommand(
            Command::new("save")
                .about("Save a tab as a preset")
                .arg(required("name", "NAME"))
                .arg(Arg::new("tab_id").value_name("TAB_ID"))
                .arg(flag("current"))
                .arg(flag("overwrite")),
        )
        .subcommand(
            Command::new("load")
                .about("Open a preset as a new tab")
                .arg(required("name", "NAME"))
                .arg(option("workspace", "WORKSPACE_ID"))
                .arg(flag("current"))
                .arg(flag("no-focus")),
        )
        .subcommand(
            Command::new("edit")
                .about("Open a preset's file in $VISUAL or $EDITOR and check it afterwards")
                .arg(required("name", "NAME")),
        )
        .subcommand(
            Command::new("rename")
                .about("Rename a preset")
                .arg(required("name", "NAME"))
                .arg(required("new_name", "NEW_NAME")),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a preset")
                .arg(required("name", "NAME")),
        )
}
