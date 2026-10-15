//! Preset files in one folder, `<name>.toml` each. The file name is the
//! preset's name.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::file::{self, PresetFile};

const EXTENSION: &str = "toml";
const MAX_NAME_LEN: usize = 64;

/// Written above every saved preset, for whoever opens the file.
const HEADER: &str = "\
# A herdsman tab preset. [root] is the tab: a pane, a split (`split`, `ratio`,
# `first`, `second`) or a stack ([[...stack]] panes). A pane can have a `label`,
# a `cwd` relative to the workspace's folder, a `command` typed into its shell,
# and `env`. See the presets page of herdsman's documentation.

";

#[derive(Debug)]
pub(super) enum StoreError {
    InvalidName(String),
    NotFound(String),
    Exists(String),
    Invalid { name: String, message: String },
    Io(io::Error),
}

impl StoreError {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::InvalidName(_) => "invalid_preset_name",
            Self::NotFound(_) => "preset_not_found",
            Self::Exists(_) => "preset_exists",
            Self::Invalid { .. } => "invalid_preset",
            Self::Io(_) => "preset_io_failed",
        }
    }

    pub(super) fn message(&self) -> String {
        match self {
            Self::InvalidName(name) => format!(
                "{name:?} is not a preset name: use lowercase letters, digits, `-` and `_`, \
                 starting with a letter or digit, up to {MAX_NAME_LEN} characters"
            ),
            Self::NotFound(name) => format!("preset {name} not found"),
            Self::Exists(name) => format!("preset {name} already exists"),
            Self::Invalid { name, message } => format!("preset {name}: {message}"),
            Self::Io(err) => err.to_string(),
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// A preset's name and its contents, or why they could not be read.
pub(super) type Listed = (String, Result<PresetFile, String>);

pub(super) struct PresetStore {
    dir: PathBuf,
}

impl PresetStore {
    pub(super) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub(super) fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.{EXTENSION}"))
    }

    /// Every preset, sorted by name, each read or with the reason it could
    /// not be. A missing folder holds no presets.
    pub(super) fn list(&self) -> Result<Vec<Listed>, StoreError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };
        let mut presets = Vec::new();
        for entry in entries {
            let path = entry?.path();
            let Some(name) = preset_name(&path) else {
                continue;
            };
            let preset = fs::read_to_string(&path)
                .map_err(|err| err.to_string())
                .and_then(|text| file::parse(&text));
            presets.push((name, preset));
        }
        presets.sort_by(|(a, _), (b, _)| a.cmp(b));
        Ok(presets)
    }

    pub(super) fn read(&self, name: &str) -> Result<PresetFile, StoreError> {
        validate_name(name)?;
        let text = match fs::read_to_string(self.path(name)) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::NotFound(name.to_string()))
            }
            Err(err) => return Err(err.into()),
        };
        file::parse(&text).map_err(|message| StoreError::Invalid {
            name: name.to_string(),
            message,
        })
    }

    pub(super) fn write(
        &self,
        name: &str,
        preset: &PresetFile,
        overwrite: bool,
    ) -> Result<(), StoreError> {
        validate_name(name)?;
        let path = self.path(name);
        if !overwrite && path.exists() {
            return Err(StoreError::Exists(name.to_string()));
        }
        let text = file::render(preset).map_err(|message| StoreError::Invalid {
            name: name.to_string(),
            message,
        })?;
        fs::create_dir_all(&self.dir)?;
        // Write beside the target and rename over it, so a reader never sees
        // half a file.
        let partial = self.dir.join(format!(".{name}.{EXTENSION}.partial"));
        fs::write(&partial, format!("{HEADER}{text}"))?;
        fs::rename(&partial, &path).inspect_err(|_| {
            let _ = fs::remove_file(&partial);
        })?;
        Ok(())
    }

    pub(super) fn rename(&self, name: &str, new_name: &str) -> Result<(), StoreError> {
        validate_name(name)?;
        validate_name(new_name)?;
        let from = self.path(name);
        if !from.exists() {
            return Err(StoreError::NotFound(name.to_string()));
        }
        let to = self.path(new_name);
        if name != new_name && to.exists() {
            return Err(StoreError::Exists(new_name.to_string()));
        }
        fs::rename(from, to)?;
        Ok(())
    }

    pub(super) fn delete(&self, name: &str) -> Result<(), StoreError> {
        validate_name(name)?;
        match fs::remove_file(self.path(name)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Err(StoreError::NotFound(name.to_string()))
            }
            Err(err) => Err(err.into()),
        }
    }
}

fn validate_name(name: &str) -> Result<(), StoreError> {
    let valid = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidName(name.to_string()))
    }
}

/// The preset a path in the folder holds, if it is one: `<name>.toml` with a
/// valid name. Partial writes and other files are skipped.
fn preset_name(path: &Path) -> Option<String> {
    if path.extension()? != EXTENSION {
        return None;
    }
    let name = path.file_stem()?.to_str()?;
    validate_name(name).ok()?;
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{PresetNode, PresetPane};

    fn preset(command: &str) -> PresetFile {
        PresetFile {
            label: None,
            root: PresetNode::Pane {
                pane: PresetPane {
                    command: Some(command.into()),
                    ..PresetPane::default()
                },
            },
        }
    }

    /// A folder under the temp folder, removed when the test ends.
    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn store() -> (Scratch, PresetStore) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "herdsman-preset-store-{}-{stamp}",
            std::process::id()
        ));
        let store = PresetStore::new(dir.join("presets"));
        (Scratch(dir), store)
    }

    #[test]
    fn a_missing_folder_lists_nothing() {
        let (_dir, store) = store();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn write_read_and_list() {
        let (_dir, store) = store();
        store.write("dev", &preset("claude"), false).unwrap();
        store.write("api-2", &preset("just test"), false).unwrap();

        assert_eq!(store.read("dev").unwrap(), preset("claude"));
        let text = fs::read_to_string(store.path("dev")).unwrap();
        assert!(text.starts_with("# A herdsman tab preset."), "{text}");

        let names: Vec<_> = store
            .list()
            .unwrap()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, ["api-2", "dev"]);
    }

    #[test]
    fn write_refuses_an_existing_name_unless_overwriting() {
        let (_dir, store) = store();
        store.write("dev", &preset("claude"), false).unwrap();

        let err = store.write("dev", &preset("codex"), false).unwrap_err();
        assert_eq!(err.code(), "preset_exists");
        store.write("dev", &preset("codex"), true).unwrap();
        assert_eq!(store.read("dev").unwrap(), preset("codex"));
    }

    #[test]
    fn list_reports_a_broken_file_and_skips_other_files() {
        let (_dir, store) = store();
        store.write("good", &preset("claude"), false).unwrap();
        fs::write(store.path("broken"), "[root]\nsplit = \"left\"\n").unwrap();
        fs::write(store.dir.join("notes.txt"), "").unwrap();
        fs::write(store.dir.join("Bad Name.toml"), "[root]\n").unwrap();
        fs::write(store.dir.join(".good.toml.partial"), "").unwrap();

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].0, "broken");
        assert!(listed[0].1.as_ref().unwrap_err().contains("left"));
        assert!(listed[1].1.is_ok());

        let err = store.read("broken").unwrap_err();
        assert_eq!(err.code(), "invalid_preset");
        assert!(
            err.message().starts_with("preset broken: "),
            "{}",
            err.message()
        );
    }

    #[test]
    fn rename_and_delete() {
        let (_dir, store) = store();
        store.write("old", &preset("claude"), false).unwrap();
        store.write("taken", &preset("codex"), false).unwrap();

        assert_eq!(
            store.rename("old", "taken").unwrap_err().code(),
            "preset_exists"
        );
        assert_eq!(
            store.rename("gone", "new").unwrap_err().code(),
            "preset_not_found"
        );
        store.rename("old", "new").unwrap();
        assert_eq!(store.read("new").unwrap(), preset("claude"));
        assert_eq!(store.read("old").unwrap_err().code(), "preset_not_found");

        store.delete("new").unwrap();
        assert_eq!(store.delete("new").unwrap_err().code(), "preset_not_found");
    }

    #[test]
    fn names_are_lowercase_words() {
        for good in ["dev", "a", "api-2", "my_tab", "0day"] {
            assert!(validate_name(good).is_ok(), "{good}");
        }
        let long = "a".repeat(MAX_NAME_LEN + 1);
        for bad in [
            "",
            "Dev",
            "-dev",
            "_dev",
            "a b",
            "a/b",
            "../x",
            "dev.toml",
            long.as_str(),
        ] {
            assert_eq!(
                validate_name(bad).unwrap_err().code(),
                "invalid_preset_name",
                "{bad}"
            );
        }
    }
}
