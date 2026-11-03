//! The server's side of updating: the `update.*` methods, watching the binary
//! it started from, and automatic mode, which installs and hands off on its own
//! at a quiet moment.
//!
//! herdsman's own module. `headless.rs` reaches it through hooks marked
//! `// fork: update-handoff`; the state it keeps is `app::state::fork::update`.

use super::*;
use crate::api::schema::{
    Method, OutdatedIntegration, ResponseResult, UpdateHandoff, UpdatedIntegration,
};
use crate::app::state::fork::update::{
    self as state, StatusInputs, UpdateState, DISK_CHECK_INTERVAL, INTEGRATION_CHECK_INTERVAL,
};
use crate::config::AutoIntegrations;
use crate::update::handoff as disk;

impl HeadlessServer {
    /// Called from the main loop: look at the binary on disk now and then, and
    /// let automatic mode act.
    pub(super) fn tick_update_handoff(&mut self, now: Instant) {
        if self.handoff_in_progress || self.shutting_down {
            return;
        }
        let update = &mut self.app.state.fork.update;
        if update.next_disk_check.is_none() {
            let exe = disk::running_exe();
            let stamp = exe.as_deref().and_then(disk::file_stamp);
            let auto = update.auto;
            *update = UpdateState::start(exe, stamp, now);
            update.auto = auto;
            update.skipped = state::read_skipped(&state::skipped_path());
            return;
        }
        self.collect_install();
        let update = &mut self.app.state.fork.update;
        if update.next_disk_check.is_some_and(|due| now >= due) {
            update.next_disk_check = Some(now + DISK_CHECK_INTERVAL);
            self.check_disk();
        }
        if self.app.state.fork.update.auto.handoff {
            self.auto_update_step(now);
        }
    }

    /// Note input from an attached client, which automatic mode waits out.
    pub(super) fn note_update_input(&mut self, event: &ServerEvent) {
        if matches!(
            event,
            ServerEvent::ClientInput { .. }
                | ServerEvent::ClientShellPaneInput { .. }
                | ServerEvent::ClientShellPopupInput { .. }
                | ServerEvent::ClientShellEndpointRequest { .. }
        ) {
            self.app.state.fork.update.last_input = Some(Instant::now());
        }
    }

    /// Answer an `update.*` request. Any other request comes back to be
    /// handled as usual.
    pub(super) fn handle_update_request(
        &mut self,
        msg: api::ApiRequestMessage,
    ) -> Option<api::ApiRequestMessage> {
        let id = msg.request.id.clone();
        let response = match &msg.request.method {
            Method::UpdateStatus(_) => self.encode_update_status(id),
            Method::UpdateInstall(_) => self.start_update_install(id),
            Method::UpdateHandoff(_) => return self.handle_update_handoff(msg),
            Method::UpdateIntegrations(_) => {
                let updated = self.update_integrations();
                encode_success(id, ResponseResult::IntegrationsUpdated { updated })
            }
            Method::UpdateSkip(params) => {
                let version = params.version.trim().to_owned();
                if let Err(err) = std::fs::write(state::skipped_path(), format!("{version}\n")) {
                    warn!("failed to remember the skipped update: {err}");
                }
                self.app.state.fork.update.skipped = (!version.is_empty()).then_some(version);
                self.encode_update_status(id)
            }
            _ => return Some(msg),
        };
        let _ = msg.respond_to.send(response);
        None
    }

    fn encode_update_status(&mut self, id: String) -> String {
        self.collect_install();
        let status = self.update_status(Instant::now());
        encode_success(id, ResponseResult::UpdateStatus { status })
    }

    fn update_status(&self, now: Instant) -> crate::api::schema::UpdateStatus {
        let update = &self.app.state.fork.update;
        update.status(StatusInputs {
            running: &crate::build_info::version(),
            endpoint_generation: crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION,
            found: self.app.state.update_available.as_deref(),
            channel: disk::configured_channel(),
            self_update_refusal: disk::self_update_refusal(),
            install_command: crate::update::update_install_command(),
            integrations: outdated_integrations(),
            waiting_for: update
                .auto
                .handoff
                .then(|| update.quiet_wait(self.agents_busy(), now))
                .flatten(),
        })
    }

    fn start_update_install(&mut self, id: String) -> String {
        if let Some(refusal) = disk::self_update_refusal() {
            return encode_error(id, "self_update_unsupported", refusal);
        }
        let found = self.app.state.update_available.clone();
        let update = &self.app.state.fork.update;
        if !update.installing() && update.available(found.as_deref()).is_none() {
            return encode_error(id, "no_update", "no newer release to install");
        }
        self.spawn_install();
        self.encode_update_status(id)
    }

    fn spawn_install(&mut self) {
        let update = &mut self.app.state.fork.update;
        if update.installing() {
            return;
        }
        let Some(exe) = update.exe.clone() else {
            update.install_error = Some("cannot find the server's binary".into());
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("herdsman-update-install".into())
            .spawn(move || {
                let _ = tx.send(disk::install_latest(&exe));
            });
        match spawned {
            Ok(_) => {
                update.install = Some(rx);
                update.install_error = None;
            }
            Err(err) => update.install_error = Some(format!("cannot start the install: {err}")),
        }
    }

    /// Take the answer of a finished install, and look at the new binary.
    fn collect_install(&mut self) {
        let update = &mut self.app.state.fork.update;
        let Some(result) = update.install.as_ref().map(|rx| rx.try_recv()) else {
            return;
        };
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("the install stopped without an answer".into())
            }
        };
        update.install = None;
        match outcome {
            Ok(_) => self.check_disk(),
            Err(err) => {
                warn!("update install failed: {err}");
                self.app.state.fork.update.install_error = Some(err);
            }
        }
    }

    /// Look at the binary the server started from, and describe it when it
    /// changed.
    fn check_disk(&mut self) {
        let update = &mut self.app.state.fork.update;
        let Some(exe) = update.exe.clone() else {
            return;
        };
        let stamp = disk::file_stamp(&exe);
        if stamp == update.stamp {
            return;
        }
        let described = disk::describe_binary(&exe);
        if let Some(binary) = &described {
            info!(version = %binary.version, "the server's binary changed on disk");
        }
        update.saw_disk(stamp, described, &crate::build_info::version());
    }

    fn handle_update_handoff(
        &mut self,
        msg: api::ApiRequestMessage,
    ) -> Option<api::ApiRequestMessage> {
        let id = msg.request.id.clone();
        self.collect_install();
        let generation = crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION;
        let refusal = match self.app.state.fork.update.handoff(generation) {
            None => Some((
                "nothing_to_hand_off",
                "the binary on disk is the running one",
            )),
            Some(UpdateHandoff::Ready) => None,
            Some(_) => Some((
                "handoff_unsupported",
                "this server cannot hand off to the binary on disk; restart it",
            )),
        };
        if let Some((code, message)) = refusal {
            let _ = msg.respond_to.send(encode_error(id, code, message));
            return None;
        }
        match self.hand_off_to_disk() {
            Ok(()) => {
                let _ = msg
                    .respond_to
                    .send(encode_success(id, ResponseResult::Ok {}));
                wait_for_live_handoff_response_write(msg.response_write_complete);
                self.finish_live_handoff_shutdown();
            }
            Err(err) => {
                let _ = msg
                    .respond_to
                    .send(encode_error(id, "handoff_failed", err.to_string()));
            }
        }
        None
    }

    /// Hand the server off to the binary on disk, checking that it is the
    /// version and protocol the server saw there.
    #[cfg(unix)]
    fn hand_off_to_disk(&mut self) -> io::Result<()> {
        let update = &self.app.state.fork.update;
        let (Some(exe), Some(binary)) = (update.exe.clone(), update.disk.clone()) else {
            return Err(io::Error::other("nothing to hand off to"));
        };
        let result = self.perform_live_handoff(crate::api::schema::ServerLiveHandoffParams {
            import_exe: Some(exe.display().to_string()),
            expected_protocol: Some(binary.protocol),
            expected_version: Some(binary.version.clone()),
        });
        if let Err(err) = &result {
            warn!(version = %binary.version, "live handoff failed: {err}");
            let update = &mut self.app.state.fork.update;
            update.handoff_error = Some(err.to_string());
            update.failed_handoff = Some(binary.version);
        }
        result
    }

    #[cfg(not(unix))]
    fn hand_off_to_disk(&mut self) -> io::Result<()> {
        Err(io::Error::other("live handoff is not supported here"))
    }

    fn agents_busy(&self) -> bool {
        self.app.state.terminals.values().any(|terminal| {
            matches!(
                terminal.state,
                crate::detect::AgentState::Working | crate::detect::AgentState::Blocked
            )
        })
    }

    /// Automatic mode: at a quiet moment, hand off to a newer binary on disk,
    /// else install a newer release, and keep integrations current.
    fn auto_update_step(&mut self, now: Instant) {
        let busy = self.agents_busy();
        if self.app.state.fork.update.quiet_wait(busy, now).is_some() {
            return;
        }
        let update = &self.app.state.fork.update;
        let generation = crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION;
        let retry = update
            .disk
            .as_ref()
            .is_some_and(|disk| update.failed_handoff.as_deref() != Some(disk.version.as_str()));
        if update.handoff(generation) == Some(UpdateHandoff::Ready) && retry {
            info!("handing off to the new binary at a quiet moment");
            if self.hand_off_to_disk().is_ok() {
                self.finish_live_handoff_shutdown();
            } else {
                self.update_toast(
                    "Update not applied",
                    "the server keeps running its version; see `herdsman status`",
                );
            }
            return;
        }
        let found = self.app.state.update_available.clone();
        let update = &self.app.state.fork.update;
        if !update.installing()
            && update.install_error.is_none()
            && update.available(found.as_deref()).is_some()
            && disk::self_update_refusal().is_none()
        {
            info!("installing the new release at a quiet moment");
            self.spawn_install();
            return;
        }
        let update = &mut self.app.state.fork.update;
        if update.next_integration_check.is_some_and(|due| now >= due) {
            update.next_integration_check = Some(now + INTEGRATION_CHECK_INTERVAL);
            if update.auto.integrations == AutoIntegrations::Update {
                let updated = self.update_integrations();
                if !updated.is_empty() {
                    let paths = updated
                        .iter()
                        .flat_map(|integration| integration.paths.iter().map(String::as_str))
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.update_toast("Integrations updated", &paths);
                }
            }
        }
    }

    /// Update every installed integration that is out of date. Never installs
    /// one that is not installed.
    fn update_integrations(&mut self) -> Vec<UpdatedIntegration> {
        let mut updated = Vec::new();
        for integration in crate::integration::installed_integration_statuses() {
            if integration.state != crate::integration::IntegrationStatusKind::Outdated {
                continue;
            }
            let agent = crate::integration::integration_target_label(integration.target);
            match crate::integration::install_target(integration.target) {
                Ok(_) => updated.push(UpdatedIntegration {
                    agent: agent.to_owned(),
                    paths: vec![integration.path.display().to_string()],
                }),
                Err(err) => warn!(agent, "integration update failed: {err}"),
            }
        }
        self.app.state.integration_recommendations =
            crate::integration::integration_recommendations();
        updated
    }

    fn update_toast(&mut self, title: &str, context: &str) {
        if matches!(
            self.app.state.toast_config.delivery,
            config::ToastDelivery::Herdsman
        ) {
            self.app.state.toast = Some(app::state::ToastNotification {
                kind: app::state::ToastKind::UpdateInstalled,
                title: title.to_owned(),
                context: context.to_owned(),
                position: None,
                target: None,
            });
        }
    }
}

fn encode_success(id: String, result: ResponseResult) -> String {
    serde_json::to_string(&api::schema::SuccessResponse { id, result })
        .unwrap_or_else(|_| "{}".to_string())
}

fn encode_error(id: String, code: &str, message: impl Into<String>) -> String {
    serde_json::to_string(&api::schema::ErrorResponse {
        id,
        error: api::schema::ErrorBody {
            code: code.into(),
            message: message.into(),
        },
    })
    .unwrap_or_else(|_| "{}".to_string())
}

fn outdated_integrations() -> Vec<OutdatedIntegration> {
    crate::integration::installed_integration_statuses()
        .into_iter()
        .filter(|integration| {
            integration.state == crate::integration::IntegrationStatusKind::Outdated
        })
        .map(|integration| OutdatedIntegration {
            agent: crate::integration::integration_target_label(integration.target).to_owned(),
            installed: integration.installed_version,
            expected: integration.expected_version,
            paths: vec![integration.path.display().to_string()],
        })
        .collect()
}
