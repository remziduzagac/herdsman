//! The `[update.auto]` config table: whether the server updates and hands
//! itself off on its own, and what it does about outdated integrations then.
//!
//! ```toml
//! [update.auto]
//! handoff = true
//! integrations = "update"
//! ```
//!
//! herdsman's own module. `config.rs` and `config/model.rs` reach it through
//! hooks marked `// fork: update-handoff`.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AutoUpdateConfig {
    /// Install new releases and hand the server off to a newer binary on its
    /// own, at a quiet moment.
    pub handoff: bool,
    /// What automatic mode does about installed integrations that are out of
    /// date.
    pub integrations: AutoIntegrations,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoIntegrations {
    /// Update them and say which files changed.
    #[default]
    Update,
    /// Only say they need updating.
    Notify,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_mode_is_off_and_updates_integrations_when_on() {
        let config: AutoUpdateConfig = toml::from_str("").expect("parses");
        assert_eq!(config, AutoUpdateConfig::default());
        assert!(!config.handoff);
        let config: AutoUpdateConfig =
            toml::from_str("handoff = true\nintegrations = \"notify\"").expect("parses");
        assert!(config.handoff);
        assert_eq!(config.integrations, AutoIntegrations::Notify);
        assert!(toml::from_str::<AutoUpdateConfig>("integrations = \"always\"").is_err());
    }
}
