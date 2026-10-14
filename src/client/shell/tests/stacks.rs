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

/// One pane whose content starts two rows below its rect, where an endpoint
/// may draw a stack strip and the line separating it from the content.
fn surface_with_rows_above_content() -> PaneSurfaceFrame {
    let mut frame = surface();
    let buffer = Buffer::with_lines(["1 a 2 b ", "────────", "LIVE    ", "PANE    "]);
    frame.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[]);
    frame.panes[0].rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 8,
        height: 4,
    };
    frame.panes[0].inner_rect = SurfaceRect {
        x: 0,
        y: 2,
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

fn composed_with_rows_above_content(methods: Option<&[&str]>) -> (ClientShellState, Rect) {
    let mut state = state();
    if let Some(methods) = methods {
        state.set_endpoint_methods(Some(methods.iter().map(|m| (*m).to_owned()).collect()));
    }
    state.set_pane_surface(surface_with_rows_above_content());
    state.compose(106, 20).expect("composed frame");
    let inner = state.hits.panes[0].inner_rect;
    (state, inner)
}

#[test]
fn pressing_a_row_above_content_lets_the_endpoint_resolve_a_stack_member() {
    let (mut state, inner) = composed_with_rows_above_content(None);

    for offset in [1, 2] {
        assert!(matches!(
            press(&mut state, inner.x + 4, inner.y - offset),
            Some(Method::PaneFocusStackStrip(params))
                if params.pane_id == "pane_1"
                    && params.column == 4
                    && params.width == inner.width
                    && params.edge == crate::api::schema::PaneContentEdge::Top
                    && params.offset == offset
        ));
    }
    assert!(matches!(
        press(&mut state, inner.x + 1, inner.y),
        Some(Method::PaneFocus(target)) if target.pane_id == "pane_1"
    ));
}

#[test]
fn an_endpoint_without_strip_offsets_resolves_only_the_row_next_to_content() {
    let (mut state, inner) =
        composed_with_rows_above_content(Some(&["pane.focus", "pane.focus_stacked_at"]));

    assert!(matches!(
        press(&mut state, inner.x + 4, inner.y - 1),
        Some(Method::PaneFocusStackedAt(params))
            if params.column == 4 && params.edge == crate::api::schema::PaneContentEdge::Top
    ));
    assert!(matches!(
        press(&mut state, inner.x + 4, inner.y - 2),
        Some(Method::PaneFocus(target)) if target.pane_id == "pane_1"
    ));
}

#[test]
fn pressing_beside_content_is_a_plain_focus_without_the_stack_method() {
    let (mut state, inner) = composed_with_rows_above_content(Some(&["pane.focus"]));

    for offset in [1, 2] {
        let method = press(&mut state, inner.x + 4, inner.y - offset);
        assert!(matches!(method, Some(Method::PaneFocus(target)) if target.pane_id == "pane_1"));
    }
    assert!(state.visible_endpoint_notice.is_none());
}

fn open_navigator(state: &mut ClientShellState) -> Vec<ClientShellAction> {
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        KeybindMatch::Action(KeybindAction::OpenNavigator),
        &mut outcome,
    );
    outcome.actions
}

fn pane_row_labels(state: &ClientShellState) -> Vec<String> {
    let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_ref() else {
        panic!("navigator should be open");
    };
    render::client_navigator_rows(&state.endpoints, &state.active_endpoint_id, navigator)
        .into_iter()
        .filter(|row| matches!(row.target, ClientNavigatorTarget::Pane { .. }))
        .map(|row| row.label)
        .collect()
}

#[test]
fn navigator_marks_stacked_panes_once_the_endpoint_answers() {
    let mut projected = snapshot();
    let mut hidden = projected.panes[0].clone();
    hidden.pane_id = "pane_2".into();
    hidden.focused = false;
    projected.panes.push(hidden);
    let mut state = state();
    state.set_snapshot(Box::new(projected));

    let actions = open_navigator(&mut state);
    let request_id = actions
        .iter()
        .find_map(|action| match action {
            ClientShellAction::Endpoint { request, .. }
                if matches!(request.method, Method::PaneStacks(_)) =>
            {
                Some(request.id.clone())
            }
            _ => None,
        })
        .expect("the navigator asks for stacks when it opens");
    assert!(pane_row_labels(&state)
        .iter()
        .all(|label| !label.contains("stacked")));

    state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(crate::api::schema::ResponseResult::PaneStacks {
            stacks: vec![crate::api::schema::PaneStackInfo {
                workspace_id: "ws_1".into(),
                tab_id: "tab_1".into(),
                pane_ids: vec!["pane_1".into(), "pane_2".into()],
                visible_pane_id: "pane_1".into(),
            }],
        }),
    );

    let labels = pane_row_labels(&state);
    assert!(labels[0].ends_with("· stacked 1/2"), "{labels:?}");
    assert!(labels[1].ends_with("· stacked 2/2 · hidden"), "{labels:?}");
}

#[test]
fn navigator_does_not_ask_an_endpoint_without_pane_stacks() {
    let mut state = state();
    state.set_endpoint_methods(Some(vec!["pane.focus".into()]));

    let actions = open_navigator(&mut state);

    assert!(actions.iter().all(|action| !matches!(
        action,
        ClientShellAction::Endpoint { request, .. } if matches!(request.method, Method::PaneStacks(_))
    )));
    assert!(state.visible_endpoint_notice.is_none());
}

#[test]
fn stack_actions_are_disabled_against_an_endpoint_without_them() {
    let mut state = state();
    state.set_endpoint_methods(Some(vec!["pane.split".into(), "pane.focus".into()]));

    assert!(endpoint_method(&mut state, KeybindAction::StackPane).is_none());
    assert!(endpoint_method(&mut state, KeybindAction::FocusStacked(0)).is_none());
}

/// A one-tab workspace whose only slot is a stack: `pane_1` shows and
/// `pane_2` waits behind it. The endpoint lists a stack's hidden members as
/// panes of their tab, so the snapshot holds both.
fn one_slot_stack(confirm_close: bool) -> ClientShellState {
    let mut projected = snapshot();
    let mut hidden = projected.panes[0].clone();
    hidden.pane_id = "pane_2".into();
    hidden.focused = false;
    projected.panes.push(hidden);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.confirm_close = confirm_close;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn close_from_pane_menu(state: &mut ClientShellState) -> Option<Method> {
    state.open_pane_context_menu("pane_1".into(), 0, 0);
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("pane context menu");
    };
    let index = menu
        .items()
        .iter()
        .position(|item| item.action == ClientContextMenuAction::ClosePane)
        .expect("close item");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    match &outcome.actions[..] {
        [ClientShellAction::Endpoint { request, .. }] => Some(request.method.clone()),
        _ => None,
    }
}

#[test]
fn closing_a_stack_member_with_others_behind_it_closes_without_asking() {
    for confirm_close in [true, false] {
        let mut keyed = one_slot_stack(confirm_close);
        assert!(matches!(
            endpoint_method(&mut keyed, KeybindAction::ClosePane),
            Some(Method::PaneClose(target)) if target.pane_id == "pane_1"
        ));
        assert!(!matches!(
            keyed.overlay,
            Some(ClientShellOverlay::ConfirmClose(_))
        ));

        let mut clicked = one_slot_stack(confirm_close);
        assert!(matches!(
            close_from_pane_menu(&mut clicked),
            Some(Method::PaneClose(target)) if target.pane_id == "pane_1"
        ));
        assert!(!matches!(
            clicked.overlay,
            Some(ClientShellOverlay::ConfirmClose(_))
        ));
    }
}

/// Whether an endpoint method is the one a key should send.
type ExpectedMethod = Box<dyn Fn(&Method) -> bool>;

fn endpoint_methods(outcome: &ClientShellInput) -> Vec<Method> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => Some(request.method.clone()),
            _ => None,
        })
        .collect()
}

/// The bytes a terminal sends for the prefix, ctrl+b, then each of herdsman's
/// prefix keys, in the legacy encoding and in the kitty keyboard protocol, with
/// the bindings taken locally and from the endpoint's keybinding profile.
#[test]
fn herdsman_prefix_keys_reach_their_endpoint_methods_from_terminal_bytes() {
    use crate::api::schema::LayoutDimension;

    let config = Config::default();
    for source in [
        ClientShellKeybindingSource::Local,
        ClientShellKeybindingSource::Endpoint,
    ] {
        let mut state = ClientShellState::new(
            ClientShellConfig::from_config(&config).with_keybinding_source(source),
        );
        let mut projected = snapshot();
        projected.server_keybindings_toml = config.local_keybindings_profile_toml().ok();
        state.set_snapshot(Box::new(projected));
        state.set_pane_surface(surface());

        let equalize = |dimension| {
            move |method: &Method| {
                matches!(method, Method::LayoutEqualize(params)
                    if params.tab_id.as_deref() == Some("tab_1") && params.dimension == dimension)
            }
        };
        let stack = |method: &Method| {
            matches!(method, Method::PaneStack(params)
                if params.target_pane_id.as_deref() == Some("pane_1") && params.focus)
        };
        let stacked = |index: Option<usize>, step: Option<i32>| {
            move |method: &Method| {
                matches!(method, Method::PaneFocusStacked(params)
                    if params.pane_id.as_deref() == Some("pane_1")
                        && params.index == index
                        && params.step == step)
            }
        };
        let cases: Vec<(&[u8], ExpectedMethod)> = vec![
            (b"\x02|", Box::new(equalize(LayoutDimension::Width))),
            (b"\x02_", Box::new(equalize(LayoutDimension::Height))),
            (b"\x02\x1bc", Box::new(stack)),
            (b"\x02\x1bn", Box::new(stacked(None, Some(1)))),
            (b"\x02\x1bp", Box::new(stacked(None, Some(-1)))),
            (b"\x02\x1b1", Box::new(stacked(Some(0), None))),
            (b"\x02\x1b9", Box::new(stacked(Some(8), None))),
            (
                b"\x1b[98;5u\x1b[124;2u",
                Box::new(equalize(LayoutDimension::Width)),
            ),
            (
                b"\x1b[98;5u\x1b[95;2u",
                Box::new(equalize(LayoutDimension::Height)),
            ),
            (b"\x1b[98;5u\x1b[99;3u", Box::new(stack)),
            (b"\x1b[98;5u\x1b[51;3u", Box::new(stacked(Some(2), None))),
        ];
        for (bytes, expected) in cases {
            let outcome = state.handle_input_bytes(bytes);
            let methods = endpoint_methods(&outcome);
            assert!(
                matches!(methods.as_slice(), [method] if expected(method)),
                "{source:?} {:?}: {methods:?}",
                String::from_utf8_lossy(bytes)
            );
        }
    }
}
