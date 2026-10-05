//! The strip a stacked slot draws inside its visible member's frame: one
//! segment per member, numbered for `focus_stacked`, with the visible one lit.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use super::panes::pane_inner_rect;
use super::text::{display_width, truncate_end};
use super::widgets::panel_contrast_fg;
use crate::api::schema::PaneContentEdge;
use crate::app::AppState;
use crate::config::TabBarPositionConfig;
use crate::detect::AgentState;
use crate::layout::{PaneId, PaneInfo};
use crate::workspace::Tab;

/// Where a stack's strip goes inside its slot, and whether a line separates it
/// from the content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StripPlacement {
    pub(crate) position: TabBarPositionConfig,
    pub(crate) separator: bool,
}

/// Bottom, with the separator line: `ui.stack_strip_position` and
/// `ui.stack_strip_separator` as configured by default.
impl Default for StripPlacement {
    fn default() -> Self {
        Self {
            position: TabBarPositionConfig::Bottom,
            separator: true,
        }
    }
}

impl StripPlacement {
    pub(crate) fn of(app: &AppState) -> Self {
        app.fork.stack_strip
    }

    /// The edge of the content the strip sits beside.
    pub(crate) fn edge(self) -> PaneContentEdge {
        match self.position {
            TabBarPositionConfig::Top => PaneContentEdge::Top,
            TabBarPositionConfig::Bottom => PaneContentEdge::Bottom,
        }
    }

    /// How many rows out from the content the strip sits, 1 being the row
    /// next to it. Assumes the separator fits, which it misses only in a slot
    /// two rows tall, where a click on the strip then just focuses the pane.
    pub(crate) fn strip_offset(self) -> u16 {
        if self.separator {
            2
        } else {
            1
        }
    }
}

/// The rows a stacked pane's inner rect divides into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StripRows {
    pub(crate) strip: Rect,
    pub(crate) separator: Option<Rect>,
    pub(crate) content: Rect,
}

/// Split a stacked pane's inner rect into its strip row, the separator line
/// between strip and content, and the content. The separator is left out when
/// it would take the content's last row. None when there is no room for strip
/// and content, so the content keeps every row.
pub(crate) fn split_strip(pane_inner: Rect, placement: StripPlacement) -> Option<StripRows> {
    if pane_inner.height < 2 || pane_inner.width == 0 {
        return None;
    }
    let separator_height = u16::from(placement.separator && pane_inner.height >= 3);
    let content_height = pane_inner.height - 1 - separator_height;
    let (strip_y, separator_y, content_y) = match placement.position {
        TabBarPositionConfig::Top => (
            pane_inner.y,
            pane_inner.y + 1,
            pane_inner.y + 1 + separator_height,
        ),
        TabBarPositionConfig::Bottom => (
            pane_inner.y + content_height + separator_height,
            pane_inner.y + content_height,
            pane_inner.y,
        ),
    };
    let row = |y| Rect::new(pane_inner.x, y, pane_inner.width, 1);
    Some(StripRows {
        strip: row(strip_y),
        separator: (separator_height > 0).then(|| row(separator_y)),
        content: Rect::new(pane_inner.x, content_y, pane_inner.width, content_height),
    })
}

/// The rect a drawn pane's terminal content gets: a stack's visible member
/// gives up rows of its frame to the strip and its separator.
pub(crate) fn content_rect(
    tab: &Tab,
    pane_id: PaneId,
    pane_inner: Rect,
    placement: StripPlacement,
) -> Rect {
    if tab.layout.stack_members(pane_id).is_none() {
        return pane_inner;
    }
    split_strip(pane_inner, placement).map_or(pane_inner, |rows| rows.content)
}

/// Column offset and width of each member's segment in a strip `width` cells
/// wide. Segments keep their natural widths while all fit, and otherwise
/// shrink to an equal share so every member stays visible.
pub(crate) fn segment_spans(width: u16, natural_widths: &[u16]) -> Vec<(u16, u16)> {
    let count = u16::try_from(natural_widths.len()).unwrap_or(u16::MAX);
    if count == 0 || width == 0 {
        return Vec::new();
    }
    let total: u32 = natural_widths.iter().map(|w| u32::from(*w)).sum();
    let share = (width / count).max(1);
    let mut x = 0u16;
    natural_widths
        .iter()
        .map_while(|natural| {
            let segment = if total <= u32::from(width) {
                *natural
            } else {
                (*natural).min(share)
            };
            let segment = segment.min(width.saturating_sub(x));
            (segment > 0).then(|| {
                let span = (x, segment);
                x += segment;
                span
            })
        })
        .collect()
}

struct MemberLabel {
    dot: Option<Color>,
    text: String,
}

impl MemberLabel {
    fn natural_width(&self) -> u16 {
        let dot = if self.dot.is_some() { 2 } else { 0 };
        u16::try_from(display_width(&self.text) + dot + 2).unwrap_or(u16::MAX)
    }
}

fn member_label(app: &AppState, tab: &Tab, pane_id: PaneId, index: usize) -> MemberLabel {
    let terminal = tab
        .terminal_id(pane_id)
        .and_then(|terminal_id| app.terminals.get(terminal_id));
    let name = terminal.and_then(|terminal| {
        terminal
            .border_label(true)
            .or_else(|| terminal.terminal_title_stripped())
    });
    let seen = tab.panes.get(&pane_id).is_none_or(|pane| pane.seen);
    let dot = terminal.and_then(|terminal| match (terminal.state, seen) {
        (AgentState::Working, _) => Some(app.palette.yellow),
        (AgentState::Blocked, _) => Some(app.palette.red),
        (AgentState::Idle, false) => Some(app.palette.teal),
        (AgentState::Idle, true) if terminal.effective_known_agent().is_some() => {
            Some(app.palette.green)
        }
        _ => None,
    });
    let number = index + 1;
    MemberLabel {
        dot,
        text: match name {
            Some(name) => format!("{number} {}", name.trim()),
            None => number.to_string(),
        },
    }
}

fn member_labels(app: &AppState, tab: &Tab, members: &[PaneId]) -> Vec<MemberLabel> {
    members
        .iter()
        .enumerate()
        .map(|(index, member)| member_label(app, tab, *member, index))
        .collect()
}

fn natural_widths(labels: &[MemberLabel]) -> Vec<u16> {
    labels.iter().map(MemberLabel::natural_width).collect()
}

/// The stack member whose segment covers `column`, counted from the left edge
/// of the pane's content in a strip `width` cells wide. Segments are laid out
/// over the content width, which clients also know, so a click resolves to the
/// segment that was drawn under it.
pub(crate) fn stack_member_at(
    app: &AppState,
    tab: &Tab,
    pane_id: PaneId,
    column: u16,
    width: u16,
) -> Option<PaneId> {
    let (members, _) = tab.layout.stack_members(pane_id)?;
    let widths = natural_widths(&member_labels(app, tab, members));
    segment_spans(width, &widths)
        .into_iter()
        .zip(members)
        .find(|((offset, span), _)| column >= *offset && column - offset < *span)
        .map(|(_, member)| *member)
}

/// Draw the strip of every stack visible in a rendered tab.
pub(super) fn render_stack_strips(
    app: &AppState,
    tab: &Tab,
    pane_infos: &[PaneInfo],
    buf: &mut Buffer,
) {
    if tab.zoomed {
        return;
    }
    for info in pane_infos {
        let Some((members, active)) = tab.layout.stack_members(info.id) else {
            continue;
        };
        let Some(rows) = split_strip(
            pane_inner_rect(info.rect, info.borders),
            StripPlacement::of(app),
        ) else {
            continue;
        };
        if let Some(separator) = rows.separator {
            render_separator(app, info, separator.intersection(buf.area), buf);
        }
        let row = rows.strip.intersection(buf.area);
        let strip = Rect::new(info.inner_rect.x, row.y, info.inner_rect.width, 1).intersection(row);
        if strip.is_empty() {
            continue;
        }
        let labels = member_labels(app, tab, members);
        let widths = natural_widths(&labels);
        buf.set_style(row, Style::default().bg(app.palette.panel_bg));
        for (index, ((offset, width), label)) in segment_spans(strip.width, &widths)
            .into_iter()
            .zip(&labels)
            .enumerate()
        {
            let style = if index != active {
                Style::default()
                    .fg(app.palette.overlay1)
                    .bg(app.palette.panel_bg)
            } else if info.is_focused {
                Style::default()
                    .fg(panel_contrast_fg(&app.palette))
                    .bg(app.palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(app.palette.text)
                    .bg(app.palette.surface1)
                    .add_modifier(Modifier::BOLD)
            };
            let segment = Rect::new(strip.x + offset, strip.y, width, 1);
            buf.set_style(segment, style);
            let mut x = segment.x.saturating_add(1);
            let end = segment.x.saturating_add(segment.width.saturating_sub(1));
            if let Some(dot) = label.dot {
                if x + 2 <= end {
                    buf.set_string(x, segment.y, "●", style.fg(dot));
                    x += 2;
                }
            }
            if x < end {
                let text = truncate_end(&label.text, usize::from(end - x));
                buf.set_stringn(x, segment.y, text, usize::from(end - x), style);
            }
        }
    }
}

/// A line across the slot between strip and content, coloured like the
/// pane's border so it reads as part of the frame.
fn render_separator(app: &AppState, info: &PaneInfo, row: Rect, buf: &mut Buffer) {
    let style = if info.is_focused {
        Style::default()
            .fg(app.palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.palette.overlay0)
    };
    for x in row.x..row.x.saturating_add(row.width) {
        buf[(x, row.y)].set_symbol("─").set_style(style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::TerminalRuntimeRegistry;
    use crate::workspace::Workspace;

    /// `editor | S[agent, *shell]`, labelled, with the stack focused.
    fn stacked_app(position: TabBarPositionConfig) -> (AppState, [PaneId; 3]) {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("stacks")];
        app.active = Some(0);
        app.fork.stack_strip.position = position;
        let editor = app.workspaces[0].tabs[0].root_pane;
        let agent = app.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        let shell = app.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        assert!(app.workspaces[0].tabs[0]
            .layout
            .move_into_stack(agent, shell));
        app.ensure_test_terminals();
        for (pane, label) in [(editor, "editor"), (agent, "agent"), (shell, "shell")] {
            let terminal_id = app.workspaces[0].terminal_id(pane).unwrap().clone();
            app.terminals
                .get_mut(&terminal_id)
                .unwrap()
                .set_manual_label(label.into());
        }
        (app, [editor, agent, shell])
    }

    fn render(app: &AppState) -> (Vec<PaneInfo>, Buffer) {
        let runtimes = TerminalRuntimeRegistry::new();
        let area = Rect::new(0, 0, 80, 12);
        let layout = crate::ui::compute_tab_surface_for(
            app,
            &runtimes,
            Some(crate::ui::TabSurfaceTarget {
                workspace_index: 0,
                tab_index: 0,
            }),
            area,
            false,
            Default::default(),
        );
        let infos = layout.pane_infos.clone();
        let (buffer, _, _, _) =
            crate::server::render_stream::render_tab_surface_virtual(app, &runtimes, layout, area);
        (infos, buffer)
    }

    fn row_text(buffer: &Buffer, row: Rect) -> String {
        (row.x..row.x + row.width)
            .map(|x| buffer[(x, row.y)].symbol())
            .collect()
    }

    fn placement(position: TabBarPositionConfig, separator: bool) -> StripPlacement {
        StripPlacement {
            position,
            separator,
        }
    }

    #[test]
    fn stacked_slot_draws_a_numbered_strip_past_a_line_and_content_loses_two_rows() {
        for position in [TabBarPositionConfig::Top, TabBarPositionConfig::Bottom] {
            let (app, [editor, agent, shell]) = stacked_app(position);

            let (infos, buffer) = render(&app);

            assert!(infos.iter().all(|info| info.id != agent));
            let info = |id| infos.iter().find(|info| info.id == id).unwrap();
            let rows = split_strip(
                pane_inner_rect(info(shell).rect, info(shell).borders),
                placement(position, true),
            )
            .unwrap();
            assert_eq!(info(shell).inner_rect, rows.content, "{position:?}");
            assert_eq!(
                info(editor).inner_rect.height,
                info(shell).inner_rect.height + 2
            );
            let text = row_text(&buffer, rows.strip);
            assert!(
                text.contains("1 agent") && text.contains("2 shell"),
                "{position:?}: {text:?}"
            );
            let separator = rows.separator.expect("room for the line");
            assert_eq!(
                row_text(&buffer, separator),
                "─".repeat(usize::from(separator.width)),
                "{position:?}"
            );
            let line = buffer[(separator.x, separator.y)].style();
            assert_eq!(
                (line.fg, line.add_modifier.contains(Modifier::BOLD)),
                (Some(app.palette.accent), true),
                "the focused slot's line takes the focused border colour"
            );
        }
    }

    #[test]
    fn without_the_separator_the_strip_sits_next_to_the_content() {
        let (mut app, [editor, _, shell]) = stacked_app(TabBarPositionConfig::Bottom);
        app.fork.stack_strip.separator = false;

        let (infos, buffer) = render(&app);

        let info = |id| infos.iter().find(|info| info.id == id).unwrap();
        let rows = split_strip(
            pane_inner_rect(info(shell).rect, info(shell).borders),
            StripPlacement::of(&app),
        )
        .unwrap();
        assert_eq!(rows.separator, None);
        assert_eq!(rows.strip.y, info(shell).inner_rect.bottom());
        assert_eq!(
            info(editor).inner_rect.height,
            info(shell).inner_rect.height + 1
        );
        assert!(row_text(&buffer, rows.strip).contains("1 agent"));
    }

    #[test]
    fn a_strip_column_resolves_to_the_member_whose_segment_covers_it() {
        // " 1 agent " and " 2 shell " are nine cells each.
        let (app, [editor, agent, shell]) = stacked_app(TabBarPositionConfig::Top);
        let tab = &app.workspaces[0].tabs[0];
        let at = |column, width| stack_member_at(&app, tab, shell, column, width);

        assert_eq!(
            [at(0, 40), at(8, 40), at(9, 40), at(17, 40), at(18, 40)],
            [Some(agent), Some(agent), Some(shell), Some(shell), None]
        );
        assert_eq!([at(4, 10), at(5, 10)], [Some(agent), Some(shell)]);
        assert_eq!(stack_member_at(&app, tab, editor, 0, 40), None);
    }

    #[test]
    fn each_rendered_label_resolves_to_its_own_member() {
        for position in [TabBarPositionConfig::Top, TabBarPositionConfig::Bottom] {
            let (app, [_, agent, shell]) = stacked_app(position);
            let (infos, buffer) = render(&app);
            let info = infos.iter().find(|info| info.id == shell).unwrap();
            let rows = split_strip(
                pane_inner_rect(info.rect, info.borders),
                StripPlacement::of(&app),
            )
            .unwrap();
            let content = info.inner_rect;
            let text = row_text(
                &buffer,
                Rect::new(content.x, rows.strip.y, content.width, 1),
            );
            let tab = &app.workspaces[0].tabs[0];

            for (label, member) in [("1 agent", agent), ("2 shell", shell)] {
                let column = u16::try_from(text.find(label).unwrap()).unwrap();
                assert_eq!(
                    stack_member_at(&app, tab, shell, column, content.width),
                    Some(member),
                    "{position:?} {label}: {text:?}"
                );
            }
        }
    }

    #[test]
    fn zoomed_stack_member_fills_the_tab_without_a_strip() {
        let (mut app, [_, _, shell]) = stacked_app(TabBarPositionConfig::Top);
        app.workspaces[0].tabs[0].zoomed = true;

        let (infos, buffer) = render(&app);

        let [info] = &infos[..] else {
            panic!("zoom shows one pane");
        };
        assert_eq!(info.id, shell);
        assert_eq!(info.inner_rect, pane_inner_rect(info.rect, info.borders));
        let top_rows = (0..3)
            .map(|y| row_text(&buffer, Rect::new(0, y, 80, 1)))
            .collect::<String>();
        assert!(!top_rows.contains("1 agent"), "{top_rows:?}");
    }

    #[test]
    fn strip_defaults_to_the_bottom_of_its_slot_behind_a_line() {
        let ui = crate::config::Config::default().ui;
        assert_eq!(
            (ui.stack_strip_position, ui.stack_strip_separator),
            (TabBarPositionConfig::Bottom, true)
        );
        assert_eq!(
            StripPlacement::of(&AppState::test_new()),
            placement(TabBarPositionConfig::Bottom, true)
        );
    }

    #[test]
    fn strip_takes_the_first_or_last_inner_row() {
        let inner = Rect::new(2, 3, 20, 10);
        let rows = |strip, content| StripRows {
            strip,
            separator: None,
            content,
        };

        assert_eq!(
            split_strip(inner, placement(TabBarPositionConfig::Top, false)),
            Some(rows(Rect::new(2, 3, 20, 1), Rect::new(2, 4, 20, 9)))
        );
        assert_eq!(
            split_strip(inner, placement(TabBarPositionConfig::Bottom, false)),
            Some(rows(Rect::new(2, 12, 20, 1), Rect::new(2, 3, 20, 9)))
        );
        assert_eq!(
            split_strip(
                Rect::new(0, 0, 20, 1),
                placement(TabBarPositionConfig::Top, false)
            ),
            None
        );
    }

    #[test]
    fn separator_takes_the_row_between_strip_and_content() {
        let inner = Rect::new(2, 3, 20, 10);

        assert_eq!(
            split_strip(inner, placement(TabBarPositionConfig::Top, true)),
            Some(StripRows {
                strip: Rect::new(2, 3, 20, 1),
                separator: Some(Rect::new(2, 4, 20, 1)),
                content: Rect::new(2, 5, 20, 8),
            })
        );
        assert_eq!(
            split_strip(inner, placement(TabBarPositionConfig::Bottom, true)),
            Some(StripRows {
                strip: Rect::new(2, 12, 20, 1),
                separator: Some(Rect::new(2, 11, 20, 1)),
                content: Rect::new(2, 3, 20, 8),
            })
        );
    }

    #[test]
    fn separator_gives_way_before_the_last_content_row() {
        for position in [TabBarPositionConfig::Top, TabBarPositionConfig::Bottom] {
            let short = split_strip(Rect::new(0, 0, 20, 2), placement(position, true)).unwrap();
            assert_eq!((short.separator, short.content.height), (None, 1));
            let roomy = split_strip(Rect::new(0, 0, 20, 3), placement(position, true)).unwrap();
            assert!(roomy.separator.is_some());
            assert_eq!(roomy.content.height, 1);
        }
    }

    #[test]
    fn strip_offset_counts_the_separator() {
        for position in [TabBarPositionConfig::Top, TabBarPositionConfig::Bottom] {
            for separator in [false, true] {
                let placement = placement(position, separator);
                let inner = Rect::new(0, 5, 20, 10);
                let rows = split_strip(inner, placement).unwrap();
                let offset = match placement.edge() {
                    PaneContentEdge::Top => rows.content.y - rows.strip.y,
                    PaneContentEdge::Bottom => rows.strip.y - rows.content.bottom() + 1,
                };
                assert_eq!(offset, placement.strip_offset(), "{placement:?}");
            }
        }
    }

    #[test]
    fn segments_keep_natural_widths_while_they_fit() {
        assert_eq!(segment_spans(40, &[6, 10, 8]), [(0, 6), (6, 10), (16, 8)]);
    }

    #[test]
    fn segments_shrink_to_equal_shares_so_every_member_stays_visible() {
        assert_eq!(segment_spans(12, &[10, 3, 10]), [(0, 4), (4, 3), (7, 4)]);
        assert_eq!(segment_spans(2, &[5, 5, 5]), [(0, 1), (1, 1)]);
        assert!(segment_spans(0, &[5]).is_empty());
        assert!(segment_spans(10, &[]).is_empty());
    }
}
