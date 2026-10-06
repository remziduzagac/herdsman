//! Named groups of linked worktrees, kept by checkout path so a worktree keeps
//! its group when its workspace closes and opens again.
//!
//! herdsman's own module. The groups are saved beside the session, in
//! `worktree-groups.json`, so herdr's session format stays untouched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tracing::warn;

/// The longest group name, in characters.
pub(crate) const MAX_GROUP_NAME_CHARS: usize = 64;

#[derive(Debug, Default)]
pub(crate) struct WorktreeGroups {
    /// Checkout path to group name.
    groups: BTreeMap<String, String>,
    /// Where changes are saved; None when this server does not save its session.
    path: Option<PathBuf>,
}

impl WorktreeGroups {
    /// The groups saved at `path`, when `restore` is set, saving changes back
    /// there when `persist` is set.
    pub(crate) fn open(path: PathBuf, restore: bool, persist: bool) -> Self {
        let groups = if restore {
            match std::fs::read_to_string(&path) {
                Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
                    warn!(path = %path.display(), err = %err, "ignoring unreadable worktree groups");
                    BTreeMap::new()
                }),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
                Err(err) => {
                    warn!(path = %path.display(), err = %err, "could not read worktree groups");
                    BTreeMap::new()
                }
            }
        } else {
            BTreeMap::new()
        };
        Self {
            groups,
            path: persist.then_some(path),
        }
    }

    /// The group of the linked worktree checked out at `checkout`.
    pub(crate) fn group(&self, checkout: &Path) -> Option<&str> {
        self.groups.get(&key(checkout)).map(String::as_str)
    }

    /// Put the worktree at `checkout` into `group`, or take it out of its
    /// group with None. True when that changed anything.
    pub(crate) fn set(&mut self, checkout: &Path, group: Option<String>) -> bool {
        let key = key(checkout);
        let changed = match group {
            Some(group) => self.groups.insert(key, group.clone()).as_deref() != Some(&group),
            None => self.groups.remove(&key).is_some(),
        };
        if changed {
            self.save();
        }
        changed
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let result = (|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(&self.groups)?)?;
            std::fs::rename(&tmp, path)
        })();
        if let Err(err) = result {
            warn!(path = %path.display(), err = %err, "could not save worktree groups");
        }
    }
}

fn key(checkout: &Path) -> String {
    checkout.display().to_string()
}

/// A group name as given over the API: trimmed, None when empty.
pub(crate) fn normalize_group_name(name: Option<String>) -> Result<Option<String>, String> {
    let Some(name) = name.map(|name| name.trim().to_owned()) else {
        return Ok(None);
    };
    if name.is_empty() {
        return Ok(None);
    }
    if name.chars().count() > MAX_GROUP_NAME_CHARS {
        return Err(format!(
            "a worktree group name is at most {MAX_GROUP_NAME_CHARS} characters"
        ));
    }
    if name.chars().any(char::is_control) {
        return Err("a worktree group name cannot contain control characters".into());
    }
    Ok(Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hs-worktree-groups-{name}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ))
    }

    #[test]
    fn groups_are_saved_and_restored_by_checkout_path() {
        let path = temp_path("roundtrip");
        let checkout = Path::new("/work/repo-feature");
        let mut groups = WorktreeGroups::open(path.clone(), true, true);
        assert!(groups.set(checkout, Some("features".into())));
        assert!(!groups.set(checkout, Some("features".into())));

        let restored = WorktreeGroups::open(path.clone(), true, true);
        assert_eq!(restored.group(checkout), Some("features"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn leaving_a_group_forgets_it_and_unsaved_groups_touch_no_file() {
        let path = temp_path("unsaved");
        let checkout = Path::new("/work/repo-fix");
        let mut groups = WorktreeGroups::open(path.clone(), false, false);
        assert!(groups.set(checkout, Some("fixes".into())));
        assert!(groups.set(checkout, None));
        assert!(!groups.set(checkout, None));
        assert_eq!(groups.group(checkout), None);
        assert!(!path.exists());
    }

    #[test]
    fn group_names_are_trimmed_and_bounded() {
        assert_eq!(
            normalize_group_name(Some("  merges ".into())),
            Ok(Some("merges".into()))
        );
        assert_eq!(normalize_group_name(Some("   ".into())), Ok(None));
        assert_eq!(normalize_group_name(None), Ok(None));
        assert!(normalize_group_name(Some("x".repeat(MAX_GROUP_NAME_CHARS + 1))).is_err());
        assert!(normalize_group_name(Some("a\tb".into())).is_err());
    }
}
