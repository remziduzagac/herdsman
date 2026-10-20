//! API types for tab presets: a tab's layout saved under a name and opened
//! again as a new tab.
//!
//! herdsman's own module. `panes.rs` re-exports it through hooks marked
//! `// fork: presets`.

use std::collections::BTreeMap;

use super::*;

/// Save a tab as a preset. `tab_id`, or `pane_id` for the tab holding that
/// pane, picks the tab; without either, the active tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetSaveParams {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
    /// Replace a preset that already has this name.
    #[serde(default)]
    pub overwrite: bool,
}

/// Open a preset as a new tab in `workspace_id`, or in the workspace holding
/// `pane_id`; without either, in the active workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetLoadParams {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
    /// Focus the new tab. Defaults to true.
    #[serde(default = "focus_by_default")]
    pub focus: bool,
    /// Values for the preset's declared parameters, by name. Every pane gets
    /// each one as an environment variable named in upper case: `pr` as `PR`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

fn focus_by_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetNameParams {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetRenameParams {
    pub name: String,
    pub new_name: String,
}

/// One preset in `preset.list`. A file that does not parse is listed with its
/// `error`, so it can be fixed or deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetSummary {
    pub name: String,
    /// The preset's file on the server's machine.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub panes: usize,
    /// The parameters it takes when it is opened.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<PresetParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetDescription {
    pub name: String,
    /// The new tab's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The preset's file on the server's machine.
    pub path: String,
    /// The parameters it takes when it is opened.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<PresetParam>,
    pub root: PresetNode,
}

/// A value a preset takes when it is opened, declared in its file's `[params]`
/// table. Opening the preset gives every pane the value as an environment
/// variable, `name` in upper case, so a pane's command reads it as `$NAME`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PresetParam {
    pub name: String,
    /// Opening the preset without a value fails.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    /// The value when none is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A preset's layout: `split` and `stack` nodes as in `layout.export`, with
/// panes that hold what to start rather than what runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PresetNode {
    Pane {
        #[serde(flatten)]
        pane: PresetPane,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<PresetNode>,
        second: Box<PresetNode>,
    },
    /// Panes sharing one slot, with `panes[active]` visible.
    Stack {
        panes: Vec<PresetPane>,
        #[serde(default)]
        active: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PresetPane {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// A folder relative to the workspace's folder, or an absolute one.
    /// Without it, the pane starts in the workspace's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// A command line typed into the pane's shell once it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

impl PresetNode {
    pub fn pane_count(&self) -> usize {
        match self {
            Self::Pane { .. } => 1,
            Self::Split { first, second, .. } => first.pane_count() + second.pane_count(),
            Self::Stack { panes, .. } => panes.len(),
        }
    }
}
