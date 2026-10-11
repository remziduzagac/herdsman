//! `herdsman tab equalize` in the CLI spec.
//!
//! herdsman's own module. `spec.rs` reaches it through hooks marked
//! `// fork: equalize`.

use super::*;

pub(super) fn tab_equalize_command() -> Command {
    Command::new("equalize")
        .about("Give a tab's panes equal widths or equal heights")
        .arg(required("dimension", "DIMENSION").value_parser(["width", "height"]))
        .arg(Arg::new("tab_id").value_name("TAB_ID"))
        .arg(flag("current"))
}
