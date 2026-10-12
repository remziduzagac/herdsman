//! herdsman's layout items in a pane's right-click menu: stack a new pane in
//! the clicked pane's slot, and give the tab's columns equal widths or its
//! rows equal heights.
//!
//! herdsman's own module. `context_menu.rs` and `state.rs` reach it through
//! hooks marked `// fork: pane menu`.

use super::{ClientContextMenuAction, ClientContextMenuItem, ClientShellInput, ClientShellState};
use crate::api::schema::{LayoutDimension, LayoutEqualizeParams, Method, PaneStackParams};
use crate::protocol::PaneSurfaceSplitDirection;

/// Which layout items a pane menu shows, decided when the menu opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LayoutItems {
    stack: bool,
    equal_widths: bool,
    equal_heights: bool,
}

impl LayoutItems {
    /// Add the items after the split items, so the layout actions sit together.
    pub(super) fn add_to(self, items: &mut Vec<ClientContextMenuItem>) {
        use ClientContextMenuAction as Action;

        let at = items
            .iter()
            .position(|item| item.action == Action::SplitDown)
            .map_or(items.len(), |index| index + 1);
        let added = [
            (self.stack, "Stack here", Action::StackHere),
            (self.equal_widths, "Equal widths", Action::EqualWidths),
            (self.equal_heights, "Equal heights", Action::EqualHeights),
        ]
        .into_iter()
        .filter(|(shown, ..)| *shown)
        .map(|(_, label, action)| ClientContextMenuItem { label, action });
        items.splice(at..at, added);
    }
}

impl ClientShellState {
    /// The layout items for a pane menu: only methods the endpoint offers, and
    /// equal sizes only along a direction the shown tab is split in.
    pub(super) fn pane_menu_layout_items(&self) -> LayoutItems {
        let split_in = |direction| {
            self.pane_surface.as_ref().is_some_and(|surface| {
                surface
                    .splits
                    .iter()
                    .any(|split| split.direction == direction)
            })
        };
        let equalize =
            self.supports_endpoint_method(&equalize_method(String::new(), LayoutDimension::Width));
        LayoutItems {
            stack: self.supports_endpoint_method(&Method::PaneStack(PaneStackParams::default())),
            equal_widths: equalize && split_in(PaneSurfaceSplitDirection::Horizontal),
            equal_heights: equalize && split_in(PaneSurfaceSplitDirection::Vertical),
        }
    }

    pub(super) fn activate_pane_layout_action(
        &mut self,
        pane_id: String,
        workspace_id: String,
        action: ClientContextMenuAction,
        outcome: &mut ClientShellInput,
    ) {
        let method = match action {
            ClientContextMenuAction::StackHere => Method::PaneStack(PaneStackParams {
                workspace_id: Some(workspace_id),
                target_pane_id: Some(pane_id),
                focus: true,
                ..Default::default()
            }),
            ClientContextMenuAction::EqualWidths => {
                equalize_method(pane_id, LayoutDimension::Width)
            }
            ClientContextMenuAction::EqualHeights => {
                equalize_method(pane_id, LayoutDimension::Height)
            }
            _ => return,
        };
        self.push_endpoint_method(method, outcome);
    }
}

/// `layout.equalize` for the tab holding `pane_id`.
fn equalize_method(pane_id: String, dimension: LayoutDimension) -> Method {
    Method::LayoutEqualize(LayoutEqualizeParams {
        tab_id: None,
        pane_id: Some(pane_id),
        dimension,
    })
}
