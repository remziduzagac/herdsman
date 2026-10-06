use super::*;
use crate::api::schema::{Method, WORKTREE_GROUP_TOKEN};
use crate::protocol::ClientShellWorkspace;

fn worktree(id: &str, branch: &str, group: Option<&str>) -> ClientShellWorkspace {
    ClientShellWorkspace {
        workspace_id: id.into(),
        active_tab_id: format!("tab_{id}"),
        new_workspace_cwd: format!("/repo-{branch}"),
        number: 0,
        label: format!("repo-{branch}"),
        custom_label: false,
        branch: Some(format!("worktree/{branch}")),
        git_ahead_behind: None,
        tokens: group
            .map(|group| vec![(WORKTREE_GROUP_TOKEN.to_owned(), group.to_owned())])
            .unwrap_or_default(),
        worktree: Some(ClientShellWorktree {
            key: "repo".into(),
            label: "repo".into(),
            is_linked_worktree: true,
        }),
        focused: false,
        agent_status: AgentStatus::Idle,
    }
}

/// `main` and four of its linked worktrees: `a` and `c` in features, `d` in
/// merges, `b` in no group.
fn grouped_snapshot() -> ClientShellSnapshot {
    let mut snapshot = snapshot();
    snapshot.workspaces[0].worktree = Some(ClientShellWorktree {
        key: "repo".into(),
        label: "repo".into(),
        is_linked_worktree: false,
    });
    snapshot.workspaces.extend([
        worktree("ws_2", "a", Some("features")),
        worktree("ws_3", "b", None),
        worktree("ws_4", "c", Some("features")),
        worktree("ws_5", "d", Some("merges")),
    ]);
    for (index, workspace) in snapshot.workspaces.iter_mut().enumerate() {
        workspace.number = index + 1;
    }
    snapshot
}

fn grouped_state() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(grouped_snapshot()));
    state.set_pane_surface(surface());
    state
}

fn entry_ids(state: &ClientShellState) -> Vec<String> {
    let snapshot = state.snapshot.as_deref().unwrap();
    render::workspace_entries(snapshot, &state.collapsed_groups)
        .into_iter()
        .map(|entry| snapshot.workspaces[entry.index].workspace_id.clone())
        .collect()
}

fn sidebar_lines(state: &mut ClientShellState) -> Vec<String> {
    let frame = state.compose(106, 40).expect("composed frame");
    frame_rows(&frame)
        .into_iter()
        .map(|row| {
            row.chars()
                .take(30)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

#[test]
fn grouped_worktrees_follow_their_group_header_and_ungrouped_ones_come_last() {
    let mut state = grouped_state();

    assert_eq!(entry_ids(&state), ["ws_1", "ws_2", "ws_4", "ws_5", "ws_3"]);

    let lines = sidebar_lines(&mut state);
    let position = |needle: &str| {
        lines
            .iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle:?} not in {lines:#?}"))
    };
    let order = [
        "├─ ▾ features",
        "│  ├─ ○ a",
        "│  └─ ○ c",
        "├─ ▾ merges",
        "│  └─ ○ d",
        "└─ ○ b",
    ]
    .map(position);
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{lines:#?}");
}

#[test]
fn a_collapsed_group_hides_its_members_except_a_focused_one() {
    let mut state = grouped_state();
    state
        .collapsed_groups
        .insert(render::sidebar::worktree_groups::collapse_key(
            "repo", "features",
        ));

    assert_eq!(entry_ids(&state), ["ws_1", "ws_5", "ws_3"]);
    let lines = sidebar_lines(&mut state);
    assert!(
        lines.iter().any(|line| line.contains("▸ features")),
        "{lines:#?}"
    );

    let mut snapshot = grouped_snapshot();
    snapshot.workspaces[0].focused = false;
    snapshot.workspaces[3].focused = true;
    state.set_snapshot(Box::new(snapshot));
    assert_eq!(entry_ids(&state), ["ws_1", "ws_4", "ws_5", "ws_3"]);
}

#[test]
fn clicking_a_group_header_collapses_and_expands_it() {
    let mut state = grouped_state();
    state.compose(106, 40).expect("composed frame");
    let header = state
        .hits
        .fork
        .worktree_group_headers
        .iter()
        .find(|hit| hit.key.ends_with(":features"))
        .expect("features header")
        .rect;
    let press = |state: &mut ClientShellState| {
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: header.x + 4,
            row: header.y,
            modifiers: KeyModifiers::empty(),
        })])
    };
    let key = render::sidebar::worktree_groups::collapse_key("repo", "features");

    press(&mut state);
    assert!(state.collapsed_groups.contains(&key));
    state.compose(106, 40).expect("composed frame");
    press(&mut state);
    assert!(!state.collapsed_groups.contains(&key));
}

#[test]
fn the_group_prompt_starts_from_the_current_group_and_empty_leaves_it() {
    let mut state = grouped_state();

    state.open_worktree_group_prompt("ws_2".into());
    let Some(ClientShellOverlay::Rename(rename)) = &state.overlay else {
        panic!("group prompt should open");
    };
    assert_eq!(rename.input.trim(), "features");

    for (typed, group) in [("merges", Some("merges")), ("", None)] {
        state.open_worktree_group_prompt("ws_2".into());
        if let Some(ClientShellOverlay::Rename(rename)) = &mut state.overlay {
            rename.input = TextEditor::new(typed, false);
        }
        let mut outcome = ClientShellInput::default();
        state.save_rename_overlay(&mut outcome);
        assert!(
            matches!(
                &outcome.actions[..],
                [ClientShellAction::Endpoint { request, .. }]
                    if matches!(
                        &request.method,
                        Method::WorkspaceSetWorktreeGroup(params)
                            if params.workspace_id == "ws_2" && params.group.as_deref() == group
                    )
            ),
            "{typed:?}"
        );
    }
}

#[test]
fn each_machine_draws_and_collapses_its_own_groups() {
    let (mut state, remote_id) = super::endpoints::state_with_remote();
    state.set_snapshot(Box::new(snapshot()));
    let mut remote = grouped_snapshot();
    remote.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&remote_id, Box::new(remote));
    state.compose(106, 40).expect("composed frame");

    let header = state
        .hits
        .fork
        .worktree_group_headers
        .iter()
        .find(|hit| hit.endpoint_id == remote_id && hit.key.ends_with(":features"))
        .expect("remote features header")
        .clone();
    assert!(state
        .hits
        .fork
        .worktree_group_headers
        .iter()
        .all(|hit| hit.endpoint_id == remote_id));

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: header.rect.x + 4,
        row: header.rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.remote_collapsed_groups[&remote_id].contains(&header.key));
    assert!(state.collapsed_groups.is_empty());
}
