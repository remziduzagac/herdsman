use super::*;
use crate::api::schema::{PresetParam, WorkspaceCreateParams};
use crate::input::KeybindAction;

fn ready(action: KeybindAction, prompt: bool, preset: Option<&str>) -> ClientShellState {
    let mut state = enabled(prompt);
    let id = default_request(&mut state, action);
    choices(&mut state, &id, preset, vec![summary("dev", None, 2, None)]);
    state
}

fn plain_workspace() -> Method {
    Method::WorkspaceCreate(WorkspaceCreateParams {
        source_workspace_id: Some("ws_1".into()),
        cwd: Some("/repo".into()),
        focus: true,
        label: None,
        env: Default::default(),
    })
}

#[test]
fn required_parameters_without_defaults_are_excluded_from_creation() {
    let mut needs_value = summary("needs-value", None, 1, None);
    needs_value.params.push(PresetParam {
        name: "task".into(),
        required: true,
        default: None,
        description: None,
    });
    let mut has_default = needs_value.clone();
    has_default.name = "has-default".into();
    has_default.params[0].required = false;
    has_default.params[0].default = Some("work".into());
    let mut state = enabled(false);
    let id = default_request(&mut state, KeybindAction::NewWorkspace);
    choices(
        &mut state,
        &id,
        Some("needs-value"),
        vec![needs_value.clone(), has_default],
    );
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
        panic!("creation picker");
    };
    assert_eq!(
        list.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["None", "has-default"]
    );
    assert_eq!(
        list.selected_entry().map(|entry| entry.name.as_str()),
        Some("None")
    );

    let mut state = enabled(false);
    let id = default_request(&mut state, KeybindAction::NewWorkspace);
    let outcome = choices(&mut state, &id, Some("needs-value"), vec![needs_value]);
    assert!(matches!(one(&outcome).1, Method::WorkspaceCreate(_)));
    assert!(state.overlay.is_none());
}

#[test]
fn workspace_default_and_create_use_the_projected_target_folder() {
    for prompt in [false, true] {
        let mut state = enabled(prompt);
        state.snapshot.as_mut().expect("snapshot").workspaces[0].new_workspace_cwd =
            "/other-repository".into();
        let lookup = begin(&mut state, KeybindAction::NewWorkspace);
        let (id, Method::PresetDefault(params)) = one(&lookup) else {
            panic!("default lookup");
        };
        assert_eq!(params.workspace_id.as_deref(), Some("ws_1"));
        assert_eq!(params.cwd.as_deref(), Some("/other-repository"));
        let mut create = choices(&mut state, &id, None, Vec::new());
        if prompt {
            create = state.handle_input_bytes(b"\r");
        }
        let (_, Method::WorkspaceCreate(params)) = one(&create) else {
            panic!("workspace create");
        };
        assert_eq!(params.cwd.as_deref(), Some("/other-repository"));
    }
}

#[test]
fn existing_worktree_uses_its_path_and_new_worktree_uses_the_source() {
    for (action, cwd) in [
        (KeybindAction::NewWorktree, None),
        (KeybindAction::OpenWorktree, Some("/repo-feature")),
    ] {
        let mut state = enabled(false);
        let prepare = begin(&mut state, action);
        let (id, Method::WorktreeList(_)) = one(&prepare) else {
            panic!("worktree list");
        };
        let lookup = answer(&mut state, &id, Ok(worktree_list_result(None)));
        let (_, Method::PresetDefault(params)) = one(&lookup) else {
            panic!("default lookup");
        };
        assert_eq!(params.workspace_id.as_deref(), Some("ws_1"));
        assert_eq!(params.kind, PresetDefaultKind::Worktree);
        assert_eq!(params.cwd.as_deref(), cwd);
    }
}

#[test]
fn discovery_errors_keep_plain_workspace_creation_available() {
    for prompt in [false, true] {
        for fail_list in [false, true] {
            let mut state = enabled(prompt);
            let mut id = default_request(&mut state, KeybindAction::NewWorkspace);
            if fail_list {
                let listed = answer(
                    &mut state,
                    &id,
                    Ok(ResponseResult::PresetDefault {
                        name: Some("dev".into()),
                    }),
                );
                id = one(&listed).0;
            }
            let mut create = answer(
                &mut state,
                &id,
                Err(ClientShellEndpointError {
                    code: Some("permission_denied".into()),
                    message: "presets denied".into(),
                }),
            );
            if prompt {
                assert!(matches!(state.overlay, Some(ClientShellOverlay::Rename(_))));
                assert!(create.actions.is_empty());
                create = state.handle_input_bytes(b"\r");
            }
            let (id, Method::WorkspaceCreate(_)) = one(&create) else {
                panic!("plain workspace fallback");
            };
            assert!(
                answer(&mut state, &id, Ok(created("workspace_created", false)))
                    .actions
                    .is_empty()
            );
            assert!(state.fork.creation.is_none());
        }
    }
}

#[test]
fn cancelling_the_list_request_does_not_trigger_the_error_fallback() {
    let mut state = enabled(false);
    let id = default_request(&mut state, KeybindAction::NewWorkspace);
    let listed = answer(
        &mut state,
        &id,
        Ok(ResponseResult::PresetDefault { name: None }),
    );
    assert!(state.cancel_endpoint_request(&one(&listed).0));
    assert!(state.overlay.is_none());
    assert!(state.fork.creation.is_none());
}

#[test]
fn ordinary_list_responses_cannot_replace_a_creation_picker() {
    for failure in [false, true] {
        let mut state = enabled(false);
        let mut opened = ClientShellInput::default();
        state.open_presets_list(None, &mut opened);
        let ordinary = one(&opened).0;
        state.handle_input_bytes(b"\x1b");
        let id = default_request(&mut state, KeybindAction::NewWorkspace);
        choices(
            &mut state,
            &id,
            Some("dev"),
            vec![summary("dev", None, 2, None)],
        );
        let result = if failure {
            Err(ClientShellEndpointError {
                code: None,
                message: "old list failed".into(),
            })
        } else {
            Ok(ResponseResult::PresetList {
                presets: vec![summary("old", None, 1, None)],
            })
        };
        assert!(answer(&mut state, &ordinary, result).actions.is_empty());
        let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
            panic!("creation picker");
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
        assert!(list.error.is_none());
        assert!(!list.loading);
    }
}

#[test]
fn workspace_create_errors_use_the_standard_endpoint_notice() {
    for code in [
        "permission_denied",
        "endpoint_timeout",
        "server_unavailable",
        "endpoint_cancelled",
    ] {
        let mut baseline = enabled(false);
        let mut sent = ClientShellInput::default();
        baseline.push_endpoint_method(plain_workspace(), &mut sent);
        let baseline_id = one(&sent).0;
        answer(
            &mut baseline,
            &baseline_id,
            Err(ClientShellEndpointError {
                code: Some(code.into()),
                message: "create failed".into(),
            }),
        );

        let mut state = ready(KeybindAction::NewWorkspace, true, Some("dev"));
        let create = state.handle_input_bytes(b"\r");
        let result = answer(
            &mut state,
            &one(&create).0,
            Err(ClientShellEndpointError {
                code: Some(code.into()),
                message: "create failed".into(),
            }),
        );
        assert!(result.actions.is_empty());
        let expected = baseline
            .visible_endpoint_notice
            .as_ref()
            .expect("baseline notice");
        let actual = state
            .visible_endpoint_notice
            .as_ref()
            .expect("create notice");
        assert_eq!(actual.key, expected.key);
        assert_eq!(actual.title, expected.title);
        assert_eq!(actual.body, expected.body);
        assert!(state.fork.creation.is_none());
    }
}

#[test]
fn an_older_workspace_result_does_not_clear_a_newer_lookup() {
    let mut state = ready(KeybindAction::NewWorkspace, false, Some("dev"));
    let create = state.handle_input_bytes(b"\r");
    let old = one(&create).0;
    let next = default_request(&mut state, KeybindAction::NewWorkspace);
    let applied = answer(&mut state, &old, Ok(created("workspace_created", false)));
    assert!(matches!(one(&applied).1, Method::PresetApply(params) if params.name == "dev"));
    assert!(state.fork.creation.is_some());
    choices(
        &mut state,
        &next,
        Some("new"),
        vec![summary("new", None, 1, None)],
    );
    let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
        panic!("newer picker");
    };
    assert_eq!(
        list.selected_entry().map(|entry| entry.name.as_str()),
        Some("new")
    );
}

#[test]
fn late_worktree_results_do_not_close_a_newer_workspace_picker() {
    for already_open in [false, true] {
        let mut state = ready(KeybindAction::OpenWorktree, false, Some("dev"));
        let open = state.handle_input_bytes(b"\r");
        let old = one(&open).0;
        let next = default_request(&mut state, KeybindAction::NewWorkspace);
        let result = answer(
            &mut state,
            &old,
            Ok(created("worktree_opened", already_open)),
        );
        if already_open {
            assert!(result.actions.is_empty());
        } else {
            assert!(matches!(one(&result).1, Method::PresetApply(_)));
        }
        assert!(state.fork.creation.is_some());
        assert!(matches!(
            state.overlay,
            Some(ClientShellOverlay::Presets(_))
        ));
        choices(
            &mut state,
            &next,
            Some("dev"),
            vec![summary("dev", None, 2, None)],
        );
        assert!(screen(&mut state).contains("dev"));
    }
}

#[test]
fn enter_with_no_matching_preset_keeps_the_picker_open() {
    for prompt in [false, true] {
        let mut state = ready(KeybindAction::NewWorkspace, prompt, Some("dev"));
        if prompt {
            state.handle_input_bytes(b"\t");
        }
        state.handle_input_bytes(b"/no-matching-preset");
        let Some(ClientShellOverlay::Presets(list)) = state.overlay.as_ref() else {
            panic!("filtered picker");
        };
        assert!(list.selected_index().is_none());
        assert!(state.handle_input_bytes(b"\r").actions.is_empty());
        assert!(matches!(
            state.overlay,
            Some(ClientShellOverlay::Presets(_))
        ));
        assert!(state.fork.creation.is_some());
        if prompt {
            state.handle_input_bytes(b"\x1b");
            let create = state.handle_input_bytes(b"\r");
            let applied = answer(
                &mut state,
                &one(&create).0,
                Ok(created("workspace_created", false)),
            );
            assert!(matches!(one(&applied).1, Method::PresetApply(params) if params.name == "dev"));
        }
    }
}

#[test]
fn preset_rows_use_blank_rows_in_each_native_form() {
    for action in [
        KeybindAction::NewWorkspace,
        KeybindAction::NewWorktree,
        KeybindAction::OpenWorktree,
    ] {
        let state = ready(action, true, Some("dev"));
        for size in [(106, 30), (80, 24)] {
            let mut native = Buffer::empty(Rect::new(0, 0, size.0, size.1));
            let rendered = render::render_client_overlay(
                &mut native,
                state.overlay.as_ref().expect("form"),
                state.snapshot.as_deref().expect("snapshot"),
                &state.endpoints,
                &state.active_endpoint_id,
                &state.config.keybinds,
                &state.config.palette,
            )
            .expect("rendered form");
            let mut decorated = native.clone();
            let row = state.render_creation_preset(&mut decorated, rendered.area);
            assert!(row.width > 0, "{action:?} {size:?}");
            let original = (row.x..row.right())
                .map(|x| native[(x, row.y)].symbol())
                .collect::<String>();
            assert!(
                original.chars().all(|ch| ch == ' '),
                "{action:?} {size:?}: preset row overlaps {original:?}"
            );
            let label = (row.x..row.right())
                .map(|x| decorated[(x, row.y)].symbol())
                .collect::<String>();
            assert!(label.contains("preset: dev"));
        }
    }
}

#[test]
fn cancelling_each_form_clears_its_preset_selection() {
    for action in [
        KeybindAction::NewWorkspace,
        KeybindAction::NewWorktree,
        KeybindAction::OpenWorktree,
    ] {
        for click in [false, true] {
            let mut state = ready(action, true, Some("dev"));
            let outcome = if click {
                screen(&mut state);
                let cancel = state.hits.overlay_cancel;
                assert!(cancel.width > 0);
                mouse(&mut state, cancel)
            } else {
                state.handle_input_bytes(b"\x1b")
            };
            assert!(outcome.actions.is_empty());
            assert!(state.overlay.is_none(), "{action:?} click={click}");
            assert!(state.fork.creation.is_none(), "{action:?} click={click}");
        }
    }
}

#[test]
fn already_open_clears_selection_even_when_none_was_chosen() {
    for preset in [None, Some("dev")] {
        let mut state = ready(KeybindAction::OpenWorktree, false, preset);
        let open = state.handle_input_bytes(b"\r");
        let result = answer(
            &mut state,
            &one(&open).0,
            Ok(created("worktree_opened", true)),
        );
        assert!(result.actions.is_empty());
        assert!(state.overlay.is_none());
        assert!(state.fork.creation.is_none());
    }
}

#[test]
fn a_dismissed_form_cannot_attach_its_preset_to_an_unrelated_create() {
    let mut state = ready(KeybindAction::NewWorkspace, true, Some("dev"));
    state.overlay = None;
    let mut outcome = ClientShellInput::default();
    state.push_endpoint_method(plain_workspace(), &mut outcome);
    let result = answer(
        &mut state,
        &one(&outcome).0,
        Ok(created("workspace_created", false)),
    );
    assert!(result.actions.is_empty());
    assert!(state.fork.creation.is_none());
}

#[test]
fn cancelling_the_standalone_picker_clears_loading_and_ready_selections() {
    for loading in [false, true] {
        for click in [false, true] {
            let mut state = enabled(false);
            let id = default_request(&mut state, KeybindAction::NewWorkspace);
            if !loading {
                choices(
                    &mut state,
                    &id,
                    Some("dev"),
                    vec![summary("dev", None, 2, None)],
                );
            }
            let outcome = if click {
                screen(&mut state);
                let cancel = state.hits.overlay_cancel;
                assert!(cancel.width > 0);
                mouse(&mut state, cancel)
            } else {
                state.handle_input_bytes(b"\x1b")
            };
            assert!(outcome.actions.is_empty());
            assert!(state.overlay.is_none());
            assert!(state.fork.creation.is_none());
        }
    }
}
