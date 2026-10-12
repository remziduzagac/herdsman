use super::*;
use crate::api::schema::{LayoutDimension, LayoutEqualizeParams, Method};

fn split(direction: PaneSurfaceSplitDirection) -> PaneSurfaceSplit {
    let rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 4,
        height: 2,
    };
    PaneSurfaceSplit {
        direction,
        pos: 2,
        area: rect,
        hit_rect: rect,
        path: Vec::new(),
    }
}

fn state(splits: &[PaneSurfaceSplitDirection]) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    let mut pane_surface = surface();
    pane_surface
        .splits
        .extend(splits.iter().map(|direction| split(*direction)));
    state.set_pane_surface(pane_surface);
    state
}

fn pane_menu(state: &mut ClientShellState) -> Vec<ClientContextMenuAction> {
    state.open_pane_context_menu("pane_1".into(), 0, 0);
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => {
            menu.items().iter().map(|item| item.action).collect()
        }
        _ => panic!("pane context menu"),
    }
}

fn activate(state: &mut ClientShellState, action: ClientContextMenuAction) -> Method {
    let index = pane_menu(state)
        .iter()
        .position(|item| *item == action)
        .expect("menu item");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    match &outcome.actions[..] {
        [ClientShellAction::Endpoint { request, .. }] => request.method.clone(),
        _ => panic!("one endpoint request"),
    }
}

#[test]
fn layout_items_follow_the_split_items() {
    use ClientContextMenuAction as Action;
    use PaneSurfaceSplitDirection::{Horizontal, Vertical};

    let items = pane_menu(&mut state(&[Horizontal, Vertical]));
    let split_down = items
        .iter()
        .position(|item| *item == Action::SplitDown)
        .expect("split down");
    assert_eq!(
        items[split_down + 1..split_down + 5],
        [
            Action::StackHere,
            Action::EqualWidths,
            Action::EqualHeights,
            Action::Zoom
        ]
    );
}

#[test]
fn equal_sizes_appear_only_along_a_split_direction() {
    use ClientContextMenuAction as Action;
    use PaneSurfaceSplitDirection::{Horizontal, Vertical};

    let offered = |splits: &[PaneSurfaceSplitDirection]| {
        let items = pane_menu(&mut state(splits));
        (
            items.contains(&Action::StackHere),
            items.contains(&Action::EqualWidths),
            items.contains(&Action::EqualHeights),
        )
    };

    assert_eq!(offered(&[]), (true, false, false));
    assert_eq!(offered(&[Horizontal]), (true, true, false));
    assert_eq!(offered(&[Vertical]), (true, false, true));
}

#[test]
fn an_endpoint_without_the_methods_gets_no_layout_items() {
    use ClientContextMenuAction as Action;
    use PaneSurfaceSplitDirection::{Horizontal, Vertical};

    let mut state = state(&[Horizontal, Vertical]);
    state.set_endpoint_methods(Some(vec!["pane.split".into(), "pane.zoom".into()]));
    let items = pane_menu(&mut state);

    assert!(items.contains(&Action::SplitRight));
    for action in [Action::StackHere, Action::EqualWidths, Action::EqualHeights] {
        assert!(!items.contains(&action), "{action:?} offered");
    }
}

#[test]
fn layout_items_act_on_the_clicked_pane() {
    use PaneSurfaceSplitDirection::{Horizontal, Vertical};

    let mut state = state(&[Horizontal, Vertical]);
    let Method::PaneStack(params) = activate(&mut state, ClientContextMenuAction::StackHere) else {
        panic!("stack here should use pane.stack");
    };
    assert_eq!(params.target_pane_id.as_deref(), Some("pane_1"));
    assert_eq!(params.workspace_id.as_deref(), Some("ws_1"));
    assert!(params.focus);

    for (action, dimension) in [
        (ClientContextMenuAction::EqualWidths, LayoutDimension::Width),
        (
            ClientContextMenuAction::EqualHeights,
            LayoutDimension::Height,
        ),
    ] {
        assert_eq!(
            activate(&mut state, action),
            Method::LayoutEqualize(LayoutEqualizeParams {
                tab_id: None,
                pane_id: Some("pane_1".into()),
                dimension,
            })
        );
    }
}
