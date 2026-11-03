//! Updating the server and handing it off to a new binary: `update.status`,
//! `update.install`, `update.handoff`, `update.integrations` and `update.skip`.
//!
//! herdsman's own module. `api/schema.rs`, `api/schema/server.rs` and
//! `api/schema/response.rs` reach it through hooks marked
//! `// fork: update-handoff`.

use serde::{Deserialize, Serialize};

/// What the server knows about updating itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UpdateStatus {
    /// The running server's version.
    pub running: String,
    /// The version of the binary on disk, when it differs from `running`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed: Option<String>,
    /// The newest release on `channel`, when it is newer than both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<String>,
    /// `stable` or `preview`.
    pub channel: String,
    /// Whether this install can update itself (an install-script install).
    pub self_update: bool,
    /// How to update an install a package manager owns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_command: Option<String>,
    /// An install started by `update.install` is still running.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub installing: bool,
    /// Why the last install failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_error: Option<String>,
    /// Whether the server can hand off to the binary on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<UpdateHandoff>,
    /// Why the last handoff failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_error: Option<String>,
    /// Installed integrations older than this version expects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub integrations: Vec<OutdatedIntegration>,
    /// Present when `[update] auto_handoff` is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<UpdateAuto>,
    /// The release the user chose to skip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpdateHandoff {
    /// The binary on disk is newer and the server can move to it.
    Ready,
    /// The binary on disk is newer, but this server cannot hand off: restart it.
    Unsupported,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OutdatedIntegration {
    /// The agent, such as `claude` or `codex`.
    pub agent: String,
    /// The installed integration's version; absent for one without a version mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed: Option<u32>,
    pub expected: u32,
    /// The files an update writes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

/// The automatic mode's state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UpdateAuto {
    /// What the server waits for before acting on its own, such as
    /// `agents_busy` or `recent_input`; absent when nothing is pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_for: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UpdateSkipParams {
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UpdatedIntegration {
    pub agent: String,
    /// The files written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}
