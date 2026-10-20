use super::*;
use crate::api::schema::{
    Method, PresetLoadParams, PresetNameParams, PresetSaveParams, PresetSummary, ResponseResult,
};

fn shell() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 30).expect("initial shell");
    state
}

fn requests(outcome: &ClientShellInput) -> Vec<(String, Method)> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => {
                Some((request.id.clone(), request.method.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The one request `outcome` sent.
fn one(outcome: &ClientShellInput) -> (String, Method) {
    match requests(outcome).as_slice() {
        [request] => request.clone(),
        sent => panic!("expected one request, got {sent:?}"),
    }
}

fn summary(name: &str, label: Option<&str>, panes: usize, error: Option<&str>) -> PresetSummary {
    PresetSummary {
        name: name.into(),
        path: format!("/home/user/.config/herdsman/presets/{name}.toml"),
        label: label.map(str::to_string),
        panes,
        params: Vec::new(),
        error: error.map(str::to_string),
    }
}

/// `prefix+t` (Ctrl+b, then t), answered with a saved and a broken preset.
fn open_list(state: &mut ClientShellState) {
    let outcome = state.handle_input_bytes(b"\x02t");
    let (request_id, Method::PresetList(_)) = one(&outcome) else {
        panic!(
            "prefix+t should ask for the presets: {:?}",
            requests(&outcome)
        );
    };
    state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(ResponseResult::PresetList {
            presets: vec![
                summary(
                    "broken",
                    None,
                    0,
                    Some("root: a split needs both `first` and `second`"),
                ),
                summary("dev", Some("dev"), 3, None),
            ],
        }),
    );
}

fn screen(state: &mut ClientShellState) -> String {
    let frame = state.compose(106, 30).expect("composed frame");
    frame_rows(&frame).join("\n")
}

fn select(state: &mut ClientShellState, name: &str) {
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_mut() else {
        panic!("presets list");
    };
    list.selected = list
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("preset in the list");
}

#[test]
fn prefix_t_opens_the_list_and_draws_saved_and_broken_presets() {
    let mut state = shell();
    open_list(&mut state);

    let screen = screen(&mut state);
    assert!(screen.contains("tab presets"), "{screen}");
    assert!(screen.contains("dev"), "{screen}");
    assert!(screen.contains("3 panes"), "{screen}");
    assert!(screen.contains("tab: dev"), "{screen}");
    assert!(screen.contains("broken"), "{screen}");
    assert!(screen.contains("a split needs both"), "{screen}");
    assert!(screen.contains("e edit · r rename · d delete"), "{screen}");
}

#[test]
fn an_empty_list_says_how_to_save_a_preset() {
    let mut state = shell();
    let outcome = state.handle_input_bytes(b"\x02t");
    let request_id = requests(&outcome)[0].0.clone();
    state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(ResponseResult::PresetList {
            presets: Vec::new(),
        }),
    );
    let screen = screen(&mut state);
    assert!(screen.contains("no presets yet"), "{screen}");
    assert!(screen.contains("Save tab as preset..."), "{screen}");
}

#[test]
fn enter_opens_the_selected_preset_in_the_focused_workspace() {
    let mut state = shell();
    open_list(&mut state);
    select(&mut state, "dev");

    let outcome = state.handle_input_bytes(b"\r");
    let (request_id, Method::PresetLoad(params)) = one(&outcome) else {
        panic!("Enter should load: {:?}", requests(&outcome));
    };
    assert_eq!(
        params,
        PresetLoadParams {
            name: "dev".into(),
            workspace_id: Some("ws_1".into()),
            pane_id: None,
            focus: true,
            params: Default::default(),
        }
    );
    state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(ResponseResult::LayoutApply {
            layout: crate::api::schema::LayoutDescription {
                workspace_id: "ws_1".into(),
                tab_id: "tab_2".into(),
                zoomed: false,
                focused_pane_id: "pane_2".into(),
                root: crate::api::schema::LayoutNode::Pane {
                    pane: Default::default(),
                },
            },
        }),
    );
    assert!(
        state.overlay.is_none(),
        "the list closes once the tab is open"
    );
}

#[test]
fn a_broken_preset_does_not_open_and_says_how_to_fix_it() {
    let mut state = shell();
    open_list(&mut state);
    select(&mut state, "broken");

    let outcome = state.handle_input_bytes(b"\r");
    assert!(requests(&outcome).is_empty());
    assert!(screen(&mut state).contains("broken has a mistake; press e to fix it"));
}

#[test]
fn search_filters_the_rows() {
    let mut state = shell();
    open_list(&mut state);
    state.handle_input_bytes(b"/de");
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
        panic!("presets list");
    };
    assert_eq!(list.query.as_str(), "de");
    assert_eq!(
        list.selected_entry().map(|entry| entry.name.as_str()),
        Some("dev")
    );
    assert!(screen(&mut state).contains("1/2 presets"));
}

#[test]
fn d_asks_before_deleting() {
    let mut state = shell();
    open_list(&mut state);
    select(&mut state, "dev");

    let outcome = state.handle_input_bytes(b"d");
    assert!(requests(&outcome).is_empty(), "d alone deletes nothing");
    assert!(screen(&mut state).contains("delete dev? y deletes it · any other key keeps it"));

    let outcome = state.handle_input_bytes(b"n");
    assert!(requests(&outcome).is_empty(), "any other key keeps it");
    assert!(!screen(&mut state).contains("delete dev?"));

    state.handle_input_bytes(b"d");
    let outcome = state.handle_input_bytes(b"y");
    let (_, Method::PresetDelete(PresetNameParams { name })) = one(&outcome) else {
        panic!("y should delete: {:?}", requests(&outcome));
    };
    assert_eq!(name, "dev");
}

#[test]
fn e_edits_the_preset_in_a_popup() {
    let mut state = shell();
    open_list(&mut state);
    select(&mut state, "broken");

    let outcome = state.handle_input_bytes(b"e");
    let (_, Method::PresetEdit(PresetNameParams { name })) = one(&outcome) else {
        panic!("e should edit: {:?}", requests(&outcome));
    };
    assert_eq!(name, "broken");
    assert!(state.overlay.is_none(), "the list makes way for the popup");
    assert!(state.popup_pending);
}

#[test]
fn the_tab_menu_saves_the_tab_and_asks_before_replacing_one() {
    let mut state = shell();
    state.open_tab_context_menu("tab_1".into(), 0, 0);
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("tab menu");
    };
    let items = menu.items();
    let labels: Vec<_> = items.iter().map(|item| item.label).collect();
    assert!(labels.contains(&"Save tab as preset..."), "{labels:?}");
    assert!(labels.contains(&"New tab from preset..."), "{labels:?}");
    let index = items
        .iter()
        .position(|item| item.action == ClientContextMenuAction::SavePreset)
        .expect("save item");
    state.activate_context_menu_item(index, &mut ClientShellInput::default());
    assert!(screen(&mut state).contains("save tab as preset"));

    state.handle_input_bytes(b"dev");
    let outcome = state.handle_input_bytes(b"\r");
    let (request_id, Method::PresetSave(params)) = one(&outcome) else {
        panic!("Enter should save: {:?}", requests(&outcome));
    };
    assert_eq!(
        params,
        PresetSaveParams {
            name: "dev".into(),
            tab_id: Some("tab_1".into()),
            pane_id: None,
            overwrite: false,
        }
    );
    state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Err(ClientShellEndpointError {
            code: Some("preset_exists".into()),
            message: "preset dev already exists".into(),
        }),
    );
    assert!(screen(&mut state).contains("preset exists · ↵ replaces it"));
    let outcome = state.handle_input_bytes(b"\r");
    let (_, Method::PresetSave(params)) = one(&outcome) else {
        panic!("Enter should replace: {:?}", requests(&outcome));
    };
    assert!(params.overwrite);
    assert_eq!(params.name, "dev");
}

#[test]
fn the_tab_menu_opens_the_list_for_that_tabs_workspace() {
    let mut state = shell();
    state.open_tab_context_menu("tab_1".into(), 0, 0);
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("tab menu");
    };
    let index = menu
        .items()
        .iter()
        .position(|item| item.action == ClientContextMenuAction::NewTabFromPreset)
        .expect("new tab from preset item");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    assert!(requests(&outcome)
        .iter()
        .any(|(_, method)| matches!(method, Method::PresetList(_))));
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
        panic!("presets list");
    };
    assert_eq!(list.workspace_id.as_deref(), Some("ws_1"));
}

#[test]
fn tab_names_become_preset_names() {
    use super::super::presets::preset_name_from;

    assert_eq!(preset_name_from("Dev Tab"), "dev-tab");
    assert_eq!(preset_name_from("api_2"), "api_2");
    assert_eq!(preset_name_from("  -notes"), "notes");
    assert_eq!(preset_name_from(&"x".repeat(80)).len(), 64);
}

#[test]
fn a_preset_with_required_params_asks_for_each_value_then_opens() {
    use crate::api::schema::PresetParam;

    let mut state = shell();
    let outcome = state.handle_input_bytes(b"\x02t");
    let request_id = requests(&outcome)[0].0.clone();
    let param = |name: &str, required: bool, default: Option<&str>, description: Option<&str>| {
        PresetParam {
            name: name.into(),
            required,
            default: default.map(str::to_string),
            description: description.map(str::to_string),
        }
    };
    let mut pr = summary("pr", None, 2, None);
    pr.params = vec![
        param("base", false, Some("dev"), None),
        param("pr", true, None, Some("pull request number")),
        param("ticket", true, None, None),
    ];
    state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(ResponseResult::PresetList { presets: vec![pr] }),
    );

    // Enter asks for the first required value instead of loading.
    let outcome = state.handle_input_bytes(b"\r");
    assert!(requests(&outcome).is_empty(), "{:?}", requests(&outcome));
    let drawn = screen(&mut state);
    assert!(drawn.contains("pr · pr: pull request number"), "{drawn}");

    // An empty value is not accepted; the prompt stays.
    let outcome = state.handle_input_bytes(b"\r");
    assert!(requests(&outcome).is_empty());

    state.handle_input_bytes(b"42\r");
    let drawn = screen(&mut state);
    assert!(drawn.contains("pr · ticket"), "{drawn}");

    let outcome = state.handle_input_bytes(b"HS-7\r");
    let (_, Method::PresetLoad(params)) = one(&outcome) else {
        panic!("the last value should load: {:?}", requests(&outcome));
    };
    assert_eq!(params.name, "pr");
    assert_eq!(params.workspace_id.as_deref(), Some("ws_1"));
    // The default is the server's to fill in.
    assert_eq!(
        params.params,
        std::collections::BTreeMap::from([
            ("pr".to_string(), "42".to_string()),
            ("ticket".to_string(), "HS-7".to_string()),
        ])
    );
}
