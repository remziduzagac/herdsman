//! Client-only update presentation and actions. Server state stays in update.status.

use super::super::*;
use crate::api::schema::{
    EmptyParams, Method, ResponseResult, UpdateHandoff, UpdateSkipParams, UpdateStatus,
};
use std::time::{Duration, Instant};

mod status;

#[derive(Debug, Default)]
pub(in crate::client::shell) struct UpdateState {
    endpoints: HashMap<ClientEndpointId, EndpointUpdate>,
    generation: u64,
}

#[derive(Debug, Default)]
struct EndpointUpdate {
    boot_id: String,
    available: Option<String>,
    status: Option<UpdateStatus>,
    checked_at: Option<Instant>,
    pending_id: Option<String>,
    unsupported: bool,
}

#[derive(Debug)]
pub(in crate::client::shell) struct UpdateDialog {
    id: u64,
    endpoint_id: ClientEndpointId,
    pub(in crate::client::shell) endpoint_label: String,
    pub(in crate::client::shell) status: Option<UpdateStatus>,
    pub(in crate::client::shell) selected: usize,
    pub(in crate::client::shell) scroll: usize,
    pub(in crate::client::shell) error: Option<String>,
    information: Vec<String>,
    progress: Option<Progress>,
    can_install: bool,
    can_handoff: bool,
    can_skip: bool,
    can_integrations: bool,
    release_notes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Progress {
    Checking,
    Installing { handoff: bool },
    HandingOff,
    Integrations,
    Skipping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::client::shell) enum UpdateAction {
    InstallAndHandoff,
    Install,
    Handoff,
    Integrations,
    Skip,
    ReleaseNotes,
    Later,
}

impl UpdateAction {
    pub(in crate::client::shell) fn label(self) -> &'static str {
        match self {
            Self::InstallAndHandoff => "Update and hand off",
            Self::Install => "Update only",
            Self::Handoff => "Hand off",
            Self::Integrations => "Update integrations",
            Self::Skip => "Skip this version",
            Self::ReleaseNotes => "What's new",
            Self::Later => "Later",
        }
    }
}

#[derive(Debug)]
pub(in crate::client::shell) struct UpdateRequest {
    endpoint_id: ClientEndpointId,
    dialog_id: Option<u64>,
    kind: RequestKind,
}

#[derive(Clone, Copy, Debug)]
enum RequestKind {
    Status,
    Install { handoff: bool },
    Handoff,
    Integrations,
    Skip,
}

fn release_pending(status: &UpdateStatus) -> bool {
    status
        .available
        .as_ref()
        .is_some_and(|version| Some(version) != status.skipped.as_ref())
}

fn needs_attention(status: &UpdateStatus) -> bool {
    release_pending(status)
        || matches!(
            status.handoff,
            Some(UpdateHandoff::Ready | UpdateHandoff::Unsupported)
        )
        || !status.integrations.is_empty()
        || status.installing
        || status.install_error.is_some()
        || status.handoff_error.is_some()
}

impl UpdateDialog {
    pub(in crate::client::shell) fn busy(&self) -> bool {
        self.progress
            .is_some_and(|progress| progress != Progress::Checking)
            || (self.error.is_none()
                && self.status.as_ref().is_some_and(|status| status.installing))
    }

    pub(in crate::client::shell) fn progress_label(&self) -> Option<&'static str> {
        match self.progress {
            Some(Progress::Checking) => Some("checking for updates…"),
            Some(Progress::Installing { .. }) => Some("updating…"),
            Some(Progress::HandingOff) => Some("handing off…"),
            Some(Progress::Integrations) => Some("updating integrations…"),
            Some(Progress::Skipping) => Some("skipping this version…"),
            None if self.status.as_ref().is_some_and(|status| status.installing) => {
                Some("updating…")
            }
            None => None,
        }
    }

    pub(in crate::client::shell) fn actions(&self) -> Vec<UpdateAction> {
        let mut actions = Vec::new();
        if self.progress.is_none() && !self.busy() {
            if let Some(status) = self.status.as_ref().filter(|status| !status.installing) {
                if release_pending(status) && status.self_update {
                    if self.can_install && self.can_handoff {
                        actions.push(UpdateAction::InstallAndHandoff);
                    }
                    if self.can_install {
                        actions.push(UpdateAction::Install);
                    }
                    if self.can_skip {
                        actions.push(UpdateAction::Skip);
                    }
                }
                if status.handoff == Some(UpdateHandoff::Ready) && self.can_handoff {
                    actions.push(UpdateAction::Handoff);
                }
                if !status.integrations.is_empty() && self.can_integrations {
                    actions.push(UpdateAction::Integrations);
                }
            }
            if self.release_notes {
                actions.push(UpdateAction::ReleaseNotes);
            }
        }
        if !self.busy() {
            actions.push(UpdateAction::Later);
        }
        actions
    }

    pub(in crate::client::shell) fn details(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(status) = self.status.as_ref() {
            lines.push(format!("Running: {} · {}", status.running, status.channel));
            if let Some(installed) = status.installed.as_ref() {
                lines.push(format!("Installed: {installed}"));
            }
            if let Some(available) = status.available.as_ref() {
                lines.push(format!(
                    "Available: {available}{}",
                    if release_pending(status) {
                        ""
                    } else {
                        " (skipped)"
                    }
                ));
                if !status.self_update {
                    lines.push("Update with your package manager:".into());
                    if let Some(command) = status.install_command.as_ref() {
                        lines.push(command.clone());
                    }
                }
            }
            if status.handoff == Some(UpdateHandoff::Unsupported) {
                lines.push("Restart needed to use the installed version.".into());
            }
            if let Some(error) = status.install_error.as_ref() {
                lines.push(format!("Install failed: {error}"));
            }
            if let Some(error) = status.handoff_error.as_ref() {
                lines.push(format!("Handoff failed: {error}"));
            }
            if let Some(auto) = status.auto.as_ref() {
                lines.push(format!(
                    "Automatic updates enabled{}",
                    auto.waiting_for
                        .as_ref()
                        .map(|reason| format!(" · waiting: {reason}"))
                        .unwrap_or_default()
                ));
            }
            if !status.integrations.is_empty() {
                lines.push(String::new());
                lines.push("Outdated integrations".into());
                for integration in &status.integrations {
                    lines.push(format!(
                        "{}: {} → {}",
                        integration.agent,
                        integration
                            .installed
                            .map(|version| version.to_string())
                            .unwrap_or_else(|| "unversioned".into()),
                        integration.expected
                    ));
                    lines.extend(integration.paths.iter().map(|path| format!("  {path}")));
                }
            }
            if !needs_attention(status) {
                lines.push("Everything is up to date.".into());
            }
        }
        lines.extend(self.information.iter().cloned());
        lines
    }
}

impl ClientShellState {
    fn advertises_update_method(&self, name: &str) -> bool {
        self.endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id)
            .and_then(|endpoint| endpoint.methods.as_ref())
            .is_some_and(|methods| methods.contains(name))
    }

    fn update_supported(&self) -> bool {
        self.advertises_update_method("update.status")
            && !self
                .fork
                .updates
                .endpoints
                .get(&self.active_endpoint_id)
                .is_some_and(|cached| {
                    cached.unsupported
                        && self.endpoint_boot_id(&self.active_endpoint_id)
                            == Some(cached.boot_id.as_str())
                })
    }

    pub(in crate::client::shell) fn update_menu_action(
        &self,
        action: super::super::global_menu::ClientGlobalMenuAction,
    ) -> super::super::global_menu::ClientGlobalMenuAction {
        use super::super::global_menu::ClientGlobalMenuAction;
        if action == ClientGlobalMenuAction::WhatsNew
            && self.update_supported()
            && self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.update_available.is_some())
        {
            ClientGlobalMenuAction::Update
        } else {
            action
        }
    }

    pub(in crate::client::shell) fn open_update_dialog(&mut self, outcome: &mut ClientShellInput) {
        if !self.update_supported() {
            return;
        }
        self.fork.updates.generation = self.fork.updates.generation.saturating_add(1);
        let status = self
            .fork
            .updates
            .endpoints
            .get(&self.active_endpoint_id)
            .filter(|cached| {
                self.endpoint_boot_id(&self.active_endpoint_id) == Some(cached.boot_id.as_str())
            })
            .and_then(|cached| cached.status.clone());
        self.overlay = Some(ClientShellOverlay::UpdateDialog(UpdateDialog {
            id: self.fork.updates.generation,
            endpoint_id: self.active_endpoint_id.clone(),
            endpoint_label: self.active_endpoint_label().to_owned(),
            status,
            selected: 0,
            scroll: 0,
            error: None,
            information: Vec::new(),
            progress: Some(Progress::Checking),
            can_install: self.advertises_update_method("update.install"),
            can_handoff: self.advertises_update_method("update.handoff"),
            can_skip: self.advertises_update_method("update.skip"),
            can_integrations: self.advertises_update_method("update.integrations"),
            release_notes: self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.release_notes.is_some()),
        }));
        self.chrome_drag = None;
        self.request_update_status(Instant::now(), outcome);
        outcome.repaint = true;
    }

    pub(in crate::client::shell) fn update_label_hit(&self, bar: Option<Rect>) -> Rect {
        if !self.update_supported()
            || self.mode != ClientShellMode::Navigate
            || self
                .snapshot
                .as_ref()
                .is_none_or(|snapshot| snapshot.update_available.is_none())
        {
            return Rect::default();
        }
        bar.map(|bar| {
            Rect::new(
                bar.right().saturating_sub(13.min(bar.width)),
                bar.y,
                13.min(bar.width),
                1,
            )
        })
        .unwrap_or_default()
    }

    pub(in crate::client::shell) fn route_update_dialog_key(
        &mut self,
        key: &crate::input::TerminalKey,
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_mut() else {
            return false;
        };
        if key.kind != crossterm::event::KeyEventKind::Press {
            return true;
        }
        let count = dialog.actions().len();
        match key.code {
            KeyCode::Esc if !dialog.busy() => self.overlay = None,
            KeyCode::Up | KeyCode::BackTab => dialog.selected = dialog.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => {
                dialog.selected = dialog
                    .selected
                    .saturating_add(1)
                    .min(count.saturating_sub(1))
            }
            KeyCode::PageUp => dialog.scroll = dialog.scroll.saturating_sub(5),
            KeyCode::PageDown => {
                dialog.scroll = dialog
                    .scroll
                    .saturating_add(5)
                    .min(self.hits.help_max_scroll)
            }
            KeyCode::Enter => self.activate_update_dialog(outcome),
            _ => {}
        }
        outcome.repaint = true;
        true
    }

    pub(in crate::client::shell) fn handle_update_dialog_mouse(
        &mut self,
        mouse: &crossterm::event::MouseEvent,
        outcome: &mut ClientShellInput,
    ) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};
        let point = (mouse.column, mouse.row);
        if let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_mut() {
            match mouse.kind {
                MouseEventKind::ScrollUp => dialog.scroll = dialog.scroll.saturating_sub(3),
                MouseEventKind::ScrollDown => {
                    dialog.scroll = dialog
                        .scroll
                        .saturating_add(3)
                        .min(self.hits.help_max_scroll)
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((_, index)) = self
                        .hits
                        .worktree_rows
                        .iter()
                        .find(|(rect, _)| contains(*rect, point))
                    {
                        dialog.selected = *index;
                        self.activate_update_dialog(outcome);
                    }
                }
                _ => {}
            }
            outcome.repaint = true;
            return true;
        }
        if self.overlay.is_none()
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && contains(self.hits.fork.update_label, point)
        {
            self.open_update_dialog(outcome);
            return true;
        }
        false
    }

    fn activate_update_dialog(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_ref() else {
            return;
        };
        if dialog.endpoint_id != self.active_endpoint_id {
            return;
        }
        let Some(action) = dialog.actions().get(dialog.selected).copied() else {
            return;
        };
        let (method, kind, progress) = match action {
            UpdateAction::InstallAndHandoff | UpdateAction::Install => (
                Method::UpdateInstall(EmptyParams {}),
                RequestKind::Install {
                    handoff: action == UpdateAction::InstallAndHandoff,
                },
                Progress::Installing {
                    handoff: action == UpdateAction::InstallAndHandoff,
                },
            ),
            UpdateAction::Handoff => (
                Method::UpdateHandoff(EmptyParams {}),
                RequestKind::Handoff,
                Progress::HandingOff,
            ),
            UpdateAction::Integrations => (
                Method::UpdateIntegrations(EmptyParams {}),
                RequestKind::Integrations,
                Progress::Integrations,
            ),
            UpdateAction::Skip => {
                let Some(version) = dialog
                    .status
                    .as_ref()
                    .and_then(|status| status.available.clone())
                else {
                    return;
                };
                (
                    Method::UpdateSkip(UpdateSkipParams { version }),
                    RequestKind::Skip,
                    Progress::Skipping,
                )
            }
            UpdateAction::ReleaseNotes => {
                self.open_release_notes();
                outcome.repaint = true;
                return;
            }
            UpdateAction::Later => {
                self.overlay = None;
                outcome.repaint = true;
                return;
            }
        };
        if let Some(ClientShellOverlay::UpdateDialog(dialog)) = self.overlay.as_mut() {
            dialog.progress = Some(progress);
            dialog.error = None;
            dialog.selected = 0;
        }
        self.send_update_request(method, kind, outcome);
        outcome.repaint = true;
    }
}
