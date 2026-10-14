//! Stacked panes in the headless server.
//!
//! herdsman's own module. `tests/mod.rs` reaches it through a hook marked
//! `// fork: stacks tests`.

use super::*;

/// A shell client's stack request lays its tab out again for the client that
/// controls the geometry, as a split does, so a member left at another size
/// (a new member spawns at its split's size, then joins the slot) is fixed.
#[tokio::test]
async fn stack_requests_lay_the_stack_out_for_the_controlling_client() {
    let mut server = test_headless_server();
    let mut workspace = crate::workspace::Workspace::test_new("stack-geometry");
    let first = workspace.tabs[0].root_pane;
    let second = workspace.test_split(ratatui::layout::Direction::Horizontal);
    for pane in [first, second] {
        workspace.insert_test_runtime(
            pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b""),
        );
    }
    assert!(workspace.tabs[0].layout.move_into_stack(second, first));
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.mode = crate::app::Mode::Terminal;
    let first_public = server.app.public_pane_id(0, first).expect("pane id");

    let (control, _) = connect_test_shell(&mut server, 65, 100, 30);
    let _ = control.recv().expect("snapshot");
    let size = |server: &HeadlessServer, pane| {
        server.app.state.workspaces[0].test_runtimes[&pane].current_size()
    };
    let laid_out = size(&server, first);
    assert_eq!(size(&server, second), laid_out);

    for index in [0, 1] {
        server.app.state.workspaces[0].test_runtimes[&second].resize(10, 20, 0, 0);
        let (respond_to, _response_rx) = std::sync::mpsc::channel();
        assert!(server.handle_client_shell_api_request(
            65,
            crate::api::ApiRequestMessage {
                request: crate::api::schema::Request {
                    id: format!("show-member-{index}"),
                    method: crate::api::schema::Method::PaneFocusStacked(
                        crate::api::schema::PaneFocusStackedParams {
                            pane_id: Some(first_public.clone()),
                            index: Some(index),
                            step: None,
                        },
                    ),
                },
                respond_to,
                response_write_complete: None,
            },
        ));
        assert_eq!(
            size(&server, second),
            laid_out,
            "after showing member {index}"
        );
    }
    shutdown_test_runtimes(&mut server);
}
