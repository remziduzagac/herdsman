//! `layout.equalize`: equal widths or equal heights for a tab's panes.
//!
//! herdsman's own module. `layouts.rs` and `app/api.rs` reach it through hooks
//! marked `// fork: equalize`.

use super::*;
use crate::api::schema::{LayoutDimension, LayoutEqualizeParams};

impl App {
    pub(in crate::app::api) fn handle_layout_equalize(
        &mut self,
        id: String,
        params: LayoutEqualizeParams,
    ) -> String {
        let Some((ws_idx, tab_idx)) = self.resolve_layout_export_target(&LayoutExportParams {
            tab_id: params.tab_id,
            pane_id: params.pane_id,
        }) else {
            return encode_error(id, "layout_not_found", "layout target not found");
        };
        let direction = match params.dimension {
            LayoutDimension::Width => Direction::Horizontal,
            LayoutDimension::Height => Direction::Vertical,
        };

        let Some(changed) = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .and_then(|ws| ws.tabs.get_mut(tab_idx))
            .map(|tab| tab.layout.equalize(direction))
        else {
            return encode_error(id, "layout_not_found", "layout unavailable");
        };

        if changed {
            self.schedule_session_save();
        }
        let Some(layout) = self.layout_description(ws_idx, tab_idx) else {
            return encode_error(id, "layout_not_found", "layout unavailable");
        };
        if changed {
            self.emit_layout_updated_event(ws_idx, tab_idx);
        }
        encode_success(id, ResponseResult::LayoutEqualized { layout })
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::app_with_workspace;
    use super::*;
    use crate::api::schema::{ErrorResponse, SuccessResponse};

    fn equalize(app: &mut App, tab_id: Option<String>, dimension: LayoutDimension) -> String {
        app.handle_layout_equalize(
            "req".into(),
            LayoutEqualizeParams {
                tab_id,
                pane_id: None,
                dimension,
            },
        )
    }

    fn ratios(node: &LayoutNode, out: &mut Vec<(SplitDirection, f32)>) {
        if let LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } = node
        {
            out.push((direction.clone(), *ratio));
            ratios(first, out);
            ratios(second, out);
        }
    }

    #[test]
    fn equalize_width_evens_columns_and_reports_the_layout() {
        let mut app = app_with_workspace();
        let workspace = &mut app.state.workspaces[0];
        workspace.test_split(Direction::Horizontal);
        workspace.test_split(Direction::Horizontal);
        workspace.test_split(Direction::Vertical);
        let tab_id = app.public_tab_id(0, 0);

        let response = equalize(&mut app, tab_id.clone(), LayoutDimension::Width);

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::LayoutEqualized { layout } = success.result else {
            panic!("expected layout equalized response");
        };
        let mut splits = Vec::new();
        ratios(&layout.root, &mut splits);
        let widths: Vec<_> = splits
            .iter()
            .filter(|(direction, _)| *direction == SplitDirection::Right)
            .map(|(_, ratio)| *ratio)
            .collect();
        assert_eq!(widths, [1.0 / 3.0, 0.5]);
        assert!(splits.contains(&(SplitDirection::Down, 0.5)));
        assert!(matches!(
            &app.event_hub.events_after(0).last().expect("layout event").1.data,
            EventData::LayoutUpdated { layout } if Some(&layout.tab_id) == tab_id.as_ref()
        ));
    }

    #[test]
    fn equalize_without_a_change_emits_no_layout_event() {
        let mut app = app_with_workspace();
        app.state.workspaces[0].test_split(Direction::Horizontal);
        let events_before = app.event_hub.events_after(0).len();

        let response = equalize(&mut app, None, LayoutDimension::Width);

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert!(matches!(
            success.result,
            ResponseResult::LayoutEqualized { .. }
        ));
        assert_eq!(app.event_hub.events_after(0).len(), events_before);
    }

    #[test]
    fn equalize_rejects_an_unknown_tab() {
        let mut app = app_with_workspace();

        let response = equalize(&mut app, Some("w9:t9".into()), LayoutDimension::Height);

        let error: ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(error.error.code, "layout_not_found");
    }
}
