//! Stacks in layout export, apply and validation.
//!
//! herdsman's own module. `layouts.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;

impl App {
    /// `layout_node_description` for a stack: every member, and which one shows.
    pub(super) fn layout_stack_description(
        &self,
        ws_idx: usize,
        tab_idx: usize,
        panes: &[PaneId],
        active: usize,
    ) -> Option<LayoutNode> {
        Some(LayoutNode::Stack {
            panes: panes
                .iter()
                .map(|pane_id| self.layout_pane_description(ws_idx, tab_idx, *pane_id))
                .collect::<Option<_>>()?,
            active,
        })
    }

    /// `apply_layout_node_to_pane` for a stack: `pane_id` becomes the first
    /// member, the others spawn beside it and fold in, then `active` shows.
    pub(super) fn apply_layout_stack_to_pane(
        &mut self,
        ws_idx: usize,
        pane_id: PaneId,
        panes: &[LayoutPane],
        active: usize,
    ) -> Result<(), String> {
        let mut members = vec![pane_id];
        if let Some(first) = panes.first() {
            self.apply_layout_pane_label(ws_idx, pane_id, first);
        }
        for member in panes.iter().skip(1) {
            // Spawn beside the slot, then fold the new pane into it.
            let new_pane =
                self.layout_split_pane(ws_idx, pane_id, SplitDirection::Right, 0.5, member)?;
            if !self
                .layout_mut_for_pane(ws_idx, pane_id)?
                .move_into_stack(pane_id, new_pane)
            {
                return Err("could not stack layout pane".into());
            }
            members.push(new_pane);
        }
        if let Some(visible) = members.get(active) {
            self.layout_mut_for_pane(ws_idx, pane_id)?
                .show_in_stack(*visible);
        }
        Ok(())
    }

    fn layout_mut_for_pane(
        &mut self,
        ws_idx: usize,
        pane_id: PaneId,
    ) -> Result<&mut crate::layout::TileLayout, String> {
        let ws = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .ok_or("workspace not found")?;
        let tab_idx = ws
            .find_tab_index_for_pane(pane_id)
            .ok_or("pane not found")?;
        Ok(&mut ws.tabs[tab_idx].layout)
    }
}

/// `first_layout_leaf` for a stack. Validation rejects empty stacks; the
/// default only keeps this total.
pub(super) fn first_member(panes: &[LayoutPane]) -> &LayoutPane {
    static EMPTY_STACK_LEAF: std::sync::LazyLock<LayoutPane> =
        std::sync::LazyLock::new(LayoutPane::default);
    panes.first().unwrap_or(&EMPTY_STACK_LEAF)
}

/// `validate_layout_node` for a stack: at least one member, `active` in range,
/// and every member valid as a pane.
pub(super) fn validate_stack(
    panes: &[LayoutPane],
    active: usize,
    depth: usize,
    stats: &mut LayoutTreeStats,
) -> Result<(), String> {
    if panes.is_empty() {
        return Err("stack must hold at least one pane".into());
    }
    if active >= panes.len() {
        return Err(format!(
            "stack active index {active} is out of range for {} panes",
            panes.len()
        ));
    }
    for pane in panes {
        validate_layout_node(&LayoutNode::Pane { pane: pane.clone() }, depth + 1, stats)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::tests::app_with_workspace;
    use super::*;
    use crate::api::schema::SuccessResponse;
    use crate::app::api::test_support::shutdown_test_runtimes;

    fn labelled(label: &str) -> LayoutPane {
        LayoutPane {
            label: Some(label.into()),
            ..Default::default()
        }
    }

    #[test]
    fn layout_export_describes_a_stack_and_its_visible_member() {
        let mut app = app_with_workspace();
        let root = app.state.workspaces[0].tabs[0].root_pane;
        let member = app.state.workspaces[0].test_split(Direction::Horizontal);
        app.state.ensure_test_terminals();
        let layout = &mut app.state.workspaces[0].tabs[0].layout;
        assert!(layout.move_into_stack(root, member));
        layout.focus_pane(member);

        let response = app.handle_layout_export(
            "req".into(),
            LayoutExportParams {
                tab_id: None,
                pane_id: None,
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::LayoutExport { layout } = success.result else {
            panic!("expected layout export response");
        };
        let LayoutNode::Stack { panes, active } = layout.root else {
            panic!("expected stack layout root");
        };
        assert_eq!(
            panes
                .iter()
                .map(|pane| pane.pane_id.clone())
                .collect::<Vec<_>>(),
            [root, member].map(|id| app.public_pane_id(0, id))
        );
        assert_eq!(active, 1);
        assert_eq!(
            layout.focused_pane_id,
            app.public_pane_id(0, member).unwrap()
        );
    }

    #[tokio::test]
    async fn layout_apply_builds_a_stack_showing_the_requested_member() {
        let mut app = app_with_workspace();

        let response = app.handle_layout_apply(
            "req".into(),
            LayoutApplyParams {
                workspace_id: None,
                tab_id: None,
                tab_label: Some("agents".into()),
                focus: true,
                root: LayoutNode::Split {
                    direction: SplitDirection::Right,
                    ratio: 0.4,
                    first: Box::new(LayoutNode::Pane {
                        pane: labelled("editor"),
                    }),
                    second: Box::new(LayoutNode::Stack {
                        panes: vec![labelled("claude"), labelled("codex"), labelled("gemini")],
                        active: 1,
                    }),
                },
            },
        );

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::LayoutApply { layout } = success.result else {
            panic!("expected layout apply response");
        };
        let LayoutNode::Split { first, second, .. } = layout.root else {
            panic!("expected split layout root");
        };
        let LayoutNode::Pane { pane: editor } = *first else {
            panic!("expected editor pane");
        };
        let LayoutNode::Stack { panes, active } = *second else {
            panic!("expected agent stack");
        };
        assert_eq!(editor.label.as_deref(), Some("editor"));
        assert_eq!(
            panes
                .iter()
                .map(|pane| pane.label.as_deref())
                .collect::<Vec<_>>(),
            [Some("claude"), Some("codex"), Some("gemini")]
        );
        assert_eq!(active, 1);
        assert_eq!(layout.focused_pane_id, editor.pane_id.unwrap());
        let tab = &app.state.workspaces[0].tabs[1];
        assert_eq!(tab.layout.pane_count(), 4);
        assert_eq!(
            tab.layout
                .panes(ratatui::layout::Rect::new(0, 0, 100, 40))
                .len(),
            2
        );
        app.state.assert_invariants_for_test();
        shutdown_test_runtimes(&mut app);
    }

    #[test]
    fn layout_validation_rejects_empty_stacks_and_hidden_indexes_out_of_range() {
        let empty = LayoutNode::Stack {
            panes: Vec::new(),
            active: 0,
        };
        assert!(validate_layout_tree(&empty)
            .unwrap_err()
            .contains("at least one pane"));

        let out_of_range = LayoutNode::Stack {
            panes: vec![LayoutPane::default(), LayoutPane::default()],
            active: 2,
        };
        assert!(validate_layout_tree(&out_of_range)
            .unwrap_err()
            .contains("out of range"));

        let too_many = LayoutNode::Stack {
            panes: vec![LayoutPane::default(); MAX_LAYOUT_PANES + 1],
            active: 0,
        };
        assert!(validate_layout_tree(&too_many)
            .unwrap_err()
            .contains("more than"));
    }
}
