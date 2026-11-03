//! What a running server needs from the updater: whether this install can
//! update itself, installing the newest release while the server keeps
//! running, and what the binary on disk is, so the server can hand off to it.
//!
//! herdsman's own module. `update.rs` reaches it through a hook marked
//! `// fork: update-handoff`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;

/// Why this install cannot update itself, or None when it can. The server
/// runs outside any pane, so being inside herdsman is no reason here.
pub(crate) fn self_update_refusal() -> Option<String> {
    if cfg!(windows) {
        return Some("updating from a running server is not supported on Windows".into());
    }
    let managed = is_homebrew_managed_install()
        || is_mise_managed_install()
        || is_nix_managed_install()
        || system_packages::is_system_package_install();
    managed.then(|| {
        format!(
            "this install is updated with `{}`",
            update_install_command()
        )
    })
}

/// The configured update channel, `stable` or `preview`.
pub(crate) fn configured_channel() -> &'static str {
    UpdateChannel::configured().as_str()
}

/// Install the newest release on the configured channel over `exe`, the
/// running server's binary, without touching the server. Returns the version
/// installed, or None when there is nothing newer.
#[cfg(not(windows))]
pub(crate) fn install_latest(exe: &Path) -> Result<Option<String>, String> {
    if let Some(refusal) = self_update_refusal() {
        return Err(refusal);
    }
    let Some(release) = check_latest()? else {
        return Ok(None);
    };
    if let Err(e) = crate::release_notes::save_pending(release.label(), &release.notes_body) {
        tracing::warn!("failed to save pending release notes: {e}");
    }
    let mut update = download_update(&release)?;
    // After an earlier install, the running binary's own path names the
    // replaced file; install over the path the server started from.
    update.current_exe = exe.to_path_buf();
    install_downloaded_update(update)?;
    tracing::info!(
        version = release.label(),
        "installed update from the server"
    );
    Ok(Some(release.label().to_owned()))
}

#[cfg(windows)]
pub(crate) fn install_latest(_exe: &Path) -> Result<Option<String>, String> {
    Err(self_update_refusal().unwrap_or_default())
}

/// A binary on disk, as its own `status client --json` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiskBinary {
    pub version: String,
    pub protocol: u32,
    pub endpoint_generation: Option<u32>,
}

/// Ask the binary at `exe` what it is. None when it cannot be run or answers
/// with something else.
pub(crate) fn describe_binary(exe: &Path) -> Option<DiskBinary> {
    let output = crate::noninteractive_process::command(exe)
        .args(["status", "client", "--json"])
        .env_remove(crate::HERDSMAN_ENV_VAR)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_client_status(&output.stdout)
}

fn parse_client_status(json: &[u8]) -> Option<DiskBinary> {
    let value: serde_json::Value = serde_json::from_slice(json).ok()?;
    let client = value.get("client").unwrap_or(&value);
    Some(DiskBinary {
        version: client.get("version")?.as_str()?.to_owned(),
        protocol: u32::try_from(client.get("protocol")?.as_u64()?).ok()?,
        endpoint_generation: client
            .get("endpoint_protocol_generation")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
    })
}

/// What tells a replaced binary apart: its size and modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    inode: u64,
}

pub(crate) fn file_stamp(path: &Path) -> Option<FileStamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        #[cfg(unix)]
        inode: std::os::unix::fs::MetadataExt::ino(&metadata),
    })
}

/// The path the running binary started from. On Linux, once the file is
/// replaced, the process's own path gains a ` (deleted)` suffix.
pub(crate) fn running_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let text = exe.to_string_lossy();
    match text.strip_suffix(" (deleted)") {
        Some(path) => Some(PathBuf::from(path)),
        None => Some(exe),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_status_names_its_version_protocol_and_generation() {
        let json =
            br#"{"client":{"version":"0.4.0","protocol":7,"endpoint_protocol_generation":1}}"#;
        assert_eq!(
            parse_client_status(json),
            Some(DiskBinary {
                version: "0.4.0".into(),
                protocol: 7,
                endpoint_generation: Some(1),
            })
        );
        let flat = br#"{"version":"0.4.0","protocol":7}"#;
        assert_eq!(
            parse_client_status(flat).map(|binary| binary.endpoint_generation),
            Some(None)
        );
        assert_eq!(parse_client_status(b"not json"), None);
        assert_eq!(parse_client_status(br#"{"client":{"version":"x"}}"#), None);
    }

    #[test]
    fn a_replaced_file_gets_a_new_stamp() {
        let dir = std::env::temp_dir().join(format!("herdsman-stamp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bin");
        std::fs::write(&path, b"old").unwrap();
        let before = file_stamp(&path).expect("stamp");
        let next = dir.join("bin.new");
        std::fs::write(&next, b"newer").unwrap();
        std::fs::rename(&next, &path).unwrap();
        assert_ne!(file_stamp(&path), Some(before));
        assert_eq!(file_stamp(&dir.join("missing")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
