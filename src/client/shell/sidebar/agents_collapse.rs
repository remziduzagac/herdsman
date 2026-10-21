//! The agents panel collapses to its header: a click on its title, or
//! `keys.toggle_agent_panel` (`prefix+a`), and the spaces list above takes the
//! rows it gave up. `[ui] agent_panel_start_collapsed` sets how a client starts.
//!
//! herdsman's own module. `sidebar.rs`, `endpoint_sidebar.rs`, `mouse.rs` and
//! `actions.rs` reach it through hooks marked `// fork: agents collapse`.
//!
//! Collapsing is presentation, so each client keeps its own state, which lasts
//! until the client exits.

use super::*;

/// The rows a collapsed panel keeps: its rule, its title, and the bottom row,
/// where the sidebar's own collapse button sits.
const COLLAPSED_ROWS: u16 = 3;

/// The panel's title, as `render_agent_panel_header` draws it.
const TITLE: &str = " agents";

/// The spaces and agents areas once the panel is collapsed: the agents area
/// keeps [`COLLAPSED_ROWS`] at the bottom, and the divider can't be dragged.
pub(in crate::client::shell) fn sections(
    workspace: Rect,
    detail: Rect,
    collapsed: bool,
    hits: &mut ShellHitMap,
) -> (Rect, Rect) {
    if !collapsed || detail.is_empty() {
        return (workspace, detail);
    }
    hits.sidebar_section_divider = Rect::default();
    let total = workspace.height.saturating_add(detail.height);
    let rows = COLLAPSED_ROWS.min(total);
    let workspace_height = total - rows;
    (
        Rect::new(workspace.x, workspace.y, workspace.width, workspace_height),
        Rect::new(
            detail.x,
            workspace.y.saturating_add(workspace_height),
            detail.width,
            rows,
        ),
    )
}

/// Marks the panel's title with `▾` (open) or `▸` (collapsed), and makes the
/// title and the mark the button. Left out where the title row has no room.
pub(in crate::client::shell) fn render_toggle(
    buffer: &mut Buffer,
    area: Rect,
    collapsed: bool,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    hits.fork.agents_toggle = Rect::default();
    if area.height < 2 {
        return;
    }
    let y = area.y + 1;
    let mark_x = area.x.saturating_add(display_width(TITLE) + 1);
    // The sort label sits at the row's right end; keep a blank cell before it.
    let free = |x: u16| x < area.right() && buffer[(x, y)].symbol() == " ";
    if !(free(mark_x) && free(mark_x.saturating_add(1))) {
        return;
    }
    put_text(
        buffer,
        mark_x,
        y,
        1,
        if collapsed { "▸" } else { "▾" },
        Style::default().fg(config.palette.accent),
    );
    if config.mouse_capture {
        hits.fork.agents_toggle = Rect::new(area.x, y, mark_x - area.x + 1, 1);
    }
}

impl ClientShellState {
    pub(in crate::client::shell) fn toggle_agent_panel(&mut self, outcome: &mut ClientShellInput) {
        self.fork.agents_collapsed = !self.fork.agents_collapsed;
        outcome.repaint = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(height: u16) -> (Rect, Rect) {
        (
            Rect::new(0, 0, 20, height / 2),
            Rect::new(0, height / 2, 20, height - height / 2),
        )
    }

    #[test]
    fn a_collapsed_panel_keeps_three_rows_and_gives_the_rest_to_spaces() {
        let mut hits = ShellHitMap {
            sidebar_section_divider: Rect::new(0, 20, 20, 1),
            ..ShellHitMap::default()
        };
        let (workspace, detail) = split(40);

        let (workspace, detail) = sections(workspace, detail, true, &mut hits);

        assert_eq!(workspace, Rect::new(0, 0, 20, 37));
        assert_eq!(detail, Rect::new(0, 37, 20, 3));
        assert_eq!(hits.sidebar_section_divider, Rect::default());
    }

    #[test]
    fn an_open_panel_keeps_the_split_and_its_divider() {
        let divider = Rect::new(0, 20, 20, 1);
        let mut hits = ShellHitMap {
            sidebar_section_divider: divider,
            ..ShellHitMap::default()
        };
        let (workspace, detail) = split(40);

        assert_eq!(
            sections(workspace, detail, false, &mut hits),
            (workspace, detail)
        );
        assert_eq!(hits.sidebar_section_divider, divider);
    }

    #[test]
    fn a_short_sidebar_gives_the_collapsed_panel_what_there_is() {
        let mut hits = ShellHitMap::default();
        let (workspace, detail) = split(2);

        let (workspace, detail) = sections(workspace, detail, true, &mut hits);

        assert_eq!((workspace.height, detail.height), (0, 2));
    }
}
