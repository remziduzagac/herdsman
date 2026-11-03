//! Update modal, using the same panel and action-row helpers as other overlays.

use super::*;
use crate::client::shell::fork::UpdateDialog;
use ratatui::widgets::{Paragraph, Widget, Wrap};

pub(super) fn render_update_dialog(
    b: &mut Buffer,
    dialog: &UpdateDialog,
    p: &Palette,
) -> Option<OverlayRender> {
    let area = popup(b.area, 76, 26)?;
    let inner = panel(b, area, p.accent, p.panel_bg)?;
    let base = Style::default().fg(p.text).bg(p.panel_bg);
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        &format!("updates · {}", dialog.endpoint_label),
        base.add_modifier(Modifier::BOLD),
    );

    let actions = dialog.actions();
    let selected = dialog.selected.min(actions.len().saturating_sub(1));
    let capacity = usize::from(inner.height.saturating_sub(5).max(1));
    let visible = actions.len().min(capacity);
    let start = selected.saturating_sub(visible.saturating_sub(1));
    let status_y = inner.bottom().saturating_sub(visible as u16 + 1);
    let body = Rect::new(
        inner.x,
        inner.y.saturating_add(2),
        inner.width,
        status_y.saturating_sub(inner.y.saturating_add(3)),
    );
    let mut lines = dialog.details();
    if let Some(error) = dialog.error.as_ref() {
        lines.insert(0, format!("Error: {error}"));
    }
    let paragraph = Paragraph::new(lines.join("\n"))
        .style(base)
        .wrap(Wrap { trim: false });
    let max_scroll = paragraph
        .line_count(body.width.max(1))
        .saturating_sub(usize::from(body.height));
    if body.height > 0 {
        paragraph
            .scroll((
                u16::try_from(dialog.scroll.min(max_scroll)).unwrap_or(u16::MAX),
                0,
            ))
            .render(body, b);
    }
    if status_y > inner.y {
        put_text(
            b,
            inner.x,
            status_y,
            inner.width,
            dialog
                .progress_label()
                .unwrap_or(if dialog.error.is_some() {
                    " Action failed · PgUp/PgDn to read details"
                } else {
                    " ↑/↓ select · ↵ choose · esc later · wheel/PgUp/PgDn scroll"
                }),
            base.fg(if dialog.error.is_some() {
                p.red
            } else {
                p.overlay0
            }),
        );
    }
    let mut rows = Vec::new();
    let mut primary = Rect::default();
    let mut cancel = Rect::default();
    for (offset, (index, action)) in actions
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .enumerate()
    {
        let row = Rect::new(inner.x, status_y + 1 + offset as u16, inner.width, 1);
        let style = if index == selected {
            base.fg(contrast(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            base.bg(p.surface0)
        };
        button(b, row, &format!(" {}", action.label()), style);
        if index == selected {
            primary = row;
        }
        if action.label() == "Later" {
            cancel = row;
        }
        rows.push((row, index));
    }
    Some(OverlayRender {
        area,
        primary,
        cancel,
        worktree_rows: rows,
        help_max_scroll: max_scroll, // The shared modal scroll limit.
        ..Default::default()
    })
}
