//! Stacked panes in the pane API: `pane.stack`, `pane.focus_stacked`,
//! `pane.focus_stacked_at`, `pane.focus_stack_strip`, `pane.stacks`, and moving
//! a pane into a stack.
//!
//! herdsman's own module. `panes.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;
use crate::api::schema::{
    PaneFocusStackStripParams, PaneFocusStackedAtParams, PaneFocusStackedParams, PaneStackInfo,
    PaneStackParams, PaneStacksParams, SuccessResponse,
};

impl App {
    /// Spawn exactly as a split does, then fold the new pane into the target's
    /// slot, so stacking shares the split path's spawn, focus, and rollback.
    pub(in crate::app::api) fn handle_pane_stack(
        &mut self,
        id: String,
        params: PaneStackParams,
    ) -> String {
        let split = PaneSplitParams {
            workspace_id: params.workspace_id,
            target_pane_id: params.target_pane_id,
            direction: crate::api::schema::SplitDirection::Right,
            ratio: None,
            cwd: params.cwd,
            focus: params.focus,
            right_click: params.right_click,
            env: params.env,
        };
        self.state.fork.split_into_stack = true;
        let response = self.handle_pane_split(id, split);
        self.state.fork.split_into_stack = false;
        response
    }

    /// `handle_pane_split`'s hook: while `pane.stack` runs, fold the new pane
    /// into the target's slot as a hidden member.
    pub(super) fn fold_split_into_stack(
        &mut self,
        ws_idx: usize,
        tab_idx: usize,
        target: PaneId,
        new_pane: PaneId,
    ) {
        if self.state.fork.split_into_stack
            && !self.state.workspaces[ws_idx].tabs[tab_idx]
                .layout
                .move_into_stack(target, new_pane)
        {
            tracing::warn!(pane = ?new_pane, "new pane could not join its target stack");
        }
    }

    /// Resolve a stack member by index and focus it like `pane.focus`, which
    /// reveals it. With no such member this answers with the pane unchanged.
    pub(in crate::app::api) fn handle_pane_focus_stacked(
        &mut self,
        id: String,
        params: PaneFocusStackedParams,
    ) -> String {
        if params.index.is_some() == params.step.is_some() {
            return encode_error(id, "invalid_params", "pass exactly one of index or step");
        }
        let Some((ws_idx, pane_id)) = self.resolve_optional_pane(params.pane_id.as_deref()) else {
            return encode_error(id, "pane_not_found", "pane not found");
        };
        let member = self.state.workspaces.get(ws_idx).and_then(|ws| {
            let tab = ws.tabs.get(ws.find_tab_index_for_pane(pane_id)?)?;
            let (members, active) = tab.layout.stack_members(pane_id)?;
            let index = match (params.index, params.step) {
                (Some(index), _) => index,
                // Wraps both ways, as next_tab and previous_tab do.
                (None, Some(step)) => {
                    let len = i64::try_from(members.len()).ok()?;
                    let from = i64::try_from(active).ok()?;
                    usize::try_from((from + i64::from(step)).rem_euclid(len)).ok()?
                }
                (None, None) => return None,
            };
            members.get(index).copied()
        });
        if let Some(member) = member {
            self.state.focus_pane_in_workspace(ws_idx, member);
            self.state.mark_active_tab_seen();
            self.state.mode = crate::app::Mode::Terminal;
        }

        let Some(pane) = self.pane_info(ws_idx, member.unwrap_or(pane_id)) else {
            return encode_error(id, "pane_not_found", "pane not found");
        };
        encode_success(id, ResponseResult::PaneInfo { pane })
    }

    /// Every stack, in layout order, with its members and visible member.
    pub(in crate::app::api) fn handle_pane_stacks(
        &mut self,
        id: String,
        params: PaneStacksParams,
    ) -> String {
        let ws_indices = match params.workspace_id.as_deref() {
            Some(workspace_id) => match self.parse_workspace_id(workspace_id) {
                Some(ws_idx) => vec![ws_idx],
                None => {
                    return encode_error(
                        id,
                        "workspace_not_found",
                        format!("workspace {workspace_id} not found"),
                    )
                }
            },
            None => (0..self.state.workspaces.len()).collect(),
        };
        let mut stacks = Vec::new();
        for ws_idx in ws_indices {
            let workspace_id = self.public_workspace_id(ws_idx);
            for (tab_idx, tab) in self.state.workspaces[ws_idx].tabs.iter().enumerate() {
                let Some(tab_id) = self.public_tab_id(ws_idx, tab_idx) else {
                    continue;
                };
                for pane in tab.layout.pane_ids() {
                    // Report each stack once, at its first member.
                    let Some((members, active)) = tab.layout.stack_members(pane) else {
                        continue;
                    };
                    if members.first() != Some(&pane) {
                        continue;
                    }
                    let pane_ids = members
                        .iter()
                        .filter_map(|member| self.public_pane_id(ws_idx, *member))
                        .collect::<Vec<_>>();
                    let Some(visible_pane_id) = members
                        .get(active)
                        .and_then(|member| self.public_pane_id(ws_idx, *member))
                    else {
                        continue;
                    };
                    stacks.push(PaneStackInfo {
                        workspace_id: workspace_id.clone(),
                        tab_id: tab_id.clone(),
                        pane_ids,
                        visible_pane_id,
                    });
                }
            }
        }
        encode_success(id, ResponseResult::PaneStacks { stacks })
    }

    /// `pane.move` into a stack. Within a tab the pane folds straight into the
    /// target's slot. From another tab or workspace, the ordinary move places
    /// it beside the target first, with all of that path's identity care, and
    /// then it folds in.
    pub(super) fn handle_pane_move_to_stack(
        &mut self,
        id: String,
        pane_id: String,
        (source_ws_idx, source_tab_idx, source_pane): (usize, usize, PaneId),
        raw_target: &str,
        focus: bool,
    ) -> String {
        let Some((target_ws_idx, target)) = self.parse_pane_id(raw_target) else {
            return encode_error(
                id,
                "target_pane_not_found",
                format!("target pane {raw_target} not found"),
            );
        };
        let Some(target_tab_idx) =
            self.state.workspaces[target_ws_idx].find_tab_index_for_pane(target)
        else {
            return encode_error(
                id,
                "target_pane_not_found",
                format!("target pane {raw_target} not found"),
            );
        };
        if (target_ws_idx, target) == (source_ws_idx, source_pane) {
            return encode_error(id, "invalid_target", "a pane cannot join its own stack");
        }

        if (target_ws_idx, target_tab_idx) != (source_ws_idx, source_tab_idx) {
            let Some(tab_id) = self.public_tab_id(target_ws_idx, target_tab_idx) else {
                return encode_error(id, "tab_not_found", "target tab not found");
            };
            let response = self.handle_pane_move(
                id.clone(),
                PaneMoveParams {
                    pane_id,
                    destination: PaneMoveDestination::Tab {
                        tab_id,
                        target_pane_id: Some(raw_target.to_owned()),
                        split: crate::api::schema::SplitDirection::Right,
                        ratio: None,
                    },
                    focus,
                },
            );
            let Ok(SuccessResponse {
                result: ResponseResult::PaneMove { mut move_result },
                ..
            }) = serde_json::from_str::<SuccessResponse>(&response)
            else {
                return response;
            };
            let Some((ws_idx, moved)) = self.parse_pane_id(&move_result.pane.pane_id) else {
                return response;
            };
            let Some(tab_idx) = self.state.workspaces[ws_idx].find_tab_index_for_pane(moved) else {
                return response;
            };
            if !move_result.changed
                || !self.state.workspaces[ws_idx].tabs[tab_idx]
                    .layout
                    .move_into_stack(target, moved)
            {
                return response;
            }
            let (Some(pane), Some(layout)) = (
                self.pane_info(ws_idx, moved),
                self.pane_layout_snapshot(ws_idx, tab_idx),
            ) else {
                return response;
            };
            move_result.focused_pane_id = layout.focused_pane_id.clone();
            move_result.pane = Box::new(pane);
            move_result.target_layout = Box::new(layout);
            self.schedule_session_save();
            self.emit_layout_updated_event(ws_idx, tab_idx);
            return encode_success(id, ResponseResult::PaneMove { move_result });
        }

        if !self.state.workspaces[source_ws_idx].tabs[source_tab_idx]
            .layout
            .move_into_stack(target, source_pane)
        {
            return encode_error(
                id,
                "already_stacked",
                format!("{pane_id} already shares a stack with {raw_target}"),
            );
        }
        if focus {
            self.state
                .focus_pane_in_workspace(source_ws_idx, source_pane);
            self.state.mark_active_tab_seen();
            self.state.mode = crate::app::Mode::Terminal;
        }
        self.schedule_session_save();
        self.emit_layout_updated_event(source_ws_idx, source_tab_idx);
        let (Some(pane), Some(layout), Some(previous_tab_id)) = (
            self.pane_info(source_ws_idx, source_pane),
            self.pane_layout_snapshot(source_ws_idx, source_tab_idx),
            self.public_tab_id(source_ws_idx, source_tab_idx),
        ) else {
            return encode_error(id, "pane_not_found", "moved pane not found");
        };
        encode_success(
            id,
            ResponseResult::PaneMove {
                move_result: PaneMoveResult {
                    changed: true,
                    reason: None,
                    previous_pane_id: pane.pane_id.clone(),
                    previous_workspace_id: self.public_workspace_id(source_ws_idx),
                    previous_tab_id,
                    focused_pane_id: layout.focused_pane_id.clone(),
                    pane: Box::new(pane),
                    source_layout: None,
                    target_layout: Box::new(layout),
                    created_workspace: None,
                    created_tab: None,
                    closed_workspace_id: None,
                    closed_tab_id: None,
                },
            },
        )
    }

    /// A click on the row next to a pane's content, from clients that predate
    /// the strip separator: `pane.focus_stack_strip` one row out.
    pub(in crate::app::api) fn handle_pane_focus_stacked_at(
        &mut self,
        id: String,
        params: PaneFocusStackedAtParams,
    ) -> String {
        self.handle_pane_focus_stack_strip(
            id,
            PaneFocusStackStripParams {
                pane_id: params.pane_id,
                column: params.column,
                width: params.width,
                edge: params.edge,
                offset: 1,
            },
        )
    }

    /// A click beside a pane's content: the endpoint drew any stack strip
    /// there, so it resolves which member the click hit, then focuses that
    /// member, or the pane itself, exactly as `pane.focus` does.
    pub(in crate::app::api) fn handle_pane_focus_stack_strip(
        &mut self,
        id: String,
        params: PaneFocusStackStripParams,
    ) -> String {
        let Some((ws_idx, pane_id)) = self.parse_pane_id(&params.pane_id) else {
            return pane_not_found(id, &params.pane_id);
        };
        let placement = crate::ui::StripPlacement::of(&self.state);
        let member = (params.edge == placement.edge() && params.offset == placement.strip_offset())
            .then(|| {
                let ws = self.state.workspaces.get(ws_idx)?;
                let tab = ws.tabs.get(ws.find_tab_index_for_pane(pane_id)?)?;
                (!tab.zoomed).then_some(())?;
                crate::ui::stack_member_at(&self.state, tab, pane_id, params.column, params.width)
            })
            .flatten();
        let Some(target) = self.public_pane_id(ws_idx, member.unwrap_or(pane_id)) else {
            return pane_not_found(id, &params.pane_id);
        };
        self.handle_pane_focus(id, PaneTarget { pane_id: target })
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{
        app_with_linked_worktree, app_with_test_workspace, seed_terminal_states,
    };
    use super::*;
    use crate::api::schema::{ErrorResponse, PaneContentEdge};

    /// A workspace whose new panes spawn a short-lived real process.
    fn app_that_spawns_panes() -> (App, PaneId, String) {
        let (mut app, public_pane_id) = app_with_test_workspace();
        app.state.default_shell = crate::app::api::test_support::exiting_test_command().into();
        app.state.shell_mode = crate::config::ShellModeConfig::NonLogin;
        app.state.active = Some(0);
        app.state.selected = 0;
        let root = app.state.workspaces[0].tabs[0].root_pane;
        (app, root, public_pane_id)
    }

    fn stacked_pane(app: &App, response: &str) -> PaneId {
        let success: SuccessResponse = serde_json::from_str(response).unwrap();
        let ResponseResult::PaneInfo { pane } = success.result else {
            panic!("expected pane info response");
        };
        app.parse_pane_id(&pane.pane_id).unwrap().1
    }

    #[tokio::test]
    async fn pane_stack_opens_a_hidden_member_in_the_target_slot() {
        let (mut app, root, root_public) = app_that_spawns_panes();

        let response = app.handle_pane_stack(
            "req".into(),
            PaneStackParams {
                target_pane_id: Some(root_public),
                ..Default::default()
            },
        );

        let member = stacked_pane(&app, &response);
        let layout = &app.state.workspaces[0].tabs[0].layout;
        assert_eq!(layout.stack_members(root), Some((&[root, member][..], 0)));
        assert_eq!(layout.focused(), root);
        assert!(!layout.is_visible(member));
        assert!(matches!(
            &app.event_hub.events_after(0).last().expect("layout event").1.data,
            EventData::LayoutUpdated { layout } if layout.panes.len() == 1
        ));
        app.state.assert_invariants_for_test();
        crate::app::api::test_support::shutdown_test_runtimes(&mut app);
    }

    #[tokio::test]
    async fn pane_stack_with_focus_shows_and_focuses_the_new_member() {
        let (mut app, root, root_public) = app_that_spawns_panes();

        let response = app.handle_pane_stack(
            "req".into(),
            PaneStackParams {
                target_pane_id: Some(root_public),
                focus: true,
                ..Default::default()
            },
        );

        let member = stacked_pane(&app, &response);
        let layout = &app.state.workspaces[0].tabs[0].layout;
        assert_eq!(layout.stack_members(root), Some((&[root, member][..], 1)));
        assert_eq!(layout.focused(), member);
        app.state.assert_invariants_for_test();
        crate::app::api::test_support::shutdown_test_runtimes(&mut app);
    }

    #[tokio::test]
    async fn pane_stack_onto_a_member_joins_the_same_stack() {
        let (mut app, root, root_public) = app_that_spawns_panes();
        let response = app.handle_pane_stack(
            "req".into(),
            PaneStackParams {
                target_pane_id: Some(root_public),
                ..Default::default()
            },
        );
        let first = stacked_pane(&app, &response);
        let first_public = app.public_pane_id(0, first).unwrap();

        let response = app.handle_pane_stack(
            "req".into(),
            PaneStackParams {
                target_pane_id: Some(first_public),
                ..Default::default()
            },
        );
        let second = stacked_pane(&app, &response);

        let layout = &app.state.workspaces[0].tabs[0].layout;
        assert_eq!(
            layout.stack_members(second),
            Some((&[root, first, second][..], 0))
        );
        crate::app::api::test_support::shutdown_test_runtimes(&mut app);
    }

    /// `root | S[a, *b]` in the active tab, focus on `b`.
    fn app_with_stack() -> (App, PaneId, PaneId, PaneId) {
        let (mut app, _) = app_with_test_workspace();
        app.state.active = Some(0);
        let root = app.state.workspaces[0].tabs[0].root_pane;
        let a = app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        let b = app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        app.state.ensure_test_terminals();
        assert!(app.state.workspaces[0].tabs[0].layout.move_into_stack(a, b));
        (app, root, a, b)
    }

    #[test]
    fn pane_focus_stacked_reveals_and_focuses_the_member_at_an_index() {
        let (mut app, _, a, b) = app_with_stack();
        app.state.workspaces[0].tabs[0]
            .panes
            .get_mut(&a)
            .unwrap()
            .seen = false;

        let response = app.handle_pane_focus_stacked(
            "req".into(),
            PaneFocusStackedParams {
                index: Some(0),
                ..Default::default()
            },
        );

        let member = stacked_pane(&app, &response);
        let tab = &app.state.workspaces[0].tabs[0];
        assert_eq!(member, a);
        assert_eq!(tab.layout.focused(), a);
        assert!(tab.layout.is_visible(a) && !tab.layout.is_visible(b));
        assert!(tab.panes[&a].seen);
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn pane_focus_stacked_steps_through_members_wrapping_both_ways() {
        let (mut app, _, a, b) = app_with_stack();
        let step = |app: &mut App, step| {
            let response = app.handle_pane_focus_stacked(
                "req".into(),
                PaneFocusStackedParams {
                    step: Some(step),
                    ..Default::default()
                },
            );
            stacked_pane(app, &response)
        };

        // Focus starts on b, the last of [a, b].
        assert_eq!(step(&mut app, 1), a);
        assert_eq!(step(&mut app, 1), b);
        assert_eq!(step(&mut app, -1), a);
        assert_eq!(step(&mut app, -1), b);
        assert_eq!(step(&mut app, 4), b);
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn pane_focus_stacked_leaves_focus_alone_without_a_member_there() {
        let (mut app, root, _, b) = app_with_stack();
        let root_public = app.public_pane_id(0, root).unwrap();

        for (pane_id, index, step) in [
            (None, Some(2), None),
            (Some(root_public.clone()), Some(0), None),
            (Some(root_public), None, Some(1)),
        ] {
            let response = app.handle_pane_focus_stacked(
                "req".into(),
                PaneFocusStackedParams {
                    pane_id,
                    index,
                    step,
                },
            );
            let success: SuccessResponse = serde_json::from_str(&response).unwrap();
            assert!(matches!(success.result, ResponseResult::PaneInfo { .. }));
            assert_eq!(app.state.workspaces[0].tabs[0].layout.focused(), b);
        }
    }

    #[test]
    fn pane_focus_stacked_needs_exactly_one_of_index_or_step() {
        let (mut app, _, _, _) = app_with_stack();

        for (index, step) in [(None, None), (Some(0), Some(1))] {
            let response = app.handle_pane_focus_stacked(
                "req".into(),
                PaneFocusStackedParams {
                    pane_id: None,
                    index,
                    step,
                },
            );
            let error: ErrorResponse = serde_json::from_str(&response).unwrap();
            assert_eq!(error.error.code, "invalid_params");
        }
    }

    fn focus_stack_strip(
        app: &mut App,
        pane: PaneId,
        column: u16,
        edge: PaneContentEdge,
        offset: u16,
    ) -> PaneId {
        let pane_id = app.public_pane_id(0, pane).unwrap();
        let response = app.handle_pane_focus_stack_strip(
            "req".into(),
            PaneFocusStackStripParams {
                pane_id,
                column,
                width: 40,
                edge,
                offset,
            },
        );
        stacked_pane(app, &response)
    }

    fn focus_stacked_at(app: &mut App, pane: PaneId, column: u16, edge: PaneContentEdge) -> PaneId {
        let pane_id = app.public_pane_id(0, pane).unwrap();
        let response = app.handle_pane_focus_stacked_at(
            "req".into(),
            PaneFocusStackedAtParams {
                pane_id,
                column,
                width: 40,
                edge,
            },
        );
        stacked_pane(app, &response)
    }

    #[test]
    fn pane_focus_stack_strip_focuses_the_member_under_a_strip_column() {
        // The strip defaults to the bottom edge, past a separator line.
        // Unlabelled members draw as " 1 " and " 2 ": columns 0-2 and 3-5.
        let (mut app, _, a, b) = app_with_stack();
        let bottom = PaneContentEdge::Bottom;

        assert_eq!(focus_stack_strip(&mut app, b, 1, bottom, 2), a);
        let layout = &app.state.workspaces[0].tabs[0].layout;
        assert_eq!((layout.focused(), layout.is_visible(b)), (a, false));
        assert_eq!(focus_stack_strip(&mut app, a, 4, bottom, 2), b);
        assert_eq!(focus_stack_strip(&mut app, b, 20, bottom, 2), b);
        app.state.fork.stack_strip.position = crate::config::TabBarPositionConfig::Top;
        assert_eq!(
            focus_stack_strip(&mut app, b, 1, PaneContentEdge::Top, 2),
            a
        );
        app.state.fork.stack_strip.separator = false;
        assert_eq!(
            focus_stack_strip(&mut app, a, 4, PaneContentEdge::Top, 1),
            b
        );
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn pane_focus_stack_strip_off_the_strip_focuses_the_pane_itself() {
        let (mut app, root, a, b) = app_with_stack();
        let bottom = PaneContentEdge::Bottom;

        assert_eq!(focus_stack_strip(&mut app, b, 1, bottom, 1), b, "the line");
        assert_eq!(focus_stack_strip(&mut app, b, 1, bottom, 3), b, "the frame");
        assert_eq!(
            focus_stack_strip(&mut app, b, 1, PaneContentEdge::Top, 2),
            b
        );
        assert_eq!(focus_stack_strip(&mut app, root, 1, bottom, 2), root);
        app.state.fork.stack_strip.separator = false;
        assert_eq!(focus_stack_strip(&mut app, b, 1, bottom, 2), b);
        app.state.workspaces[0].tabs[0].zoomed = true;
        assert_eq!(focus_stack_strip(&mut app, b, 1, bottom, 1), b);
        assert!(!app.state.workspaces[0].tabs[0].layout.is_visible(a));
    }

    #[test]
    fn pane_focus_stacked_at_is_a_click_on_the_row_next_to_the_content() {
        let (mut app, _, a, b) = app_with_stack();

        assert_eq!(focus_stacked_at(&mut app, b, 1, PaneContentEdge::Bottom), b);
        app.state.fork.stack_strip.separator = false;
        assert_eq!(focus_stacked_at(&mut app, b, 1, PaneContentEdge::Bottom), a);
    }

    fn move_to_stack(app: &mut App, pane: PaneId, target: PaneId, focus: bool) -> String {
        let pane_id = app.public_pane_id(0, pane).unwrap();
        let target_pane_id = app.public_pane_id(0, target).unwrap();
        app.handle_pane_move(
            "req".into(),
            PaneMoveParams {
                pane_id,
                destination: PaneMoveDestination::Stack { target_pane_id },
                focus,
            },
        )
    }

    fn moved(response: &str) -> PaneMoveResult {
        let success: SuccessResponse = serde_json::from_str(response).unwrap();
        let ResponseResult::PaneMove { move_result } = success.result else {
            panic!("expected pane move response");
        };
        move_result
    }

    #[test]
    fn pane_move_to_stack_within_a_tab_joins_the_target_stack() {
        // root | S[a, *b], focus on b.
        let (mut app, root, a, b) = app_with_stack();

        let result = moved(&move_to_stack(&mut app, root, a, false));

        assert!(result.changed);
        assert_eq!(result.pane.pane_id, app.public_pane_id(0, root).unwrap());
        let layout = &app.state.workspaces[0].tabs[0].layout;
        assert_eq!(layout.stack_members(root), Some((&[a, b, root][..], 1)));
        assert_eq!(layout.focused(), b);
        assert_eq!(result.target_layout.panes.len(), 1);
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn pane_move_to_stack_with_focus_shows_the_moved_pane() {
        let (mut app, root, a, b) = app_with_stack();

        moved(&move_to_stack(&mut app, root, b, true));

        let layout = &app.state.workspaces[0].tabs[0].layout;
        assert_eq!(layout.stack_members(root), Some((&[a, b, root][..], 2)));
        assert_eq!(layout.focused(), root);
    }

    #[test]
    fn pane_move_to_stack_from_another_tab_moves_then_joins() {
        let mut app = app_with_linked_worktree();
        let target = app.state.workspaces[0].tabs[0].root_pane;
        let source_tab = app.state.workspaces[0].test_add_tab(Some("source"));
        let source = app.state.workspaces[0].tabs[source_tab].root_pane;
        seed_terminal_states(&mut app);

        let result = moved(&move_to_stack(&mut app, source, target, true));

        assert!(result.changed);
        assert!(result.closed_tab_id.is_some());
        assert_eq!(result.target_layout.panes.len(), 1);
        let tab = &app.state.workspaces[0].tabs[0];
        assert_eq!(
            tab.layout.stack_members(target),
            Some((&[target, source][..], 1))
        );
        assert_eq!(tab.layout.focused(), source);
        app.state.assert_invariants_for_test();
    }

    #[test]
    fn pane_move_to_stack_rejects_itself_and_a_current_stackmate() {
        let (mut app, _, a, b) = app_with_stack();

        for (pane, target, code) in [(a, a, "invalid_target"), (a, b, "already_stacked")] {
            let error: ErrorResponse =
                serde_json::from_str(&move_to_stack(&mut app, pane, target, false)).unwrap();
            assert_eq!(error.error.code, code);
        }
    }

    #[test]
    fn pane_stacks_lists_each_stack_once_with_its_visible_member() {
        let (mut app, _, a, b) = app_with_stack();

        let response = app.handle_pane_stacks("req".into(), PaneStacksParams::default());

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::PaneStacks { stacks } = success.result else {
            panic!("expected pane stacks response");
        };
        let public = |id| app.public_pane_id(0, id).unwrap();
        assert_eq!(
            stacks,
            [PaneStackInfo {
                workspace_id: app.public_workspace_id(0),
                tab_id: app.public_tab_id(0, 0).unwrap(),
                pane_ids: vec![public(a), public(b)],
                visible_pane_id: public(b),
            }]
        );
    }

    #[test]
    fn pane_stack_rejects_a_missing_target() {
        let (mut app, _) = app_with_test_workspace();

        let response = app.handle_pane_stack(
            "req".into(),
            PaneStackParams {
                target_pane_id: Some("w9:p9".into()),
                ..Default::default()
            },
        );

        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "pane_not_found");
    }
}
