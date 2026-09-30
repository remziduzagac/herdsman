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
use crate::app::AppState;
use crate::config::TabBarPositionConfig;
use crate::detect::AgentState;
use crate::layout::{PaneId, PaneInfo};
use crate::workspace::Tab;

/// Split a stacked pane's inner rect into its strip row and its content. None
/// when there is no room for both, so the content keeps every row.
pub(crate) fn split_strip(
    pane_inner: Rect,
    position: TabBarPositionConfig,
) -> Option<(Rect, Rect)> {
    if pane_inner.height < 2 || pane_inner.width == 0 {
        return None;
    }
    let content_height = pane_inner.height - 1;
    let (strip_y, content_y) = match position {
        TabBarPositionConfig::Top => (pane_inner.y, pane_inner.y + 1),
        TabBarPositionConfig::Bottom => (pane_inner.y + content_height, pane_inner.y),
    };
    Some((
        Rect::new(pane_inner.x, strip_y, pane_inner.width, 1),
        Rect::new(pane_inner.x, content_y, pane_inner.width, content_height),
    ))
}

/// The rect a drawn pane's terminal content gets: a stack's visible member
/// gives up one row of its frame to the strip.
pub(crate) fn content_rect(
    tab: &Tab,
    pane_id: PaneId,
    pane_inner: Rect,
    position: TabBarPositionConfig,
) -> Rect {
    if tab.layout.stack_members(pane_id).is_none() {
        return pane_inner;
    }
    split_strip(pane_inner, position).map_or(pane_inner, |(_, content)| content)
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
        let Some((row, _)) = split_strip(
            pane_inner_rect(info.rect, info.borders),
            app.stack_strip_position,
        ) else {
            continue;
        };
        let row = row.intersection(buf.area);
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
        app.stack_strip_position = position;
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

    #[test]
    fn stacked_slot_draws_a_numbered_strip_and_content_loses_one_row() {
        for position in [TabBarPositionConfig::Top, TabBarPositionConfig::Bottom] {
            let (app, [editor, agent, shell]) = stacked_app(position);

            let (infos, buffer) = render(&app);

            assert!(infos.iter().all(|info| info.id != agent));
            let info = |id| infos.iter().find(|info| info.id == id).unwrap();
            let (strip, content) = split_strip(
                pane_inner_rect(info(shell).rect, info(shell).borders),
                position,
            )
            .unwrap();
            assert_eq!(info(shell).inner_rect, content, "{position:?}");
            assert_eq!(
                info(editor).inner_rect.height,
                info(shell).inner_rect.height + 1
            );
            let text = row_text(&buffer, strip);
            assert!(
                text.contains("1 agent") && text.contains("2 shell"),
                "{position:?}: {text:?}"
            );
        }
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
            let (row, _) = split_strip(pane_inner_rect(info.rect, info.borders), position).unwrap();
            let content = info.inner_rect;
            let text = row_text(&buffer, Rect::new(content.x, row.y, content.width, 1));
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
    fn strip_defaults_to_the_bottom_of_its_slot() {
        assert_eq!(
            crate::config::Config::default().ui.stack_strip_position,
            TabBarPositionConfig::Bottom
        );
        assert_eq!(
            AppState::test_new().stack_strip_position,
            TabBarPositionConfig::Bottom
        );
    }

    #[test]
    fn strip_takes_the_first_or_last_inner_row() {
        let inner = Rect::new(2, 3, 20, 10);

        assert_eq!(
            split_strip(inner, TabBarPositionConfig::Top),
            Some((Rect::new(2, 3, 20, 1), Rect::new(2, 4, 20, 9)))
        );
        assert_eq!(
            split_strip(inner, TabBarPositionConfig::Bottom),
            Some((Rect::new(2, 12, 20, 1), Rect::new(2, 3, 20, 9)))
        );
        assert_eq!(
            split_strip(Rect::new(0, 0, 20, 1), TabBarPositionConfig::Top),
            None
        );
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
