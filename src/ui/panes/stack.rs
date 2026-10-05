//! Keeping a stack's hidden members sized like its visible one.
//!
//! herdsman's own module. `panes.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;

/// The rect a pane's terminal content gets: a stack's visible member gives up
/// rows of its frame to the strip and its separator.
pub(super) fn content_rect(
    app: &AppState,
    tab: &crate::workspace::Tab,
    pane_id: crate::layout::PaneId,
    pane_inner: Rect,
) -> Rect {
    super::super::stack_strip::content_rect(
        tab,
        pane_id,
        pane_inner,
        super::super::stack_strip::StripPlacement::of(app),
    )
}

/// Size a stack's hidden members like its visible one, so switching members
/// never reflows them. Unchanged sizes return early inside the runtime.
pub(super) fn resize_hidden_stack_members(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    workspace_index: usize,
    tab: &crate::workspace::Tab,
    visible: crate::layout::PaneId,
    pane_inner: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    let Some((members, _)) = tab.layout.stack_members(visible) else {
        return;
    };
    for member in members.iter().copied().filter(|member| *member != visible) {
        let Some((terminal_id, rt)) =
            runtime_for_tab_pane(app, terminal_runtimes, workspace_index, tab, member)
        else {
            continue;
        };
        if app.direct_attach_resize_locks.contains(terminal_id) {
            continue;
        }
        let inner_rect = terminal_inner_rect(rt, pane_inner, app.pane_scrollbars);
        rt.resize(
            inner_rect.height,
            inner_rect.width,
            cell_size.width_px,
            cell_size.height_px,
        );
    }
}
