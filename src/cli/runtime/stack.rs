//! Requests behind the stack CLI commands.
//!
//! herdsman's own module. `runtime.rs` declares it with a hook marked
//! `// fork: stacks`.

use super::*;

pub(in crate::cli) fn pane_stack(
    params: crate::api::schema::PaneStackParams,
) -> std::io::Result<i32> {
    print_method_response("cli:pane:stack", Method::PaneStack(params))
}

pub(in crate::cli) fn pane_stacks(
    params: crate::api::schema::PaneStacksParams,
) -> std::io::Result<i32> {
    print_method_response("cli:pane:stacks", Method::PaneStacks(params))
}

pub(in crate::cli) fn pane_focus_id(pane_id: String) -> std::io::Result<i32> {
    print_method_response(
        "cli:pane:focus",
        Method::PaneFocus(crate::api::schema::PaneTarget { pane_id }),
    )
}

pub(in crate::cli) fn pane_focus_stacked(
    params: crate::api::schema::PaneFocusStackedParams,
) -> std::io::Result<i32> {
    print_method_response("cli:pane:focus-stacked", Method::PaneFocusStacked(params))
}
