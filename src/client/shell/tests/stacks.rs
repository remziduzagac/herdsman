use super::*;
use crate::api::schema::Method;
use crate::input::{KeybindAction, KeybindMatch};

fn endpoint_method(state: &mut ClientShellState, action: KeybindAction) -> Option<Method> {
    let mut input = ClientShellInput::default();
    state.record_binding(KeybindMatch::Action(action), &mut input);
    match &input.actions[..] {
        [ClientShellAction::Endpoint { request, .. }] => Some(request.method.clone()),
        _ => None,
    }
}

fn state() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state
}

#[test]
fn focus_stacked_asks_the_endpoint_to_resolve_the_focused_stack_member() {
    let mut state = state();

    let Some(Method::PaneFocusStacked(params)) =
        endpoint_method(&mut state, KeybindAction::FocusStacked(2))
    else {
        panic!("focus stacked should use the pane.focus_stacked endpoint method");
    };
    assert_eq!(params.pane_id.as_deref(), Some("pane_1"));
    assert_eq!(params.index, 2);
}

#[test]
fn stack_pane_stacks_a_focused_new_pane_onto_the_focused_pane() {
    let mut state = state();

    let Some(Method::PaneStack(params)) = endpoint_method(&mut state, KeybindAction::StackPane)
    else {
        panic!("stack pane should use the pane.stack endpoint method");
    };
    assert_eq!(params.target_pane_id.as_deref(), Some("pane_1"));
    assert!(params.focus);
}

#[test]
fn stack_actions_are_disabled_against_an_endpoint_without_them() {
    let mut state = state();
    state.set_endpoint_methods(Some(vec!["pane.split".into(), "pane.focus".into()]));

    assert!(endpoint_method(&mut state, KeybindAction::StackPane).is_none());
    assert!(endpoint_method(&mut state, KeybindAction::FocusStacked(0)).is_none());
}
