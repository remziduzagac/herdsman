//! API types for stacked panes.
//!
//! herdsman's own module. `panes.rs` re-exports it through hooks marked
//! `// fork: stacks`.

use super::*;

/// Open a new pane in the target pane's slot, stacked with it rather than split
/// beside it. Without `focus` the new member starts hidden behind the visible one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PaneStackParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub focus: bool,
    #[serde(default)]
    pub right_click: PaneRightClickTarget,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PaneStacksParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

/// One stack: its members in strip order and the one currently visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaneStackInfo {
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_ids: Vec<String>,
    pub visible_pane_id: String,
}

/// Show and focus a member of the stack holding `pane_id`, or of the focused
/// pane's stack: member `index` (zero-based), or the member `step` places from
/// the visible one, wrapping. Pass exactly one. A pane outside a stack or an
/// index past its last member leaves focus unchanged, as a missing tab does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PaneFocusStackedParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<i32>,
}

/// Focus the stack member drawn at a point of the strip beside `pane_id`'s
/// content: `column` cells from the content's left edge, in a strip `width`
/// cells wide, on the row just above (`top`) or below (`bottom`) the content.
/// Any point that is not on a strip focuses `pane_id` itself, like `pane.focus`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaneFocusStackedAtParams {
    pub pane_id: String,
    pub column: u16,
    pub width: u16,
    pub edge: PaneContentEdge,
}

/// Focus the stack member drawn at a point of the strip beside `pane_id`'s
/// content: `column` cells from the content's left edge, in a strip `width`
/// cells wide, `offset` rows above (`top`) or below (`bottom`) the content, 1
/// being the row next to it. The strip sits at offset 1, or at 2 when a
/// separator line runs between it and the content. Any other point focuses
/// `pane_id` itself, like `pane.focus`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaneFocusStackStripParams {
    pub pane_id: String,
    pub column: u16,
    pub width: u16,
    pub edge: PaneContentEdge,
    pub offset: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaneContentEdge {
    Top,
    Bottom,
}
