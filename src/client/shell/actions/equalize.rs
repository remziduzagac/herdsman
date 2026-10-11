//! Equalize keybindings as endpoint methods.
//!
//! herdsman's own module. `actions.rs` reaches it through hooks marked
//! `// fork: equalize`.

use crate::api::schema::{LayoutDimension, LayoutEqualizeParams, Method};
use crate::input::KeybindAction;

/// The endpoint method for an equalize keybinding, against the tab this client
/// shows: with independent tab views it can differ from the server's active tab.
pub(super) fn keybind_method(action: KeybindAction, focused_tab: Option<String>) -> Option<Method> {
    let dimension = match action {
        KeybindAction::EqualizeWidths => LayoutDimension::Width,
        KeybindAction::EqualizeHeights => LayoutDimension::Height,
        _ => return None,
    };
    Some(Method::LayoutEqualize(LayoutEqualizeParams {
        tab_id: Some(focused_tab?),
        pane_id: None,
        dimension,
    }))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::input::{resolve_prefix_binding, KeybindMatch, TerminalKey};

    #[test]
    fn default_keys_draw_the_divider_they_even_out() {
        let config = crate::config::Config::default();
        assert!(config.collect_diagnostics().is_empty());
        let keybinds = config.keybinds();
        let action = |code, modifiers| match resolve_prefix_binding(
            &keybinds,
            &TerminalKey::new(KeyCode::Char(code), modifiers),
        ) {
            Some(KeybindMatch::Action(action)) => Some(action),
            _ => None,
        };

        for modifiers in [KeyModifiers::empty(), KeyModifiers::SHIFT] {
            assert_eq!(action('|', modifiers), Some(KeybindAction::EqualizeWidths));
            assert_eq!(action('_', modifiers), Some(KeybindAction::EqualizeHeights));
        }
    }

    #[test]
    fn equalize_targets_the_tab_this_client_shows() {
        let method = keybind_method(KeybindAction::EqualizeHeights, Some("w1:t2".into()));
        assert_eq!(
            method,
            Some(Method::LayoutEqualize(LayoutEqualizeParams {
                tab_id: Some("w1:t2".into()),
                pane_id: None,
                dimension: LayoutDimension::Height,
            }))
        );

        assert_eq!(keybind_method(KeybindAction::EqualizeWidths, None), None);
        assert_eq!(
            keybind_method(KeybindAction::Zoom, Some("w1:t2".into())),
            None
        );
    }
}
