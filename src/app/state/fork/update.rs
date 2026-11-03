//! What the server keeps about updating itself: the binary it started from and
//! what is on disk there now, an install running in the background, a release
//! the user skipped, and when someone last typed, for automatic mode.
//!
//! The decisions here are pure; `server/headless/update.rs` does the work.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::api::schema::{OutdatedIntegration, UpdateAuto, UpdateHandoff, UpdateStatus};
use crate::config::AutoUpdateConfig;
use crate::update::handoff::{DiskBinary, FileStamp};

/// How often the server looks at its binary on disk.
pub(crate) const DISK_CHECK_INTERVAL: Duration = Duration::from_secs(60);
/// How often the loop looks for a finished install.
const INSTALL_POLL: Duration = Duration::from_secs(1);
/// How long nobody may have typed before automatic mode acts.
pub(crate) const QUIET_FOR: Duration = Duration::from_secs(5 * 60);
/// How often automatic mode looks at the integrations.
pub(crate) const INTEGRATION_CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Default)]
pub(crate) struct UpdateState {
    /// The binary the server started from.
    pub exe: Option<PathBuf>,
    /// That file as the server last saw it.
    pub stamp: Option<FileStamp>,
    /// What is on disk there, when it is not the running version.
    pub disk: Option<DiskBinary>,
    pub next_disk_check: Option<Instant>,
    /// The answer of an install running in the background.
    pub install: Option<Receiver<Result<Option<String>, String>>>,
    pub install_error: Option<String>,
    pub handoff_error: Option<String>,
    /// A version automatic mode failed to hand off to; it does not try again.
    pub failed_handoff: Option<String>,
    /// A release the user chose to skip.
    pub skipped: Option<String>,
    pub last_input: Option<Instant>,
    pub auto: AutoUpdateConfig,
    pub next_integration_check: Option<Instant>,
}

/// Facts from outside this state that `status` reports.
pub(crate) struct StatusInputs<'a> {
    pub running: &'a str,
    pub endpoint_generation: u32,
    /// The release the periodic check found, if any.
    pub found: Option<&'a str>,
    pub channel: &'a str,
    pub self_update_refusal: Option<String>,
    pub install_command: &'a str,
    pub integrations: Vec<OutdatedIntegration>,
    pub waiting_for: Option<&'static str>,
}

impl UpdateState {
    pub fn start(exe: Option<PathBuf>, stamp: Option<FileStamp>, now: Instant) -> Self {
        Self {
            exe,
            stamp,
            next_disk_check: Some(now + DISK_CHECK_INTERVAL),
            // A server that just took over checks the integrations first.
            next_integration_check: Some(now),
            ..Self::default()
        }
    }

    /// When the server loop should next look: the disk check, or soon while
    /// an install runs.
    pub fn next_deadline(&self, now: Instant) -> Option<Instant> {
        let install = self.installing().then(|| now + INSTALL_POLL);
        self.next_disk_check.into_iter().chain(install).min()
    }

    pub fn installing(&self) -> bool {
        self.install.is_some()
    }

    /// Record what is on disk; only a version other than `running` counts.
    pub fn saw_disk(&mut self, stamp: Option<FileStamp>, disk: Option<DiskBinary>, running: &str) {
        self.stamp = stamp;
        self.disk = disk.filter(|disk| disk.version != running);
    }

    /// The newest release worth offering: the one found, unless it is already
    /// on disk or skipped.
    pub fn available<'a>(&self, found: Option<&'a str>) -> Option<&'a str> {
        found.filter(|version| {
            self.disk
                .as_ref()
                .is_none_or(|disk| disk.version != *version)
                && self.skipped.as_deref() != Some(*version)
        })
    }

    /// Whether the server can move to the binary on disk.
    pub fn handoff(&self, endpoint_generation: u32) -> Option<UpdateHandoff> {
        let disk = self.disk.as_ref()?;
        let same_generation = disk
            .endpoint_generation
            .is_none_or(|generation| generation == endpoint_generation);
        Some(if cfg!(unix) && same_generation {
            UpdateHandoff::Ready
        } else {
            UpdateHandoff::Unsupported
        })
    }

    /// What automatic mode waits for, or None when it may act now.
    pub fn quiet_wait(&self, agents_busy: bool, now: Instant) -> Option<&'static str> {
        if agents_busy {
            Some("agents_busy")
        } else if self
            .last_input
            .is_some_and(|input| now.saturating_duration_since(input) < QUIET_FOR)
        {
            Some("recent_input")
        } else {
            None
        }
    }

    pub fn status(&self, inputs: StatusInputs<'_>) -> UpdateStatus {
        let self_update = inputs.self_update_refusal.is_none();
        UpdateStatus {
            running: inputs.running.to_owned(),
            installed: self.disk.as_ref().map(|disk| disk.version.clone()),
            available: self.available(inputs.found).map(str::to_owned),
            channel: inputs.channel.to_owned(),
            self_update,
            install_command: (!self_update).then(|| inputs.install_command.to_owned()),
            installing: self.installing(),
            install_error: self.install_error.clone(),
            handoff: self.handoff(inputs.endpoint_generation),
            handoff_error: self.handoff_error.clone(),
            integrations: inputs.integrations,
            auto: self.auto.handoff.then(|| UpdateAuto {
                waiting_for: inputs.waiting_for.map(str::to_owned),
            }),
            skipped: self.skipped.clone(),
        }
    }
}

/// Where a skipped release is remembered.
pub(crate) fn skipped_path() -> PathBuf {
    crate::session::data_dir().join("update-skipped")
}

pub(crate) fn read_skipped(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let version = text.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(version: &str, generation: Option<u32>) -> DiskBinary {
        DiskBinary {
            version: version.into(),
            protocol: 1,
            endpoint_generation: generation,
        }
    }

    fn inputs<'a>(found: Option<&'a str>) -> StatusInputs<'a> {
        StatusInputs {
            running: "0.3.0",
            endpoint_generation: 1,
            found,
            channel: "stable",
            self_update_refusal: None,
            install_command: "herdsman update",
            integrations: Vec::new(),
            waiting_for: None,
        }
    }

    #[test]
    fn only_another_version_on_disk_is_ready_to_hand_off() {
        let mut state = UpdateState::default();
        state.saw_disk(None, Some(disk("0.3.0", Some(1))), "0.3.0");
        assert_eq!(state.handoff(1), None);
        state.saw_disk(None, Some(disk("0.4.0", Some(1))), "0.3.0");
        let expected = if cfg!(unix) {
            UpdateHandoff::Ready
        } else {
            UpdateHandoff::Unsupported
        };
        assert_eq!(state.handoff(1), Some(expected));
        state.saw_disk(None, Some(disk("1.0.0", Some(2))), "0.3.0");
        assert_eq!(state.handoff(1), Some(UpdateHandoff::Unsupported));
    }

    #[test]
    fn a_release_already_installed_or_skipped_is_not_offered() {
        let mut state = UpdateState::default();
        assert_eq!(state.available(Some("0.4.0")), Some("0.4.0"));
        state.saw_disk(None, Some(disk("0.4.0", None)), "0.3.0");
        assert_eq!(state.available(Some("0.4.0")), None);
        state.skipped = Some("0.5.0".into());
        assert_eq!(state.available(Some("0.5.0")), None);
        assert_eq!(state.available(Some("0.6.0")), Some("0.6.0"));
    }

    #[test]
    fn automatic_mode_waits_for_agents_and_typing() {
        let now = Instant::now();
        let mut state = UpdateState::default();
        assert_eq!(state.quiet_wait(true, now), Some("agents_busy"));
        assert_eq!(state.quiet_wait(false, now), None);
        state.last_input = Some(now);
        assert_eq!(state.quiet_wait(false, now), Some("recent_input"));
        assert_eq!(state.quiet_wait(false, now + QUIET_FOR), None);
    }

    #[test]
    fn the_status_names_what_can_be_done() {
        let mut state = UpdateState::default();
        let status = state.status(inputs(Some("0.4.0")));
        assert_eq!(status.available.as_deref(), Some("0.4.0"));
        assert!(status.self_update && status.install_command.is_none());
        assert_eq!(status.auto, None);

        state.auto.handoff = true;
        let status = state.status(StatusInputs {
            self_update_refusal: Some("brew".into()),
            install_command: "brew upgrade herdsman",
            waiting_for: Some("agents_busy"),
            ..inputs(None)
        });
        assert!(!status.self_update);
        assert_eq!(
            status.install_command.as_deref(),
            Some("brew upgrade herdsman")
        );
        assert_eq!(
            status.auto.and_then(|auto| auto.waiting_for).as_deref(),
            Some("agents_busy")
        );
    }

    #[test]
    fn a_skipped_release_is_read_back_and_blank_means_none() {
        let dir = std::env::temp_dir().join(format!("herdsman-skip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("update-skipped");
        assert_eq!(read_skipped(&path), None);
        std::fs::write(&path, "0.4.0\n").unwrap();
        assert_eq!(read_skipped(&path).as_deref(), Some("0.4.0"));
        std::fs::write(&path, " \n").unwrap();
        assert_eq!(read_skipped(&path), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
