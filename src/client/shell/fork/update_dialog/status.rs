use super::*;

impl ClientShellState {
    fn prepare_update_cache(&mut self) {
        let Some(boot_id) = self.endpoint_boot_id(&self.active_endpoint_id) else {
            return;
        };
        if self
            .fork
            .updates
            .endpoints
            .get(&self.active_endpoint_id)
            .is_none_or(|cached| cached.boot_id != boot_id)
        {
            self.fork.updates.endpoints.insert(
                self.active_endpoint_id.clone(),
                EndpointUpdate {
                    boot_id: boot_id.to_owned(),
                    ..Default::default()
                },
            );
        }
    }

    pub(in crate::client::shell) fn poll_update_status(
        &mut self,
        now: Instant,
        outcome: &mut ClientShellInput,
    ) {
        if !self.update_supported() || !self.endpoint_is_online(&self.active_endpoint_id) {
            return;
        }
        self.prepare_update_cache();
        let Some(cached) = self.fork.updates.endpoints.get(&self.active_endpoint_id) else {
            return;
        };
        if cached
            .pending_id
            .as_ref()
            .is_some_and(|id| self.pending_requests.contains_key(id))
        {
            return;
        }
        if matches!(self.overlay.as_ref(), Some(ClientShellOverlay::UpdateDialog(dialog)) if dialog.progress == Some(Progress::HandingOff))
        {
            return;
        }
        let installing = cached
            .status
            .as_ref()
            .is_some_and(|status| status.installing)
            || matches!(self.overlay.as_ref(), Some(ClientShellOverlay::UpdateDialog(dialog)) if matches!(dialog.progress, Some(Progress::Installing { .. })));
        let interval = Duration::from_secs(if installing { 2 } else { 300 });
        let changed = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_ref())
            .is_some_and(|snapshot| snapshot.update_available != cached.available);
        if changed
            || cached
                .checked_at
                .is_none_or(|last| now.saturating_duration_since(last) >= interval)
        {
            self.request_update_status(now, outcome);
        }
    }

    pub(super) fn request_update_status(&mut self, now: Instant, outcome: &mut ClientShellInput) {
        if !self.update_supported() {
            return;
        }
        self.prepare_update_cache();
        if self
            .fork
            .updates
            .endpoints
            .get(&self.active_endpoint_id)
            .and_then(|cached| cached.pending_id.as_ref())
            .is_some_and(|id| self.pending_requests.contains_key(id))
        {
            return;
        }
        let available = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_ref())
            .and_then(|snapshot| snapshot.update_available.clone());
        if let Some(cached) = self
            .fork
            .updates
            .endpoints
            .get_mut(&self.active_endpoint_id)
        {
            cached.checked_at = Some(now);
            cached.available = available;
        }
        self.send_update_request(
            Method::UpdateStatus(EmptyParams {}),
            RequestKind::Status,
            outcome,
        );
    }

    pub(super) fn send_update_request(
        &mut self,
        method: Method,
        kind: RequestKind,
        outcome: &mut ClientShellInput,
    ) {
        let name = crate::api::api_method_name(&method);
        if !self.advertises_update_method(name)
            || !self.endpoint_is_online(&self.active_endpoint_id)
        {
            self.fail_update_dialog(format!(
                "{} cannot run {name} right now.",
                self.active_endpoint_label()
            ));
            return;
        }
        let dialog_id = match self.overlay.as_ref() {
            Some(ClientShellOverlay::UpdateDialog(dialog))
                if dialog.endpoint_id == self.active_endpoint_id =>
            {
                Some(dialog.id)
            }
            _ => None,
        };
        let request = UpdateRequest {
            endpoint_id: self.active_endpoint_id.clone(),
            dialog_id,
            kind,
        };
        if self.push_endpoint_method_with_kind(
            method,
            PendingEndpointKind::UpdateDialog(request),
            outcome,
        ) {
            let id = outcome.actions.last().and_then(|action| match action {
                ClientShellAction::Endpoint { request, .. } => Some(request.id.clone()),
                _ => None,
            });
            if let Some(cached) = self
                .fork
                .updates
                .endpoints
                .get_mut(&self.active_endpoint_id)
            {
                cached.pending_id = id;
            }
        } else {
            self.fail_update_dialog("The update action could not be sent.".into());
        }
    }

    fn fail_update_dialog(&mut self, message: String) {
        if let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_mut() {
            dialog.progress = None;
            dialog.error = Some(message);
            dialog.scroll = 0;
            dialog.selected = 0;
        }
    }

    /// Project cached API state onto the existing client-owned chrome. The
    /// endpoint's original snapshot stays untouched; no wire fields are added.
    pub(in crate::client::shell) fn project_update_chrome(
        &self,
        snapshot: &mut ClientShellSnapshot,
    ) {
        if !self.update_supported() {
            return;
        }
        let Some(status) = self
            .fork
            .updates
            .endpoints
            .get(&self.active_endpoint_id)
            .filter(|cached| cached.boot_id == snapshot.boot_id)
            .and_then(|cached| cached.status.as_ref())
        else {
            return;
        };
        snapshot.update_available = if needs_attention(status) {
            Some(
                status
                    .available
                    .as_ref()
                    .or(status.installed.as_ref())
                    .unwrap_or(&status.running)
                    .clone(),
            )
        } else {
            None
        };
        snapshot.integration_updates_available = !status.integrations.is_empty();
    }

    pub(in crate::client::shell) fn refresh_update_chrome(&mut self) {
        let Some(mut snapshot) = self.snapshot.take() else {
            return;
        };
        if let Some(original) = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_ref())
        {
            snapshot.update_available = original.update_available.clone();
            snapshot.integration_updates_available = original.integration_updates_available;
        }
        self.project_update_chrome(&mut snapshot);
        self.snapshot = Some(snapshot);
    }

    pub(in crate::client::shell) fn receive_update_result(
        &mut self,
        request: UpdateRequest,
        result: Result<ResponseResult, ClientShellEndpointError>,
    ) -> (bool, Vec<ClientShellAction>) {
        if request.endpoint_id != self.active_endpoint_id {
            return (false, Vec::new());
        }
        if let Some(cached) = self.fork.updates.endpoints.get_mut(&request.endpoint_id) {
            cached.pending_id = None;
        }
        let owns = matches!(self.overlay.as_ref(), Some(ClientShellOverlay::UpdateDialog(dialog))
            if dialog.endpoint_id == request.endpoint_id && (Some(dialog.id) == request.dialog_id || matches!(request.kind, RequestKind::Status)));
        let mut outcome = ClientShellInput::default();
        match (request.kind, result) {
            (
                RequestKind::Status | RequestKind::Install { .. } | RequestKind::Skip,
                Ok(ResponseResult::UpdateStatus { status }),
            ) => {
                self.prepare_update_cache();
                if let Some(cached) = self.fork.updates.endpoints.get_mut(&request.endpoint_id) {
                    cached.status = Some(status.clone());
                }
                if owns {
                    let mut handoff = false;
                    if let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_mut() {
                        let installing = match request.kind {
                            RequestKind::Install { handoff } => Some(handoff),
                            RequestKind::Status => match dialog.progress {
                                Some(Progress::Installing { handoff }) => Some(handoff),
                                _ => None,
                            },
                            _ => None,
                        };
                        dialog.status = Some(status.clone());
                        dialog.selected = 0;
                        if let Some(after_install) = installing {
                            if status.installing {
                                dialog.progress = Some(Progress::Installing {
                                    handoff: after_install,
                                });
                            } else {
                                dialog.progress = None;
                                if let Some(error) = status.install_error.as_ref() {
                                    dialog.error = Some(error.clone());
                                    dialog.scroll = 0;
                                } else if after_install
                                    && status.handoff == Some(UpdateHandoff::Ready)
                                {
                                    handoff = true;
                                }
                            }
                        } else {
                            dialog.progress = None;
                        }
                    }
                    if handoff {
                        if let Some(ClientShellOverlay::UpdateDialog(dialog)) =
                            self.overlay.as_mut()
                        {
                            dialog.progress = Some(Progress::HandingOff);
                        }
                        self.send_update_request(
                            Method::UpdateHandoff(EmptyParams {}),
                            RequestKind::Handoff,
                            &mut outcome,
                        );
                    }
                }
                self.refresh_update_chrome();
            }
            (RequestKind::Handoff, Ok(ResponseResult::Ok {})) => {
                // The server writes the response before shutting down. Keep the
                // modal waiting for the client reload handled outside the shell.
            }
            (RequestKind::Integrations, Ok(ResponseResult::IntegrationsUpdated { updated })) => {
                if owns {
                    if let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_mut() {
                        dialog.information = updated
                            .into_iter()
                            .flat_map(|integration| {
                                std::iter::once(format!("Updated {}", integration.agent)).chain(
                                    integration
                                        .paths
                                        .into_iter()
                                        .map(|path| format!("  {path}")),
                                )
                            })
                            .collect();
                        dialog.progress = Some(Progress::Checking);
                    }
                    self.request_update_status(Instant::now(), &mut outcome);
                }
            }
            (RequestKind::Status, Err(error))
                if matches!(
                    error.code.as_deref(),
                    Some("unknown_method" | "method_not_found" | "unsupported_method")
                ) =>
            {
                if let Some(cached) = self.fork.updates.endpoints.get_mut(&request.endpoint_id) {
                    cached.unsupported = true;
                }
                self.refresh_update_chrome();
                if owns {
                    self.overlay = None;
                    self.open_release_notes();
                }
            }
            (_, Err(error)) => {
                if owns {
                    self.fail_update_dialog(error.message);
                } else if error.code.as_deref() != Some("endpoint_cancelled") {
                    self.push_endpoint_notice(
                        ClientEndpointNoticeKind::Rejected,
                        "update.status",
                        "Update status unavailable",
                        error.message,
                    );
                }
            }
            (_, Ok(_)) => {
                if owns {
                    self.fail_update_dialog(
                        "The server returned an unexpected update result.".into(),
                    );
                }
            }
        }
        (true, outcome.actions)
    }
}
