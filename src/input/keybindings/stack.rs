//! Tests for the stack keybindings, which `keybindings.rs` resolves next to
//! the tab ones (marked `// fork: stacks`).
//!
//! herdsman's own module.

use super::*;
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn stack_bindings_mirror_tab_bindings_on_alt_without_conflicts() {
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

    // Tabs on plain keys, stack members on the same keys with alt.
    for (code, tab, stacked) in [
        (
            '3',
            KeybindAction::SwitchTab(2),
            KeybindAction::FocusStacked(2),
        ),
        ('n', KeybindAction::NextTab, KeybindAction::NextStacked),
        (
            'p',
            KeybindAction::PreviousTab,
            KeybindAction::PreviousStacked,
        ),
        ('c', KeybindAction::NewTab, KeybindAction::StackPane),
        ('x', KeybindAction::ClosePane, KeybindAction::ClosePane),
    ] {
        assert_eq!(
            action(code, KeyModifiers::empty()),
            Some(tab),
            "prefix+{code}"
        );
        assert_eq!(
            action(code, KeyModifiers::ALT),
            Some(stacked),
            "prefix+alt+{code}"
        );
    }
}

#[test]
fn focus_stacked_can_be_rebound_to_direct_keys() {
    let config: crate::config::Config =
        toml::from_str("[keys]\nfocus_stacked = \"alt+1..9\"\nstack_pane = []").unwrap();
    assert!(config.collect_diagnostics().is_empty());
    let keybinds = config.keybinds();

    assert!(matches!(
        resolve_direct_binding(
            &keybinds,
            &TerminalKey::new(KeyCode::Char('1'), KeyModifiers::ALT)
        ),
        Some(KeybindMatch::Action(KeybindAction::FocusStacked(0)))
    ));
    assert!(keybinds.stack_pane.bindings.is_empty());
}
