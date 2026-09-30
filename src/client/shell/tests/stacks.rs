use super::*;
use crate::api::schema::Method;
use crate::input::{KeybindAction, KeybindMatch};
use crate::protocol::{PaneSurfaceFrame, SurfaceRect};
use ratatui::buffer::Buffer;

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
    assert_eq!((params.index, params.step), (Some(2), None));
}

#[test]
fn next_and_previous_stacked_ask_the_endpoint_to_step_through_members() {
    let mut state = state();

    for (action, step) in [
        (KeybindAction::NextStacked, 1),
        (KeybindAction::PreviousStacked, -1),
    ] {
        let Some(Method::PaneFocusStacked(params)) = endpoint_method(&mut state, action) else {
            panic!("{action:?} should use the pane.focus_stacked endpoint method");
        };
        assert_eq!(params.pane_id.as_deref(), Some("pane_1"));
        assert_eq!((params.index, params.step), (None, Some(step)));
    }
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

/// One pane whose content starts a row below its rect, where an endpoint may
/// draw a stack strip.
fn surface_with_a_row_above_content() -> PaneSurfaceFrame {
    let mut frame = surface();
    let buffer = Buffer::with_lines(["1 a 2 b ", "LIVE    ", "PANE    "]);
    frame.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[]);
    frame.panes[0].rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 8,
        height: 3,
    };
    frame.panes[0].inner_rect = SurfaceRect {
        x: 0,
        y: 1,
        width: 8,
        height: 2,
    };
    frame
}

fn press(state: &mut ClientShellState, column: u16, row: u16) -> Option<Method> {
    let outcome =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::empty(),
        })]);
    outcome.actions.iter().find_map(|action| match action {
        ClientShellAction::Endpoint { request, .. } => Some(request.method.clone()),
        _ => None,
    })
}

#[test]
fn pressing_the_row_above_content_lets_the_endpoint_resolve_a_stack_member() {
    let mut state = state();
    state.set_pane_surface(surface_with_a_row_above_content());
    state.compose(106, 20).expect("composed frame");
    let inner = state.hits.panes[0].inner_rect;

    assert!(matches!(
        press(&mut state, inner.x + 4, inner.y - 1),
        Some(Method::PaneFocusStackedAt(params))
            if params.pane_id == "pane_1"
                && params.column == 4
                && params.width == inner.width
                && params.edge == crate::api::schema::PaneContentEdge::Top
    ));
    assert!(matches!(
        press(&mut state, inner.x + 1, inner.y),
        Some(Method::PaneFocus(target)) if target.pane_id == "pane_1"
    ));
}

#[test]
fn pressing_beside_content_is_a_plain_focus_without_the_stack_method() {
    let mut state = state();
    state.set_endpoint_methods(Some(vec!["pane.focus".into()]));
    state.set_pane_surface(surface_with_a_row_above_content());
    state.compose(106, 20).expect("composed frame");
    let inner = state.hits.panes[0].inner_rect;

    let method = press(&mut state, inner.x + 4, inner.y - 1);

    assert!(matches!(method, Some(Method::PaneFocus(target)) if target.pane_id == "pane_1"));
    assert!(state.visible_endpoint_notice.is_none());
}

#[test]
fn stack_actions_are_disabled_against_an_endpoint_without_them() {
    let mut state = state();
    state.set_endpoint_methods(Some(vec!["pane.split".into(), "pane.focus".into()]));

    assert!(endpoint_method(&mut state, KeybindAction::StackPane).is_none());
    assert!(endpoint_method(&mut state, KeybindAction::FocusStacked(0)).is_none());
}
