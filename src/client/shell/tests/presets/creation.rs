mod fixes;

use super::*;
use crate::api::schema::PresetDefaultKind;

fn enabled(prompt: bool) -> ClientShellState {
    let mut state = shell();
    state.config.prompt_new_workspace_name = prompt;
    state.set_endpoint_methods(Some(
        [
            "preset.default",
            "preset.list",
            "preset.apply",
            "workspace.create",
            "worktree.list",
            "worktree.create",
            "worktree.open",
            "tab.focus",
        ]
        .map(str::to_owned)
        .to_vec(),
    ));
    state
}

fn begin(state: &mut ClientShellState, action: crate::input::KeybindAction) -> ClientShellInput {
    let mut outcome = ClientShellInput::default();
    state.record_binding(crate::input::KeybindMatch::Action(action), &mut outcome);
    outcome
}

fn answer(
    state: &mut ClientShellState,
    id: &str,
    result: Result<ResponseResult, ClientShellEndpointError>,
) -> ClientShellInput {
    let (repaint, actions) = state.handle_endpoint_result("boot-1", id, result);
    ClientShellInput {
        repaint,
        actions,
        ..Default::default()
    }
}

fn choices(
    state: &mut ClientShellState,
    default: &str,
    name: Option<&str>,
    entries: Vec<PresetSummary>,
) -> ClientShellInput {
    let listed = answer(
        state,
        default,
        Ok(ResponseResult::PresetDefault {
            name: name.map(str::to_owned),
        }),
    );
    let (id, Method::PresetList(_)) = one(&listed) else {
        panic!("expected preset.list");
    };
    answer(
        state,
        &id,
        Ok(ResponseResult::PresetList { presets: entries }),
    )
}

fn created(kind: &str, already_open: bool) -> ResponseResult {
    serde_json::from_value(serde_json::json!({
        "type": kind,
        "workspace": { "workspace_id": "ws_2", "number": 2, "label": "new", "focused": true, "pane_count": 1, "tab_count": 1, "active_tab_id": "new-tab", "agent_status": "idle" },
        "tab": { "tab_id": "new-tab", "workspace_id": "ws_2", "number": 1, "label": "1", "focused": true, "pane_count": 1, "agent_status": "idle" },
        "root_pane": { "pane_id": "new-pane", "terminal_id": "new-terminal", "workspace_id": "ws_2", "tab_id": "new-tab", "focused": true, "agent_status": "idle", "revision": 1 },
        "worktree": { "path": "/repo-feature", "branch": "feature", "is_bare": false, "is_detached": false, "is_prunable": false, "is_linked_worktree": true, "label": "feature" },
        "already_open": already_open,
    })).expect("creation result")
}

fn default_request(state: &mut ClientShellState, action: crate::input::KeybindAction) -> String {
    let mut outcome = begin(state, action);
    if matches!(
        action,
        crate::input::KeybindAction::NewWorktree | crate::input::KeybindAction::OpenWorktree
    ) {
        let (id, Method::WorktreeList(_)) = one(&outcome) else {
            panic!("prepare worktree");
        };
        outcome = answer(state, &id, Ok(worktree_list_result(None)));
    }
    let (id, Method::PresetDefault(params)) = one(&outcome) else {
        panic!("default lookup: {:?}", requests(&outcome));
    };
    assert_eq!(params.workspace_id.as_deref(), Some("ws_1"));
    assert_eq!(
        params.kind,
        if action == crate::input::KeybindAction::NewWorkspace {
            PresetDefaultKind::Workspace
        } else {
            PresetDefaultKind::Worktree
        }
    );
    id
}

fn mouse(state: &mut ClientShellState, rect: Rect) -> ClientShellInput {
    let mut outcome = ClientShellInput::default();
    state.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::empty(),
        },
        &mut outcome,
    );
    outcome
}

#[test]
fn older_servers_keep_the_existing_creation_path_without_a_picker() {
    let mut state = shell();
    state.config.prompt_new_workspace_name = false;
    let outcome = begin(&mut state, crate::input::KeybindAction::NewWorkspace);
    assert!(matches!(one(&outcome).1, Method::WorkspaceCreate(_)));
    assert!(state.overlay.is_none());
    state.set_endpoint_methods(Some(vec!["workspace.create".into(), "preset.list".into()]));
    let outcome = begin(&mut state, crate::input::KeybindAction::NewWorkspace);
    assert!(matches!(one(&outcome).1, Method::WorkspaceCreate(_)));
    assert!(state.visible_endpoint_notice.is_none());
}

#[test]
fn unknown_default_method_falls_back_without_a_notice_or_preset() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    let outcome = answer(
        &mut state,
        &id,
        Err(ClientShellEndpointError {
            code: Some("unknown_method".into()),
            message: "old server".into(),
        }),
    );
    assert!(matches!(one(&outcome).1, Method::WorkspaceCreate(_)));
    assert!(state.overlay.is_none());
    assert!(state.visible_endpoint_notice.is_none());
}

#[test]
fn default_is_preselected_and_broken_presets_are_omitted() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    let result = choices(
        &mut state,
        &id,
        Some("dev"),
        vec![
            summary("broken", None, 0, Some("bad")),
            summary("dev", Some("development"), 2, None),
        ],
    );
    assert!(result.actions.is_empty());
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
        panic!("picker");
    };
    assert_eq!(
        list.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["None", "dev"]
    );
    assert_eq!(
        list.selected_entry().map(|entry| entry.name.as_str()),
        Some("dev")
    );
    assert!(screen(&mut state).contains("None starts a plain shell"));
    let create = state.handle_input_bytes(b"\r");
    let (id, Method::WorkspaceCreate(_)) = one(&create) else {
        panic!("create workspace");
    };
    let applied = answer(&mut state, &id, Ok(created("workspace_created", false)));
    let (_, Method::PresetApply(params)) = one(&applied) else {
        panic!("apply preset");
    };
    assert_eq!(params.name, "dev");
    assert_eq!(params.tab_id, "new-tab");
    assert!(params.focus && params.params.is_empty());
}

#[test]
fn missing_or_broken_default_selects_none() {
    for default in ["missing", "broken"] {
        let mut state = enabled(false);
        let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
        choices(
            &mut state,
            &id,
            Some(default),
            vec![
                summary("broken", None, 0, Some("bad")),
                summary("dev", None, 1, None),
            ],
        );
        let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
            panic!("picker");
        };
        assert_eq!(
            list.selected_entry().map(|entry| entry.name.as_str()),
            Some("None")
        );
        let create = state.handle_input_bytes(b"\r");
        let (id, Method::WorkspaceCreate(_)) = one(&create) else {
            panic!("create");
        };
        assert!(
            answer(&mut state, &id, Ok(created("workspace_created", false)))
                .actions
                .is_empty()
        );
    }
}

#[test]
fn no_valid_presets_creates_without_a_picker() {
    for entries in [Vec::new(), vec![summary("broken", None, 0, Some("bad"))]] {
        let mut state = enabled(false);
        let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
        let outcome = choices(&mut state, &id, Some("broken"), entries);
        assert!(matches!(one(&outcome).1, Method::WorkspaceCreate(_)));
        assert!(state.overlay.is_none());
    }
}

#[test]
fn named_workspace_keeps_its_name_while_choosing_a_preset() {
    let mut state = enabled(true);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    state.handle_input_bytes(b"my workspace");
    assert!(screen(&mut state).contains("preset: dev"));
    state.handle_input_bytes(b"\t");
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::Presets(_))
    ));
    state.handle_input_bytes(b"\x1b");
    assert!(matches!(state.overlay, Some(ClientShellOverlay::Rename(_))));
    let create = state.handle_input_bytes(b"\r");
    let (id, Method::WorkspaceCreate(params)) = one(&create) else {
        panic!("named workspace");
    };
    assert_eq!(params.label.as_deref(), Some("my workspace"));
    let result = answer(&mut state, &id, Ok(created("workspace_created", false)));
    assert!(matches!(one(&result).1, Method::PresetApply(_)));
}

#[test]
fn cancelling_a_lookup_cannot_create_or_reopen_a_picker() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    let listed = answer(
        &mut state,
        &id,
        Ok(ResponseResult::PresetDefault {
            name: Some("old".into()),
        }),
    );
    let old_list = one(&listed).0;
    state.handle_input_bytes(b"\x1b");
    let next = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    let ignored = answer(
        &mut state,
        &old_list,
        Ok(ResponseResult::PresetList {
            presets: vec![summary("old", None, 1, None)],
        }),
    );
    assert!(ignored.actions.is_empty());
    choices(
        &mut state,
        &next,
        Some("dev"),
        vec![summary("dev", None, 1, None)],
    );
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
        panic!("current picker");
    };
    assert_eq!(
        list.selected_entry().map(|entry| entry.name.as_str()),
        Some("dev")
    );
}

#[test]
fn cancelling_an_endpoint_request_does_not_submit_a_workspace() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    assert!(state.cancel_endpoint_request(&id));
    assert!(state.overlay.is_none());
    assert!(state.fork.creation.is_none());
}

#[test]
fn worktree_creation_applies_to_the_returned_tab_after_success() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorktree);
    assert!(
        state.handle_input_bytes(b"\r").actions.is_empty(),
        "wait for the default"
    );
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    assert!(screen(&mut state).contains("preset: dev"));
    let create = state.handle_input_bytes(b"\r");
    let (id, Method::WorktreeCreate(_)) = one(&create) else {
        panic!("create worktree");
    };
    let result = answer(&mut state, &id, Ok(created("worktree_created", false)));
    let (_, Method::PresetApply(params)) = one(&result) else {
        panic!("apply");
    };
    assert_eq!(params.tab_id, "new-tab");
    assert_eq!(params.name, "dev");
}

#[test]
fn worktree_preset_choice_is_clickable_and_can_select_none() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorktree);
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    screen(&mut state);
    let field = state.hits.fork.creation_preset;
    assert!(field.width > 0);
    mouse(&mut state, field);
    screen(&mut state);
    let row = state.hits.worktree_rows[0].0;
    mouse(&mut state, row);
    let choose = state.hits.overlay_primary;
    mouse(&mut state, choose);
    assert!(screen(&mut state).contains("preset: None"));
    let create = state.handle_input_bytes(b"\r");
    let id = one(&create).0;
    let result = answer(&mut state, &id, Ok(created("worktree_created", false)));
    assert!(
        matches!(one(&result).1, Method::TabFocus(_)),
        "keep the ordinary focus behavior"
    );
}

#[test]
fn open_worktree_applies_only_when_it_was_not_already_open() {
    for already_open in [false, true] {
        let mut state = enabled(false);
        let id = default_request(&mut state, crate::input::KeybindAction::OpenWorktree);
        choices(
            &mut state,
            &id,
            Some("dev"),
            vec![summary("dev", None, 2, None)],
        );
        let open = state.handle_input_bytes(b"\r");
        let (id, Method::WorktreeOpen(_)) = one(&open) else {
            panic!("open worktree");
        };
        let result = answer(
            &mut state,
            &id,
            Ok(created("worktree_opened", already_open)),
        );
        if already_open {
            assert!(result.actions.is_empty());
        } else {
            assert!(
                matches!(one(&result).1, Method::PresetApply(params) if params.tab_id == "new-tab")
            );
        }
    }
}

#[test]
fn failed_worktree_creation_keeps_the_preset_for_retry() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorktree);
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    let create = state.handle_input_bytes(b"\r");
    let id = one(&create).0;
    let result = answer(
        &mut state,
        &id,
        Err(ClientShellEndpointError {
            code: Some("branch_exists".into()),
            message: "branch exists".into(),
        }),
    );
    assert!(result.actions.is_empty());
    assert!(screen(&mut state).contains("branch exists"));
    let retried = state.handle_input_bytes(b"\r");
    let result = answer(
        &mut state,
        &one(&retried).0,
        Ok(created("worktree_created", false)),
    );
    assert!(matches!(one(&result).1, Method::PresetApply(params) if params.name == "dev"));
}

#[test]
fn worktree_rows_wait_for_the_default_before_submitting() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::OpenWorktree);
    screen(&mut state);
    let row = state.hits.worktree_rows[0].0;
    assert!(mouse(&mut state, row).actions.is_empty());
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    screen(&mut state);
    let row = state.hits.worktree_rows[0].0;
    assert!(matches!(
        one(&mouse(&mut state, row)).1,
        Method::WorktreeOpen(_)
    ));
}

#[test]
fn keyboard_can_choose_none_in_the_standalone_picker() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    state.handle_input_bytes(b"\x1b[A");
    let create = state.handle_input_bytes(b"\r");
    let id = one(&create).0;
    let result = answer(&mut state, &id, Ok(created("workspace_created", false)));
    assert!(result.actions.is_empty());
}

#[test]
fn apply_failure_leaves_the_new_workspace_available_and_reports_the_error() {
    let mut state = enabled(false);
    let id = default_request(&mut state, crate::input::KeybindAction::NewWorkspace);
    choices(
        &mut state,
        &id,
        Some("dev"),
        vec![summary("dev", None, 2, None)],
    );
    let create = state.handle_input_bytes(b"\r");
    let applied = answer(
        &mut state,
        &one(&create).0,
        Ok(created("workspace_created", false)),
    );
    let result = answer(
        &mut state,
        &one(&applied).0,
        Err(ClientShellEndpointError {
            code: Some("missing_param".into()),
            message: "preset needs a parameter".into(),
        }),
    );
    assert!(result.actions.is_empty());
    assert!(state.overlay.is_none());
    assert!(state.snapshot.is_some());
    assert!(state
        .visible_endpoint_notice
        .as_ref()
        .is_some_and(|notice| notice.body.contains("needs a parameter")));
}
