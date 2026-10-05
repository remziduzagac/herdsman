//! Stack keybindings as endpoint methods.
//!
//! herdsman's own module. `actions.rs` reaches it through hooks marked
//! `// fork: stacks`.

use crate::api::schema::{Method, PaneFocusStackedParams, PaneStackParams};
use crate::input::KeybindAction;

/// The endpoint method for a stack keybinding, against the focused pane.
pub(super) fn keybind_method(
    action: KeybindAction,
    focused_workspace: String,
    focused_pane: Option<String>,
) -> Option<Method> {
    match action {
        KeybindAction::StackPane => Some(Method::PaneStack(PaneStackParams {
            workspace_id: Some(focused_workspace),
            target_pane_id: focused_pane,
            focus: true,
            ..Default::default()
        })),
        // Stack membership is server layout, so the endpoint resolves the index.
        KeybindAction::FocusStacked(index) => {
            Some(Method::PaneFocusStacked(PaneFocusStackedParams {
                pane_id: focused_pane,
                index: Some(index),
                step: None,
            }))
        }
        KeybindAction::NextStacked | KeybindAction::PreviousStacked => {
            Some(Method::PaneFocusStacked(PaneFocusStackedParams {
                pane_id: focused_pane,
                index: None,
                step: Some(if action == KeybindAction::NextStacked {
                    1
                } else {
                    -1
                }),
            }))
        }
        _ => None,
    }
}
