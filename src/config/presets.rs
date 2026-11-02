//! The `[presets]` config section: which tab preset a new workspace starts
//! from, for any workspace, for any worktree, and per repository.
//!
//! ```toml
//! [presets]
//! workspace = "dev"
//! worktree = "worktree"
//!
//! [presets.projects.herdsman]
//! workspace = "herdsman"
//! worktree = "herdsman-worktree"
//! ```
//!
//! herdsman's own module. `config.rs`, `config/model.rs` and `config/io.rs`
//! reach it through hooks marked `// fork: presets`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::api::schema::PresetDefaultKind;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct PresetsConfig {
    /// The preset for a new workspace.
    pub workspace: Option<String>,
    /// The preset for a new or newly opened linked worktree.
    pub worktree: Option<String>,
    /// Per repository, by the name the sidebar gives it: its main checkout's
    /// folder name, shared by all its worktrees.
    pub projects: BTreeMap<String, ProjectPresets>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ProjectPresets {
    pub workspace: Option<String>,
    pub worktree: Option<String>,
}

impl PresetsConfig {
    /// The preset a new workspace of `kind` starts from, in the repository
    /// named `repo` when it is in one. The repository's own setting wins, then
    /// the general one; a worktree never falls back to the workspace setting.
    pub fn default_for(&self, kind: PresetDefaultKind, repo: Option<&str>) -> Option<&str> {
        let project = repo.and_then(|repo| self.projects.get(repo));
        let (own, general) = match kind {
            PresetDefaultKind::Workspace => (
                project.and_then(|project| project.workspace.as_deref()),
                self.workspace.as_deref(),
            ),
            PresetDefaultKind::Worktree => (
                project.and_then(|project| project.worktree.as_deref()),
                self.worktree.as_deref(),
            ),
        };
        own.or(general).filter(|name| !name.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PresetsConfig {
        toml::from_str(
            r#"
            workspace = "dev"
            worktree = "worktree"

            [projects.herdsman]
            worktree = "herdsman-worktree"

            [projects.notes]
            workspace = "writing"
            "#,
        )
        .expect("presets config parses")
    }

    #[test]
    fn a_repositorys_own_setting_wins_over_the_general_one() {
        let config = config();
        assert_eq!(
            config.default_for(PresetDefaultKind::Worktree, Some("herdsman")),
            Some("herdsman-worktree")
        );
        assert_eq!(
            config.default_for(PresetDefaultKind::Workspace, Some("notes")),
            Some("writing")
        );
    }

    #[test]
    fn without_a_repository_setting_the_general_one_applies() {
        let config = config();
        assert_eq!(
            config.default_for(PresetDefaultKind::Workspace, Some("herdsman")),
            Some("dev")
        );
        assert_eq!(
            config.default_for(PresetDefaultKind::Worktree, Some("notes")),
            Some("worktree")
        );
        assert_eq!(
            config.default_for(PresetDefaultKind::Workspace, None),
            Some("dev")
        );
    }

    #[test]
    fn a_worktree_does_not_fall_back_to_the_workspace_setting() {
        let config = PresetsConfig {
            workspace: Some("dev".into()),
            ..Default::default()
        };
        assert_eq!(config.default_for(PresetDefaultKind::Worktree, None), None);
        assert_eq!(
            PresetsConfig::default().default_for(PresetDefaultKind::Workspace, None),
            None
        );
    }

    #[test]
    fn a_blank_name_means_no_preset() {
        let config: PresetsConfig = toml::from_str(r#"workspace = " ""#).expect("parses");
        assert_eq!(config.default_for(PresetDefaultKind::Workspace, None), None);
    }
}
