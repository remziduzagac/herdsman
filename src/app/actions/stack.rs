//! Tests for how stacks affect agent completion and seen state, which
//! `actions.rs` handles in hooks marked `// fork: stacks`.
//!
//! herdsman's own module.

use super::tests::app_with_workspaces;
use super::*;

#[test]
fn hidden_stack_member_completion_stays_done_until_it_is_revealed() {
    let mut state = app_with_workspaces(&["active"]);
    state.active = Some(0);
    state.outer_terminal_focus = Some(true);
    let visible = state.workspaces[0].tabs[0].root_pane;
    let hidden = state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    state.ensure_test_terminals();
    let tab = &mut state.workspaces[0].tabs[0];
    tab.layout.focus_pane(visible);
    assert!(tab.layout.move_into_stack(visible, hidden));
    let terminal_id = tab.panes[&hidden].attached_terminal_id.clone();
    state.terminals.get_mut(&terminal_id).unwrap().state = AgentState::Working;
    assert!(!state.pane_is_in_active_tab(0, hidden));
    assert!(state.pane_is_in_active_tab(0, visible));

    state.handle_app_event(AppEvent::StateChanged {
        pane_id: hidden,
        agent: Some(Agent::Pi),
        state: AgentState::Idle,
        visible_blocker: false,
        visible_working: false,
        process_exited: false,
        observed_at: std::time::Instant::now(),
    });
    state.mark_active_tab_seen();
    state.workspaces[0].switch_tab(0);

    assert!(!state.workspaces[0].tabs[0].panes[&hidden].seen);
    assert!(state.focus_pane_in_workspace(0, hidden));
    assert!(state.mark_active_tab_seen());
    assert!(state.workspaces[0].tabs[0].panes[&hidden].seen);
    assert!(state.workspaces[0].tabs[0].layout.is_visible(hidden));
}
