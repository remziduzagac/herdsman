//! Drawing the presets list, laid out like the open-worktree picker: a search
//! line, two-line rows, a status line and the open and close buttons.
//!
//! herdsman's own module. `overlays.rs` reaches it through hooks marked
//! `// fork: presets`.

use super::*;
use crate::client::shell::presets::PresetsOverlay;

pub(super) fn render_presets_overlay(
    b: &mut Buffer,
    list: &PresetsOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let popup_height = (list.entries.len().saturating_mul(2) + 8).clamp(12, 26) as u16;
    let popup = popup(b.area, 72, popup_height)?;
    let inner = panel(b, popup, p.accent, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        "tab presets",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );

    let search = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let filtered = list.filtered_indices();
    put_text(
        b,
        search.x,
        search.y,
        search.width,
        &if list.search_focused {
            " / ".to_owned()
        } else if !list.query.is_empty() {
            format!(" / {}", list.query)
        } else {
            " / filter presets".to_owned()
        },
        Style::default()
            .fg(if list.search_focused {
                p.text
            } else {
                p.overlay0
            })
            .bg(p.panel_bg),
    );
    let count = match (filtered.len(), list.entries.len()) {
        (1, 1) => "1 preset".to_owned(),
        (shown, total) if shown == total => format!("{total} presets"),
        (shown, total) => format!("{shown}/{total} presets"),
    };
    let cursor = if list.search_focused {
        text_editor::render(
            b,
            Rect::new(
                search.x + 3,
                search.y,
                search.width.saturating_sub(4 + display_width(&count)),
                1,
            ),
            &list.query,
            Style::default().fg(p.text).bg(p.panel_bg),
        )
    } else {
        None
    };
    put_right_text(
        b,
        search,
        search.y,
        &count,
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        &"─".repeat(inner.width as usize),
        Style::default().fg(p.surface1).bg(p.panel_bg),
    );

    let body = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(6),
    );
    let selected = list.selected_index();
    let visible_count = (body.height / 2).max(1) as usize;
    let selected_position = filtered
        .iter()
        .position(|index| Some(*index) == selected)
        .unwrap_or(0);
    let start = selected_position
        .saturating_sub(visible_count.saturating_sub(1))
        .min(filtered.len().saturating_sub(visible_count));
    let mut row_hits = Vec::new();
    for (visible, entry_index) in filtered
        .iter()
        .copied()
        .skip(start)
        .take(visible_count)
        .enumerate()
    {
        let entry = &list.entries[entry_index];
        let rect = Rect::new(body.x, body.y + visible as u16 * 2, body.width, 2);
        row_hits.push((rect, entry_index));
        let is_selected = Some(entry_index) == selected;
        let style = if is_selected {
            Style::default().fg(contrast(p)).bg(p.accent)
        } else {
            Style::default().fg(p.text).bg(p.panel_bg)
        };
        b.set_style(rect, style);
        put_text(
            b,
            rect.x,
            rect.y,
            rect.width,
            &format!(" {}", entry.name),
            style.add_modifier(Modifier::BOLD),
        );
        let size = match (entry.error.is_some(), entry.panes) {
            (true, _) => "broken".to_owned(),
            (false, 1) => "1 pane".to_owned(),
            (false, panes) => format!("{panes} panes"),
        };
        put_right_text(b, rect, rect.y, &format!("{size} "), style);
        let (detail, detail_style) = match (&entry.error, &entry.label) {
            (Some(error), _) => (
                format!(" {error}"),
                if is_selected {
                    style
                } else {
                    Style::default().fg(p.red).bg(p.panel_bg)
                },
            ),
            (None, label) => (
                format!(" tab: {}", label.as_deref().unwrap_or("unnamed")),
                if is_selected {
                    style
                } else {
                    Style::default().fg(p.overlay0).bg(p.panel_bg)
                },
            ),
        };
        put_text(b, rect.x, rect.y + 1, rect.width, &detail, detail_style);
    }
    let muted = Style::default().fg(p.overlay0).bg(p.panel_bg);
    if list.entries.is_empty() && !list.loading {
        put_text(b, body.x, body.y, body.width, " no presets yet", muted);
        put_text(
            b,
            body.x,
            body.y + 1,
            body.width,
            " right-click a tab and choose Save tab as preset...",
            muted,
        );
    } else if filtered.is_empty() && !list.loading {
        put_text(b, body.x, body.y, body.width, " no matching presets", muted);
    }

    let status_y = inner.bottom() - 3;
    let (status, status_style) = if list.loading {
        (
            " loading…".to_owned(),
            Style::default().fg(p.accent).bg(p.panel_bg),
        )
    } else if list.busy {
        (
            " working…".to_owned(),
            Style::default().fg(p.accent).bg(p.panel_bg),
        )
    } else if let Some(name) = list.confirm_delete.as_deref() {
        (
            format!(" delete {name}? y deletes it · any other key keeps it"),
            Style::default()
                .fg(p.red)
                .bg(p.panel_bg)
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(error) = list.error.as_deref() {
        (
            format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        )
    } else if list.entries.is_empty() {
        (String::new(), muted)
    } else {
        (" e edit · r rename · d delete".to_owned(), muted)
    };
    put_text(b, inner.x, status_y, inner.width, &status, status_style);

    let buttons = row(inner, &[10, 11], 2, inner.height.saturating_sub(1));
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        " ↵ open ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc close ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        cancel: *cancel,
        // The rows and the search box reuse the worktree picker's click
        // targets; only one of the two lists is open at a time.
        worktree_search: search,
        worktree_rows: row_hits,
        cursor: cursor.filter(|_| !list.busy),
        ..OverlayRender::default()
    })
}
