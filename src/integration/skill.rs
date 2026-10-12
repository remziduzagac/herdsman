//! herdsman's agent skill, installed with an agent's integration.
//!
//! `herdsman integration install <agent>` writes the skill this binary embeds,
//! the one `herdsman --skill` prints, where the agent reads user skills, and
//! `uninstall` removes it. After `herdsman update`, the new binary rewrites the
//! copies it finds, so an installed skill always describes the CLI beside it.
//! A skill herdsman did not write stays as it is: a link, such as one into a
//! dotfiles checkout, or a file whose front matter names another skill.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::api::schema::IntegrationTarget;

const SKILL: &str = include_str!("../../skills/herdsman/SKILL.md");
const SKILL_NAME: &str = "herdsman";
const SKILL_FILE: &str = "SKILL.md";

/// The agents whose integration brings the skill.
const SKILL_TARGETS: [IntegrationTarget; 3] = [
    IntegrationTarget::Claude,
    IntegrationTarget::Codex,
    IntegrationTarget::Opencode,
];

/// The folder holding herdsman's skill for `target`'s agent, or `None` for an
/// agent that does not get the skill.
fn skill_dir(target: IntegrationTarget) -> io::Result<Option<PathBuf>> {
    let config_dir = match target {
        IntegrationTarget::Claude => super::env::claude_dir()?,
        IntegrationTarget::Codex => super::env::codex_dir()?,
        IntegrationTarget::Opencode => super::env::opencode_dir()?,
        _ => return Ok(None),
    };
    Ok(Some(config_dir.join("skills").join(SKILL_NAME)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Found {
    Missing,
    /// The skill's folder or file is a link; whoever made it keeps it current.
    Link,
    /// A file whose front matter names another skill.
    Other,
    /// herdsman's skill; `current` when it matches this binary's.
    Herdsman {
        current: bool,
    },
}

fn inspect(dir: &Path) -> io::Result<Found> {
    let file = dir.join(SKILL_FILE);
    for path in [dir, file.as_path()] {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(Found::Link),
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Found::Missing),
            Err(err) => return Err(err),
        }
    }
    let content = fs::read_to_string(&file)?;
    Ok(if names_herdsman(&content) {
        Found::Herdsman {
            current: content == SKILL,
        }
    } else {
        Found::Other
    })
}

/// Whether `content`'s front matter gives the skill herdsman's name.
fn names_herdsman(content: &str) -> bool {
    let mut lines = content.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return false;
    }
    lines
        .take_while(|line| line.trim_end() != "---")
        .filter_map(|line| line.strip_prefix("name:"))
        .any(|name| name.trim().trim_matches(['"', '\'']) == SKILL_NAME)
}

fn write(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join(SKILL_FILE), SKILL)
}

fn left_unchanged(found: Found, file: &Path) -> String {
    match found {
        Found::Link => format!("left linked herdsman skill unchanged at {}", file.display()),
        _ => format!("left {} unchanged: it is another skill", file.display()),
    }
}

fn install_in(dir: &Path) -> io::Result<String> {
    let file = dir.join(SKILL_FILE);
    match inspect(dir)? {
        Found::Missing | Found::Herdsman { .. } => {
            write(dir)?;
            Ok(format!("installed herdsman skill to {}", file.display()))
        }
        found => Ok(left_unchanged(found, &file)),
    }
}

fn uninstall_in(dir: &Path) -> io::Result<String> {
    let file = dir.join(SKILL_FILE);
    match inspect(dir)? {
        Found::Herdsman { .. } => {
            fs::remove_file(&file)?;
            // Removes the folder only when empty; anything else put there stays.
            let _ = fs::remove_dir(dir);
            Ok(format!("removed herdsman skill at {}", file.display()))
        }
        Found::Missing => Ok(format!("no herdsman skill found at {}", file.display())),
        found => Ok(left_unchanged(found, &file)),
    }
}

/// Rewrites the skill in `dir` when it is an older herdsman skill.
fn refresh_in(dir: &Path) -> io::Result<Option<String>> {
    if inspect(dir)? != (Found::Herdsman { current: false }) {
        return Ok(None);
    }
    write(dir)?;
    Ok(Some(format!(
        "updated herdsman skill at {}",
        dir.join(SKILL_FILE).display()
    )))
}

fn warning(file: &Path, err: &io::Error) -> String {
    format!(
        "{} herdsman skill at {}: {err}",
        super::INSTALL_WARNING_PREFIX,
        file.display()
    )
}

/// Runs `action` on `target`'s skill folder. A failure becomes a warning
/// line, since the integration works without the skill.
fn for_target(
    target: IntegrationTarget,
    action: fn(&Path) -> io::Result<String>,
) -> Option<String> {
    let dir = skill_dir(target).transpose()?;
    Some(match dir {
        Ok(dir) => action(&dir).unwrap_or_else(|err| warning(&dir.join(SKILL_FILE), &err)),
        Err(err) => format!("{} herdsman skill: {err}", super::INSTALL_WARNING_PREFIX),
    })
}

/// Installs the skill for `target`'s agent, once its integration is in place.
pub(super) fn install(target: IntegrationTarget) -> Option<String> {
    for_target(target, install_in)
}

/// Removes the skill for `target`'s agent with its integration.
pub(super) fn uninstall(target: IntegrationTarget) -> Option<String> {
    for_target(target, uninstall_in)
}

/// Rewrites every herdsman skill older than this binary's. `herdsman update`
/// runs the binary it installed with `integration status --outdated-only`,
/// which calls this, so the update brings the skill along.
pub(crate) fn refresh_after_update() {
    for target in SKILL_TARGETS {
        let Ok(Some(dir)) = skill_dir(target) else {
            continue;
        };
        match refresh_in(&dir) {
            Ok(Some(message)) => eprintln!("{message}"),
            Ok(None) => {}
            Err(err) => eprintln!("{}", warning(&dir.join(SKILL_FILE), &err)),
        }
    }
}

fn describe(found: Found) -> &'static str {
    match found {
        Found::Missing => "not installed",
        Found::Link => "linked, left unchanged",
        Found::Other => "another skill, left unchanged",
        Found::Herdsman { current: true } => "current",
        Found::Herdsman { current: false } => "outdated",
    }
}

/// Prints `herdsman integration status`'s skill lines: one for each agent
/// whose integration is installed, and one for any other skill folder found.
pub(crate) fn print_statuses() {
    let installed: Vec<IntegrationTarget> = super::registry::installed_integration_statuses()
        .into_iter()
        .filter(|status| status.state != super::IntegrationStatusKind::NotInstalled)
        .map(|status| status.target)
        .collect();
    for target in SKILL_TARGETS {
        let Ok(Some(dir)) = skill_dir(target) else {
            continue;
        };
        let state = match inspect(&dir) {
            Ok(Found::Missing) if !installed.contains(&target) => continue,
            Ok(found) => describe(found).to_string(),
            Err(err) => format!("unreadable: {err}"),
        };
        println!(
            "{} skill: {state} ({})",
            super::integration_target_label(target),
            dir.join(SKILL_FILE).display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLDER: &str = "---\nname: herdsman\ndescription: an older copy\n---\n\n# Herdsman\n";
    const ANOTHER: &str = "---\nname: herder\ndescription: not ours\n---\n\nname: herdsman\n";

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "herdsman-skill-test-{}-{name}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(SKILL_FILE), content).unwrap();
    }

    fn read(dir: &Path) -> String {
        fs::read_to_string(dir.join(SKILL_FILE)).unwrap()
    }

    #[test]
    fn the_name_comes_from_the_front_matter_only() {
        assert!(names_herdsman(SKILL));
        assert!(names_herdsman(OLDER));
        assert!(names_herdsman("---\r\nname: \"herdsman\"\r\n---\r\n"));
        assert!(!names_herdsman(ANOTHER));
        assert!(!names_herdsman("# Herdsman\n\nname: herdsman\n"));
        assert!(!names_herdsman(""));
    }

    #[test]
    fn install_writes_the_embedded_skill_and_replaces_an_older_one() {
        let base = scratch("install");
        let dir = base.join("skills").join(SKILL_NAME);

        let message = install_in(&dir).unwrap();
        assert!(
            message.starts_with("installed herdsman skill to"),
            "{message}"
        );
        assert_eq!(read(&dir), SKILL);
        assert_eq!(inspect(&dir).unwrap(), Found::Herdsman { current: true });

        put(&dir, OLDER);
        assert_eq!(inspect(&dir).unwrap(), Found::Herdsman { current: false });
        install_in(&dir).unwrap();
        assert_eq!(read(&dir), SKILL);

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn install_and_uninstall_leave_another_skill_alone() {
        let base = scratch("another");
        let dir = base.join(SKILL_NAME);
        put(&dir, ANOTHER);

        assert_eq!(inspect(&dir).unwrap(), Found::Other);
        assert!(install_in(&dir).unwrap().ends_with("it is another skill"));
        assert!(uninstall_in(&dir).unwrap().ends_with("it is another skill"));
        assert_eq!(refresh_in(&dir).unwrap(), None);
        assert_eq!(read(&dir), ANOTHER);

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn a_linked_skill_file_is_left_to_whoever_linked_it() {
        let base = scratch("linked-file");
        let source = base.join("dotfiles");
        put(&source, OLDER);
        let dir = base.join(SKILL_NAME);
        fs::create_dir_all(&dir).unwrap();
        if !super::super::test_support::symlink_file(
            &source.join(SKILL_FILE),
            &dir.join(SKILL_FILE),
        ) {
            return;
        }

        assert_eq!(inspect(&dir).unwrap(), Found::Link);
        assert!(install_in(&dir).unwrap().starts_with("left linked"));
        assert_eq!(refresh_in(&dir).unwrap(), None);
        assert!(uninstall_in(&dir).unwrap().starts_with("left linked"));
        assert_eq!(read(&source), OLDER);
        assert_eq!(read(&dir), OLDER);

        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_skill_folder_is_left_to_whoever_linked_it() {
        let base = scratch("linked-dir");
        let checkout = base.join("checkout");
        put(&checkout, OLDER);
        let dir = base.join(SKILL_NAME);
        std::os::unix::fs::symlink(&checkout, &dir).unwrap();

        assert_eq!(inspect(&dir).unwrap(), Found::Link);
        install_in(&dir).unwrap();
        refresh_in(&dir).unwrap();
        uninstall_in(&dir).unwrap();
        assert_eq!(read(&checkout), OLDER);

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn uninstall_removes_the_skill_and_only_an_empty_folder() {
        let base = scratch("uninstall");
        let dir = base.join(SKILL_NAME);
        put(&dir, OLDER);

        assert!(uninstall_in(&dir)
            .unwrap()
            .starts_with("removed herdsman skill"));
        assert!(!dir.exists());
        assert!(uninstall_in(&dir)
            .unwrap()
            .starts_with("no herdsman skill found"));

        put(&dir, SKILL);
        fs::write(dir.join("notes.md"), "mine").unwrap();
        uninstall_in(&dir).unwrap();
        assert!(!dir.join(SKILL_FILE).exists());
        assert!(dir.join("notes.md").exists());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn refresh_rewrites_only_an_older_herdsman_skill() {
        let base = scratch("refresh");
        let older = base.join("older");
        let current = base.join("current");
        let missing = base.join("missing");
        put(&older, OLDER);
        put(&current, SKILL);

        assert!(refresh_in(&older)
            .unwrap()
            .is_some_and(|message| message.starts_with("updated herdsman skill")));
        assert_eq!(read(&older), SKILL);
        assert_eq!(refresh_in(&current).unwrap(), None);
        assert_eq!(refresh_in(&missing).unwrap(), None);
        assert!(!missing.exists());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn the_skill_goes_to_each_agents_skills_folder() {
        let _lock = super::super::integration_env_lock();
        let base = scratch("targets");
        std::env::set_var(
            super::super::env::CLAUDE_CONFIG_DIR_ENV_VAR,
            base.join("claude"),
        );
        std::env::set_var(super::super::env::CODEX_HOME_ENV_VAR, base.join("codex"));

        assert_eq!(
            skill_dir(IntegrationTarget::Claude).unwrap(),
            Some(base.join("claude").join("skills").join(SKILL_NAME))
        );
        assert_eq!(
            skill_dir(IntegrationTarget::Codex).unwrap(),
            Some(base.join("codex").join("skills").join(SKILL_NAME))
        );
        assert!(skill_dir(IntegrationTarget::Opencode)
            .unwrap()
            .is_some_and(|dir| dir.ends_with(".config/opencode/skills/herdsman")));
        assert_eq!(skill_dir(IntegrationTarget::Pi).unwrap(), None);

        std::env::remove_var(super::super::env::CLAUDE_CONFIG_DIR_ENV_VAR);
        std::env::remove_var(super::super::env::CODEX_HOME_ENV_VAR);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn the_claude_integration_installs_and_removes_the_skill() {
        let _lock = super::super::integration_env_lock();
        let base = scratch("claude");
        let claude_dir = base.join("claude");
        fs::create_dir_all(&claude_dir).unwrap();
        std::env::set_var(super::super::env::CLAUDE_CONFIG_DIR_ENV_VAR, &claude_dir);
        let dir = claude_dir.join("skills").join(SKILL_NAME);

        let installed = super::super::install_target(IntegrationTarget::Claude).unwrap();
        assert!(installed
            .iter()
            .any(|message| message.starts_with("installed herdsman skill to")));
        assert_eq!(read(&dir), SKILL);

        let removed = super::super::uninstall_target(IntegrationTarget::Claude).unwrap();
        assert!(removed
            .iter()
            .any(|message| message.starts_with("removed herdsman skill at")));
        assert!(!dir.exists());

        std::env::remove_var(super::super::env::CLAUDE_CONFIG_DIR_ENV_VAR);
        let _ = fs::remove_dir_all(base);
    }
}
