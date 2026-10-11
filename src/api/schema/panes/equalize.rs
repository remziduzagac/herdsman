//! API types for equal pane widths and heights.
//!
//! herdsman's own module. `panes.rs` re-exports it through hooks marked
//! `// fork: equalize`.

use super::*;

/// Give a tab's panes equal widths or equal heights. `tab_id` or `pane_id`
/// picks the tab; without either, the active tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LayoutEqualizeParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
    pub dimension: LayoutDimension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDimension {
    /// Every column the same width; heights keep their ratios.
    Width,
    /// Every row the same height; widths keep their ratios.
    Height,
}
