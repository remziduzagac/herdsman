use super::*;

fn agent(pane_id: &str, name: &str) -> ClientShellAgent {
    ClientShellAgent {
        pane_id: pane_id.into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some(name.into()),
        display_agent: None,
        agent: None,
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: AgentStatus::Idle,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: false,
    }
}

fn state_with(config: &Config) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(config));
    let mut projected = snapshot();
    projected.server_keybindings_toml = config.local_keybindings_profile_toml().ok();
    projected.agents = vec![agent("pane_1", "first"), agent("pane_2", "second")];
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn sidebar_lines(state: &mut ClientShellState) -> Vec<String> {
    let frame = state.compose(106, 40).expect("composed frame");
    frame_rows(&frame)
        .into_iter()
        .map(|row| {
            row.chars()
                .take(26)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

fn title_row(lines: &[String]) -> usize {
    lines
        .iter()
        .position(|line| line.starts_with(" agents"))
        .unwrap_or_else(|| panic!("no agents title in {lines:#?}"))
}

#[test]
fn the_agents_title_collapses_the_panel_and_opens_it_again() {
    let mut state = state_with(&Config::default());

    let open = sidebar_lines(&mut state);
    let open_title = title_row(&open);
    assert!(open[open_title].starts_with(" agents ▾"), "{open:#?}");
    assert!(open.iter().any(|line| line.contains("first")), "{open:#?}");

    let toggle = state.hits.fork.agents_toggle;
    assert!(!toggle.is_empty());
    let click = |state: &mut ClientShellState| {
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: toggle.x + 2,
            row: toggle.y,
            modifiers: KeyModifiers::empty(),
        })])
    };

    click(&mut state);
    assert!(state.fork.agents_collapsed);
    let collapsed = sidebar_lines(&mut state);
    let collapsed_title = title_row(&collapsed);
    assert!(
        collapsed[collapsed_title].starts_with(" agents ▸"),
        "{collapsed:#?}"
    );
    assert_eq!(
        collapsed_title, 38,
        "the title sits one row above the bottom"
    );
    assert!(collapsed_title > open_title);
    assert!(
        !collapsed.iter().any(|line| line.contains("first")),
        "{collapsed:#?}"
    );
    assert_eq!(state.hits.sidebar_section_divider, Rect::default());

    let toggle = state.hits.fork.agents_toggle;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: toggle.x + 2,
        row: toggle.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(!state.fork.agents_collapsed);
}

#[test]
fn prefix_a_toggles_the_panel_with_local_and_endpoint_keybindings() {
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

        for (bytes, collapsed) in [(&b"\x02a"[..], true), (b"\x1b[98;5u\x1b[97u", false)] {
            let outcome = state.handle_input_bytes(bytes);
            let sent = outcome
                .actions
                .iter()
                .any(|action| matches!(action, ClientShellAction::Endpoint { .. }));
            assert!(
                !sent,
                "{source:?}: the panel is the client's, nothing goes to the server"
            );
            assert_eq!(
                state.fork.agents_collapsed, collapsed,
                "{source:?} {bytes:?}"
            );
        }
    }
}

#[test]
fn agent_panel_start_collapsed_starts_the_client_collapsed() {
    let mut config = Config::default();
    config.ui.agent_panel_start_collapsed = true;

    let mut state = state_with(&config);

    assert!(state.fork.agents_collapsed);
    let lines = sidebar_lines(&mut state);
    assert!(
        lines[title_row(&lines)].starts_with(" agents ▸"),
        "{lines:#?}"
    );
}

#[test]
fn agent_panel_start_collapsed_and_the_key_parse_from_config() {
    let config: Config = toml::from_str(
        "[ui]\nagent_panel_start_collapsed = true\n[keys]\ntoggle_agent_panel = \"prefix+shift+a\"\n",
    )
    .expect("config parses");

    assert!(config.ui.agent_panel_start_collapsed);
    assert!(!Config::default().ui.agent_panel_start_collapsed);
    assert_eq!(
        config.keys.toggle_agent_panel,
        crate::config::BindingConfig::one("prefix+shift+a")
    );
}

/// With saved machines the sidebar lists every machine's agents; collapsing
/// works the same there, and the sidebar's own `«` stays on the bottom row.
#[test]
fn the_machines_sidebar_collapses_its_agents_panel_too() {
    let (mut state, _remote) = super::endpoints::state_with_remote();

    state.compose(100, 28).expect("machines sidebar");
    let toggle = state.hits.fork.agents_toggle;
    assert!(!toggle.is_empty());
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: toggle.x + 2,
        row: toggle.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.fork.agents_collapsed);

    let frame = state.compose(100, 28).expect("collapsed agents panel");
    let lines = frame_rows(&frame);
    assert!(lines[26].starts_with(" agents ▸"), "{lines:#?}");
    assert!(lines[0].starts_with(" machines"), "{lines:#?}");
    assert_eq!(state.hits.fork.agents_toggle.y, 26);
    let sidebar_toggle = state.hits.sidebar_toggle;
    assert_eq!(sidebar_toggle.y, 27);
    let buffer = frame.to_ratatui_buffer().expect("sidebar buffer");
    assert_eq!(buffer[(sidebar_toggle.x, sidebar_toggle.y)].symbol(), "«");
}
