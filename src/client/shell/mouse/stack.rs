//! Presses on a stack strip, which the endpoint resolves to a member.
//!
//! herdsman's own module. `mouse.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;

impl ClientShellState {
    /// Pressing a pane focuses it. A press on a row outside its content,
    /// within its columns, may land on a stack strip the endpoint drew there, so
    /// the endpoint resolves it when it can; otherwise it is a plain focus.
    /// Endpoints without `pane.focus_stack_strip` only draw a strip on the row
    /// next to the content.
    pub(super) fn pane_press_method(
        &self,
        hit: PaneHit,
        mouse: MouseEvent,
    ) -> crate::api::schema::Method {
        use crate::api::schema::{
            Method, PaneContentEdge, PaneFocusStackStripParams, PaneFocusStackedAtParams,
            PaneTarget,
        };

        let inner = hit.inner_rect;
        let content_end = inner.y.saturating_add(inner.height);
        let edge = if mouse.row < inner.y {
            Some((PaneContentEdge::Top, inner.y - mouse.row))
        } else if mouse.row >= content_end {
            Some((PaneContentEdge::Bottom, mouse.row - content_end + 1))
        } else {
            None
        };
        let beside_content = mouse.column >= inner.x && mouse.column - inner.x < inner.width;
        let Some((edge, offset)) = edge.filter(|_| beside_content && !hit.popup) else {
            return Method::PaneFocus(PaneTarget {
                pane_id: hit.pane_id,
            });
        };
        let column = mouse.column - inner.x;
        let strip = Method::PaneFocusStackStrip(PaneFocusStackStripParams {
            pane_id: hit.pane_id.clone(),
            column,
            width: inner.width,
            edge,
            offset,
        });
        let next_to_content = (offset == 1).then(|| {
            Method::PaneFocusStackedAt(PaneFocusStackedAtParams {
                pane_id: hit.pane_id.clone(),
                column,
                width: inner.width,
                edge,
            })
        });
        std::iter::once(strip)
            .chain(next_to_content)
            .find(|method| self.supports_endpoint_method(method))
            .unwrap_or(Method::PaneFocus(PaneTarget {
                pane_id: hit.pane_id,
            }))
    }
}
