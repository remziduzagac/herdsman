use super::*;
use crate::api::schema::{
    EmptyParams, Method, OutdatedIntegration, ResponseResult, UpdateHandoff, UpdateStatus,
    UpdatedIntegration,
};
use std::time::{Duration, Instant};

const METHODS: &[&str] = &[
    "update.status",
    "update.install",
    "update.handoff",
    "update.integrations",
    "update.skip",
];

fn shell_with(methods: Option<&[&str]>) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.set_endpoint_methods(
        methods.map(|methods| methods.iter().map(|name| (*name).to_owned()).collect()),
    );
    state
}

fn status() -> UpdateStatus {
    UpdateStatus {
        running: "0.3.0".into(),
        available: Some("0.4.0".into()),
        channel: "preview".into(),
        self_update: true,
        ..Default::default()
    }
}

fn requests(outcome: &ClientShellInput) -> Vec<(ClientEndpointId, String, Method)> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint {
                endpoint_id,
                request,
                ..
            } => Some((
                endpoint_id.clone(),
                request.id.clone(),
                request.method.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn one(outcome: &ClientShellInput) -> (String, Method) {
    let requests = requests(outcome);
    assert_eq!(requests.len(), 1, "{requests:?}");
    (requests[0].1.clone(), requests[0].2.clone())
}

fn reply(
    state: &mut ClientShellState,
    id: &str,
    result: Result<ResponseResult, ClientShellEndpointError>,
) -> ClientShellInput {
    let boot = state.snapshot.as_ref().expect("snapshot").boot_id.clone();
    let (repaint, actions) = state.handle_endpoint_result(&boot, id, result);
    ClientShellInput {
        repaint,
        actions,
        ..Default::default()
    }
}

fn answer_status(state: &mut ClientShellState, id: &str, status: UpdateStatus) -> ClientShellInput {
    reply(state, id, Ok(ResponseResult::UpdateStatus { status }))
}

fn attach(state: &mut ClientShellState, now: Instant) -> String {
    let outcome = state.tick_selection_autoscroll(now);
    let (id, method) = one(&outcome);
    assert!(matches!(method, Method::UpdateStatus(_)));
    id
}

fn open(state: &mut ClientShellState, status: UpdateStatus) {
    let id = attach(state, Instant::now());
    answer_status(state, &id, status.clone());
    let mut outcome = ClientShellInput::default();
    state.open_update_dialog(&mut outcome);
    let (id, Method::UpdateStatus(_)) = one(&outcome) else {
        panic!("dialog status");
    };
    answer_status(state, &id, status);
}

fn dialog(state: &ClientShellState) -> &fork::UpdateDialog {
    let Some(ClientShellOverlay::UpdateDialog(dialog)) = state.overlay.as_ref() else {
        panic!("update dialog");
    };
    dialog
}

fn choose(state: &mut ClientShellState, label: &str) -> ClientShellInput {
    let Some(ClientShellOverlay::UpdateDialog(dialog)) = state.overlay.as_mut() else {
        panic!("update dialog");
    };
    dialog.selected = dialog
        .actions()
        .iter()
        .position(|action| action.label() == label)
        .expect("offered action");
    state.handle_input_bytes(b"\r")
}

fn text(state: &mut ClientShellState) -> String {
    frame_rows(&state.compose(106, 30).expect("frame")).join("\n")
}

fn ready_status() -> UpdateStatus {
    UpdateStatus {
        installed: Some("0.4.0".into()),
        handoff: Some(UpdateHandoff::Ready),
        available: None,
        ..status()
    }
}

#[test]
fn old_or_unadvertised_servers_keep_the_existing_update_chrome() {
    for methods in [None, Some(["workspace.create"].as_slice())] {
        let mut state = shell_with(methods);
        let mut snapshot = snapshot();
        snapshot.update_available = Some("0.4.0".into());
        state.set_snapshot(Box::new(snapshot));
        assert!(state
            .tick_selection_autoscroll(Instant::now())
            .actions
            .is_empty());
        state.mode = ClientShellMode::Navigate;
        assert!(text(&mut state).contains("update ready"));
        assert!(state.hits.fork.update_label.is_empty());
    }
}

#[test]
fn polling_starts_on_attach_and_repeats_only_at_five_minutes_or_a_release_change() {
    let mut state = shell_with(Some(METHODS));
    let now = Instant::now();
    let id = attach(&mut state, now);
    assert!(
        state
            .tick_selection_autoscroll(now + Duration::from_secs(300))
            .actions
            .is_empty(),
        "one request in flight"
    );
    answer_status(&mut state, &id, status());
    assert!(state
        .tick_selection_autoscroll(now + Duration::from_secs(299))
        .actions
        .is_empty());
    let id = attach(&mut state, now + Duration::from_secs(300));
    answer_status(&mut state, &id, status());
    let mut next = snapshot();
    next.revision += 1;
    next.update_available = Some("0.5.0".into());
    state.set_snapshot(Box::new(next));
    let id = attach(&mut state, now + Duration::from_secs(301));
    answer_status(&mut state, &id, status());
    assert!(state
        .tick_selection_autoscroll(now + Duration::from_secs(302))
        .actions
        .is_empty());
}

#[test]
fn attach_poll_waits_for_capabilities_and_refreshes_after_a_new_boot() {
    let mut state = shell_with(None);
    let now = Instant::now();
    assert!(state.tick_selection_autoscroll(now).actions.is_empty());
    state.set_endpoint_methods(Some(
        METHODS.iter().map(|method| (*method).into()).collect(),
    ));
    let id = attach(&mut state, now);
    answer_status(&mut state, &id, ready_status());
    let mut next = snapshot();
    next.boot_id = "boot-2".into();
    state.set_snapshot(Box::new(next));
    let id = attach(&mut state, now + Duration::from_secs(1));
    answer_status(&mut state, &id, status());
}

#[test]
fn handoff_and_integrations_project_chrome_without_changing_the_endpoint_snapshot() {
    for status in [
        ready_status(),
        UpdateStatus {
            available: None,
            integrations: vec![OutdatedIntegration {
                agent: "claude".into(),
                installed: Some(10),
                expected: 11,
                paths: vec!["/config/claude/hooks.json".into()],
            }],
            ..status()
        },
    ] {
        let mut state = shell_with(Some(METHODS));
        let now = Instant::now();
        let id = attach(&mut state, now);
        answer_status(&mut state, &id, status);
        state.mode = ClientShellMode::Navigate;
        assert!(text(&mut state).contains("update ready"));
        assert!(!state.hits.fork.update_label.is_empty());
        assert!(state.endpoints[0]
            .snapshot
            .as_ref()
            .expect("original")
            .update_available
            .is_none());
        assert!(
            state
                .tick_selection_autoscroll(now + Duration::from_secs(1))
                .actions
                .is_empty(),
            "projected chrome must not trigger a refresh loop"
        );
        let mut next = snapshot();
        next.revision += 1;
        state.set_snapshot(Box::new(next));
        assert!(state
            .snapshot
            .as_ref()
            .expect("active")
            .update_available
            .is_some());
    }
}

#[test]
fn the_label_and_menu_open_the_dialog_even_without_release_notes() {
    for use_menu in [false, true] {
        let mut state = shell_with(Some(METHODS));
        let id = attach(&mut state, Instant::now());
        answer_status(&mut state, &id, ready_status());
        state.mode = ClientShellMode::Navigate;
        text(&mut state);
        let mut outcome = ClientShellInput::default();
        if use_menu {
            state.toggle_global_menu();
            state.activate_global_menu_item(3, &mut outcome);
        } else {
            let label = state.hits.fork.update_label;
            state.handle_mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: label.x,
                    row: label.y,
                    modifiers: KeyModifiers::empty(),
                },
                &mut outcome,
            );
        }
        assert!(matches!(one(&outcome).1, Method::UpdateStatus(_)));
        assert!(text(&mut state).contains("updates · Local"));
    }
}

#[test]
fn update_and_handoff_polls_the_async_install_then_hands_off_once() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, status());
    assert_eq!(dialog(&state).actions()[0].label(), "Update and hand off");
    let install = state.handle_input_bytes(b"\r");
    let (id, Method::UpdateInstall(_)) = one(&install) else {
        panic!("install");
    };
    assert!(answer_status(
        &mut state,
        &id,
        UpdateStatus {
            installing: true,
            ..status()
        }
    )
    .actions
    .is_empty());
    assert!(text(&mut state).contains("updating…"));
    assert!(state.handle_input_bytes(b"\x1b").actions.is_empty());
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::UpdateDialog(_))
    ));
    let now = Instant::now();
    assert!(state
        .tick_selection_autoscroll(now + Duration::from_secs(1))
        .actions
        .is_empty());
    let id = attach(&mut state, now + Duration::from_secs(2));
    assert!(answer_status(
        &mut state,
        &id,
        UpdateStatus {
            installing: true,
            ..status()
        }
    )
    .actions
    .is_empty());
    let id = attach(&mut state, now + Duration::from_secs(4));
    let handoff = answer_status(&mut state, &id, ready_status());
    let (id, Method::UpdateHandoff(_)) = one(&handoff) else {
        panic!("handoff when ready");
    };
    reply(&mut state, &id, Ok(ResponseResult::Ok {}));
    assert!(text(&mut state).contains("handing off…"));
    assert!(state
        .tick_selection_autoscroll(now + Duration::from_secs(400))
        .actions
        .is_empty());
}

#[test]
fn update_only_never_chains_a_handoff() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, status());
    let install = choose(&mut state, "Update only");
    let (id, Method::UpdateInstall(_)) = one(&install) else {
        panic!("install");
    };
    assert!(answer_status(
        &mut state,
        &id,
        UpdateStatus {
            installing: true,
            ..status()
        }
    )
    .actions
    .is_empty());
    let id = attach(&mut state, Instant::now() + Duration::from_secs(3));
    assert!(answer_status(&mut state, &id, ready_status())
        .actions
        .is_empty());
    assert_eq!(dialog(&state).actions()[0].label(), "Hand off");
    assert!(matches!(
        one(&choose(&mut state, "Hand off")).1,
        Method::UpdateHandoff(_)
    ));
}

#[test]
fn install_errors_stop_the_chain_and_are_shown_in_the_dialog() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, status());
    let id = one(&state.handle_input_bytes(b"\r")).0;
    answer_status(
        &mut state,
        &id,
        UpdateStatus {
            installing: true,
            ..status()
        },
    );
    let id = attach(&mut state, Instant::now() + Duration::from_secs(3));
    let result = answer_status(
        &mut state,
        &id,
        UpdateStatus {
            install_error: Some("download checksum failed".into()),
            ..status()
        },
    );
    assert!(result.actions.is_empty());
    assert!(text(&mut state).contains("download checksum failed"));
    assert!(!dialog(&state).busy());
    assert!(state.visible_endpoint_notice.is_none());
}

#[test]
fn handoff_errors_allow_retry_without_closing_the_modal() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, ready_status());
    assert_eq!(dialog(&state).actions()[0].label(), "Hand off");
    let id = one(&state.handle_input_bytes(b"\r")).0;
    reply(
        &mut state,
        &id,
        Err(ClientShellEndpointError {
            code: Some("handoff_failed".into()),
            message: "handoff failed: PTY transfer".into(),
        }),
    );
    assert!(text(&mut state).contains("PTY transfer"));
    assert_eq!(dialog(&state).actions()[0].label(), "Hand off");
}

#[test]
fn skipping_the_available_version_clears_its_attention_label() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, status());
    let (id, Method::UpdateSkip(params)) = one(&choose(&mut state, "Skip this version")) else {
        panic!("skip");
    };
    assert_eq!(params.version, "0.4.0");
    answer_status(
        &mut state,
        &id,
        UpdateStatus {
            skipped: Some("0.4.0".into()),
            ..status()
        },
    );
    assert!(state
        .snapshot
        .as_ref()
        .expect("snapshot")
        .update_available
        .is_none());
    assert_eq!(
        dialog(&state)
            .actions()
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>(),
        ["Later"]
    );
}

#[test]
fn package_manager_and_unsupported_handoff_show_instructions_without_install_actions() {
    let mut state = shell_with(Some(METHODS));
    open(
        &mut state,
        UpdateStatus {
            self_update: false,
            install_command: Some("brew upgrade herdsman".into()),
            handoff: Some(UpdateHandoff::Unsupported),
            ..status()
        },
    );
    let screen = text(&mut state);
    assert!(screen.contains("brew upgrade herdsman"));
    assert!(screen.contains("Restart needed"));
    assert_eq!(
        dialog(&state)
            .actions()
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>(),
        ["Later"]
    );
}

#[test]
fn only_advertised_actions_are_offered() {
    let mut state = shell_with(Some(&["update.status", "update.integrations"]));
    open(&mut state, ready_status());
    assert_eq!(
        dialog(&state)
            .actions()
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>(),
        ["Later"]
    );
    assert!(state.visible_endpoint_notice.is_none());
}

#[test]
fn integration_updates_list_paths_then_refresh_status_and_report_changed_files() {
    let mut state = shell_with(Some(METHODS));
    open(
        &mut state,
        UpdateStatus {
            available: None,
            integrations: vec![OutdatedIntegration {
                agent: "claude".into(),
                installed: Some(10),
                expected: 11,
                paths: vec!["/config/claude/hooks.json".into()],
            }],
            ..status()
        },
    );
    let screen = text(&mut state);
    assert!(screen.contains("Outdated integrations"));
    assert!(screen.contains("/config/claude/hooks.json"));
    let (id, Method::UpdateIntegrations(_)) = one(&choose(&mut state, "Update integrations"))
    else {
        panic!("integrations");
    };
    let refresh = reply(
        &mut state,
        &id,
        Ok(ResponseResult::IntegrationsUpdated {
            updated: vec![UpdatedIntegration {
                agent: "claude".into(),
                paths: vec!["/config/claude/hooks.json".into()],
            }],
        }),
    );
    let (id, Method::UpdateStatus(_)) = one(&refresh) else {
        panic!("refresh");
    };
    answer_status(
        &mut state,
        &id,
        UpdateStatus {
            available: None,
            ..status()
        },
    );
    assert!(text(&mut state).contains("Updated claude"));
    assert!(state
        .snapshot
        .as_ref()
        .expect("snapshot")
        .update_available
        .is_none());
}

#[test]
fn release_notes_stay_reachable_from_the_update_dialog() {
    let mut state = shell_with(Some(METHODS));
    let mut snapshot = snapshot();
    snapshot.latest_release_notes_available = true;
    snapshot.release_notes = Some(crate::protocol::ClientShellReleaseNotes {
        version: "0.3.0".into(),
        body: "Release notes body".into(),
        preview: false,
    });
    state.set_snapshot(Box::new(snapshot));
    open(&mut state, status());
    assert!(choose(&mut state, "What's new").actions.is_empty());
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::ReleaseNotes(_))
    ));
    assert!(text(&mut state).contains("Release notes body"));
}

#[test]
fn update_modal_contains_keys_and_paste_even_when_a_popup_is_present() {
    let mut state = shell_with(Some(METHODS));
    state.set_pane_surface(surface_with_popup());
    open(&mut state, status());
    assert!(state.handle_input_bytes(b"x").requests.is_empty());
    assert!(state
        .handle_raw_events(vec![RawInputEvent::Paste("secret".into())])
        .requests
        .is_empty());
    assert!(state
        .handle_raw_events(vec![RawInputEvent::Text(crate::input::TextCommit::new(
            "secret"
        ))])
        .requests
        .is_empty());
    assert!(state.clipboard_image_target().is_none());
    assert!(state.handle_input_bytes(b"\x1b").actions.is_empty());
    assert!(state.overlay.is_none());
}

#[test]
fn cancelling_a_lookup_and_receiving_its_late_answer_does_not_reopen_the_dialog() {
    let mut state = shell_with(Some(METHODS));
    let mut outcome = ClientShellInput::default();
    state.open_update_dialog(&mut outcome);
    let id = one(&outcome).0;
    state.handle_input_bytes(b"\x1b");
    assert!(answer_status(&mut state, &id, ready_status())
        .actions
        .is_empty());
    assert!(state.overlay.is_none());
}

#[test]
fn unknown_status_method_disables_only_the_optional_dialog() {
    let mut state = shell_with(Some(METHODS));
    let mut snapshot = snapshot();
    snapshot.update_available = Some("0.4.0".into());
    state.set_snapshot(Box::new(snapshot));
    let id = attach(&mut state, Instant::now());
    reply(
        &mut state,
        &id,
        Err(ClientShellEndpointError {
            code: Some("unknown_method".into()),
            message: "old server".into(),
        }),
    );
    assert!(state
        .tick_selection_autoscroll(Instant::now() + Duration::from_secs(400))
        .actions
        .is_empty());
    assert!(state.visible_endpoint_notice.is_none());
    assert!(state
        .snapshot
        .as_ref()
        .expect("snapshot")
        .update_available
        .is_some());
    assert!(state.endpoint_is_online(&ClientEndpointId::Local));
}

#[test]
fn mouse_action_rows_submit_the_selected_update() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, status());
    text(&mut state);
    let later_index = dialog(&state)
        .actions()
        .iter()
        .position(|action| action.label() == "Update only")
        .expect("update only");
    let row = state
        .hits
        .worktree_rows
        .iter()
        .find(|(_, index)| *index == later_index)
        .expect("action row")
        .0;
    let mut outcome = ClientShellInput::default();
    state.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: row.x,
            row: row.y,
            modifiers: KeyModifiers::empty(),
        },
        &mut outcome,
    );
    assert!(matches!(one(&outcome).1, Method::UpdateInstall(_)));
}

#[test]
fn ssh_dialog_names_and_targets_the_active_machine() {
    use crate::client::endpoint::{ProfileId, SavedSshEndpoint};
    let mut state = shell_with(Some(METHODS));
    let profile = SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").expect("profile"),
        label: "Build machine".into(),
        target: "dev@build.example".into(),
        session: "agents".into(),
        enabled: true,
    };
    let endpoint_id = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    let mut remote = snapshot();
    remote.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&endpoint_id, Box::new(remote));
    state.set_endpoint_status(&endpoint_id, ClientEndpointStatus::Online);
    state.set_endpoint_methods_for(
        &endpoint_id,
        Some(METHODS.iter().map(|method| (*method).into()).collect()),
    );
    assert!(state.activate_endpoint_projection(&endpoint_id));
    open(&mut state, ready_status());
    assert_eq!(dialog(&state).endpoint_label, "Build machine");
    let outcome = state.handle_input_bytes(b"\r");
    let requests = requests(&outcome);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, endpoint_id);
    assert!(matches!(
        requests[0].2,
        Method::UpdateHandoff(EmptyParams {})
    ));
    assert!(state.endpoint_is_online(&ClientEndpointId::Local));
}

#[test]
fn an_installed_update_that_needs_a_restart_stays_reachable() {
    let mut state = shell_with(Some(METHODS));
    let now = Instant::now();
    let id = attach(&mut state, now);
    let status = UpdateStatus {
        handoff: Some(UpdateHandoff::Unsupported),
        ..ready_status()
    };
    answer_status(&mut state, &id, status.clone());
    state.mode = ClientShellMode::Navigate;
    assert!(text(&mut state).contains("update ready"));
    let mut outcome = ClientShellInput::default();
    state.activate_global_menu_item(3, &mut outcome);
    answer_status(&mut state, &one(&outcome).0, status);
    assert!(text(&mut state).contains("Restart needed"));
}

#[test]
fn cancellation_and_timeouts_do_not_chain_mutating_update_actions() {
    for code in [
        "endpoint_cancelled",
        "endpoint_timeout",
        "server_unavailable",
    ] {
        let mut state = shell_with(Some(METHODS));
        open(&mut state, status());
        let id = one(&state.handle_input_bytes(b"\r")).0;
        let outcome = reply(
            &mut state,
            &id,
            Err(ClientShellEndpointError {
                code: Some(code.into()),
                message: "Action interrupted".into(),
            }),
        );
        assert!(outcome.actions.is_empty());
        assert_eq!(dialog(&state).error.as_deref(), Some("Action interrupted"));
        assert!(state.endpoint_is_online(&ClientEndpointId::Local));
    }
}

#[test]
fn failed_install_poll_can_be_dismissed_without_a_handoff() {
    let mut state = shell_with(Some(METHODS));
    open(&mut state, status());
    let id = one(&state.handle_input_bytes(b"\r")).0;
    answer_status(
        &mut state,
        &id,
        UpdateStatus {
            installing: true,
            ..status()
        },
    );
    let id = attach(&mut state, Instant::now() + Duration::from_secs(3));
    assert!(reply(
        &mut state,
        &id,
        Err(ClientShellEndpointError {
            code: Some("server_unavailable".into()),
            message: "Server unavailable".into()
        })
    )
    .actions
    .is_empty());
    assert!(!dialog(&state).busy());
    assert_eq!(
        dialog(&state)
            .actions()
            .iter()
            .map(|action| action.label())
            .collect::<Vec<_>>(),
        ["Later"]
    );
    assert!(state.handle_input_bytes(b"\x1b").actions.is_empty());
    assert!(state.overlay.is_none());
}

#[test]
fn wrapped_integration_paths_can_be_scrolled_to_the_last_file() {
    let mut state = shell_with(Some(METHODS));
    let paths = (0..30)
        .map(|index| {
            format!(
                "/config/{}/{index}-last-file.json",
                "long-directory/".repeat(10)
            )
        })
        .collect::<Vec<_>>();
    open(
        &mut state,
        UpdateStatus {
            available: None,
            integrations: vec![OutdatedIntegration {
                agent: "claude".into(),
                installed: Some(10),
                expected: 11,
                paths,
            }],
            ..status()
        },
    );
    text(&mut state);
    assert!(state.hits.help_max_scroll > 60);
    for _ in 0..100 {
        state.handle_input_bytes(b"\x1b[6~");
    }
    assert!(text(&mut state).contains("29-last-file.json"));
    assert_eq!(dialog(&state).scroll, state.hits.help_max_scroll);
}
