//! Preset choices for workspace and worktree creation. The existing forms
//! keep their geometry; a spare row opens the shared presets picker.

use super::*;
use crate::api::schema::{
    PresetApplyParams, PresetDefaultKind, PresetDefaultParams, WorkspaceCreateParams,
};

#[derive(Debug, Clone, Copy)]
pub(in crate::client::shell) enum CreationKind {
    Workspace,
    WorktreeCreate,
    WorktreeOpen,
}

#[derive(Debug)]
pub(in crate::client::shell) struct CreationState {
    generation: u64,
    endpoint_id: ClientEndpointId,
    kind: CreationKind,
    source_workspace_id: Option<String>,
    target_cwd: Option<String>,
    workspace_submitting: bool,
    entries: Vec<PresetSummary>,
    selected: Option<String>,
    loading: bool,
}

#[derive(Debug)]
pub(in crate::client::shell) struct CreationPicker {
    pub(in crate::client::shell) previous: Option<Box<ClientShellOverlay>>,
}

#[derive(Debug)]
pub(in crate::client::shell) enum CreationRequest {
    Default(u64),
    List(u64),
    Created {
        generation: u64,
        kind: CreationKind,
        preset: Option<String>,
    },
    Apply,
}

impl ClientShellState {
    fn supports_creation_presets(&self) -> bool {
        self.endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id)
            .and_then(|endpoint| endpoint.methods.as_ref())
            .is_some_and(|methods| {
                ["preset.list", "preset.default", "preset.apply"]
                    .iter()
                    .all(|name| methods.contains(*name))
            })
    }

    pub(in crate::client::shell) fn begin_workspace_presets(
        &mut self,
        outcome: &mut ClientShellInput,
    ) -> bool {
        self.fork.creation = None;
        if !self.supports_creation_presets() {
            return false;
        }
        let source = self.workspace_action_id();
        if self.config.prompt_new_workspace_name {
            self.open_new_workspace_overlay();
        } else {
            let mut list = PresetsOverlay::new(source.clone());
            list.creation = Some(CreationPicker { previous: None });
            self.overlay = Some(ClientShellOverlay::Presets(list));
        }
        self.start_creation_presets(CreationKind::Workspace, source, outcome);
        true
    }

    pub(in crate::client::shell) fn begin_worktree_presets(
        &mut self,
        outcome: &mut ClientShellInput,
    ) {
        self.fork.creation = None;
        if !self.supports_creation_presets() {
            return;
        }
        let (kind, source) = match self.overlay.as_ref() {
            Some(ClientShellOverlay::WorktreeCreate(form)) => (
                CreationKind::WorktreeCreate,
                form.source_workspace_id.clone(),
            ),
            Some(ClientShellOverlay::WorktreeOpen(form)) => {
                (CreationKind::WorktreeOpen, form.source_workspace_id.clone())
            }
            _ => return,
        };
        self.start_creation_presets(kind, Some(source), outcome);
    }

    fn start_creation_presets(
        &mut self,
        kind: CreationKind,
        source: Option<String>,
        outcome: &mut ClientShellInput,
    ) {
        let target_cwd = match kind {
            CreationKind::Workspace => match self.overlay.as_ref() {
                Some(ClientShellOverlay::Rename(ClientRenameOverlay {
                    target: ClientRenameTarget::NewWorkspace { cwd, .. },
                    ..
                })) => cwd.clone(),
                _ => self.snapshot.as_deref().and_then(|snapshot| {
                    snapshot
                        .workspaces
                        .iter()
                        .find(|workspace| Some(&workspace.workspace_id) == source.as_ref())
                        .map(|workspace| workspace.new_workspace_cwd.clone())
                }),
            },
            CreationKind::WorktreeOpen => match self.overlay.as_ref() {
                Some(ClientShellOverlay::WorktreeOpen(form)) => form
                    .selected_entry_index()
                    .and_then(|index| form.entries.get(index))
                    .map(|entry| entry.path.clone()),
                _ => None,
            },
            // The checkout does not exist yet. All its worktrees share the
            // source repository's name, so its source resolves the same default.
            CreationKind::WorktreeCreate => None,
        };
        self.fork.creation_generation = self.fork.creation_generation.saturating_add(1);
        let generation = self.fork.creation_generation;
        self.fork.creation = Some(CreationState {
            generation,
            endpoint_id: self.active_endpoint_id.clone(),
            kind,
            source_workspace_id: source.clone(),
            target_cwd: target_cwd.clone(),
            workspace_submitting: false,
            entries: Vec::new(),
            selected: None,
            loading: true,
        });
        let kind = match kind {
            CreationKind::Workspace => PresetDefaultKind::Workspace,
            CreationKind::WorktreeCreate | CreationKind::WorktreeOpen => {
                PresetDefaultKind::Worktree
            }
        };
        if !self.push_endpoint_method_with_kind(
            Method::PresetDefault(PresetDefaultParams {
                kind,
                workspace_id: source,
                cwd: target_cwd,
            }),
            PendingEndpointKind::Presets(PresetsRequest::Creation(CreationRequest::Default(
                generation,
            ))),
            outcome,
        ) {
            self.fork.creation = None;
            if matches!(self.overlay, Some(ClientShellOverlay::Presets(_))) {
                self.overlay = None;
            }
        }
        outcome.repaint = true;
    }

    fn creation_form_matches(&self, choice: &CreationState) -> bool {
        if choice.endpoint_id != self.active_endpoint_id {
            return false;
        }
        match (choice.kind, self.overlay.as_ref()) {
            (_, Some(ClientShellOverlay::Presets(list))) if list.creation.is_some() => true,
            (
                CreationKind::Workspace,
                Some(ClientShellOverlay::Rename(ClientRenameOverlay {
                    target:
                        ClientRenameTarget::NewWorkspace {
                            source_workspace_id,
                            ..
                        },
                    ..
                })),
            ) => *source_workspace_id == choice.source_workspace_id,
            (CreationKind::WorktreeCreate, Some(ClientShellOverlay::WorktreeCreate(form))) => {
                Some(&form.source_workspace_id) == choice.source_workspace_id.as_ref()
            }
            (CreationKind::WorktreeOpen, Some(ClientShellOverlay::WorktreeOpen(form))) => {
                Some(&form.source_workspace_id) == choice.source_workspace_id.as_ref()
            }
            _ => false,
        }
    }

    fn creation_busy(&self) -> bool {
        match self.overlay.as_ref() {
            Some(ClientShellOverlay::WorktreeCreate(form)) => form.creating,
            Some(ClientShellOverlay::WorktreeOpen(form)) => form.opening,
            _ => false,
        }
    }

    fn open_creation_picker(&mut self, outcome: &mut ClientShellInput) {
        let Some(choice) = self.fork.creation.as_ref().filter(|choice| {
            !choice.loading && !choice.entries.is_empty() && self.creation_form_matches(choice)
        }) else {
            return;
        };
        let mut list = PresetsOverlay::new(choice.source_workspace_id.clone());
        list.entries = std::iter::once(PresetSummary {
            name: "None".to_owned(),
            path: String::new(),
            label: Some("plain shell".to_owned()),
            panes: 1,
            params: Vec::new(),
            error: None,
        })
        .chain(choice.entries.iter().cloned())
        .collect();
        list.selected = choice
            .selected
            .as_ref()
            .and_then(|name| list.entries.iter().position(|entry| &entry.name == name))
            .unwrap_or(0);
        list.loading = false;
        list.creation = Some(CreationPicker {
            previous: self.overlay.take().map(Box::new),
        });
        self.overlay = Some(ClientShellOverlay::Presets(list));
        outcome.repaint = true;
    }

    pub(in crate::client::shell) fn close_creation_picker(&mut self) {
        let previous = match self.overlay.take() {
            Some(ClientShellOverlay::Presets(mut list)) => {
                list.creation.take().and_then(|picker| picker.previous)
            }
            _ => None,
        };
        self.overlay = previous.map(|overlay| *overlay);
        if self.overlay.is_none() {
            self.fork.creation = None;
        }
    }

    /// A picker embedded in a form returns to that form; a standalone picker
    /// creates the workspace. The synthetic first entry means no preset.
    pub(in crate::client::shell) fn accept_creation_preset(
        &mut self,
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_ref() else {
            return false;
        };
        if list.creation.is_none() {
            return false;
        }
        if list.loading {
            return true;
        }
        let Some(choice) = self.fork.creation.as_mut() else {
            return true;
        };
        let Some(index) = list.selected_index() else {
            return true;
        };
        choice.selected = (index != 0)
            .then(|| list.entries.get(index))
            .flatten()
            .map(|entry| entry.name.clone());
        let Some(ClientShellOverlay::Presets(mut list)) = self.overlay.take() else {
            return true;
        };
        self.overlay = list
            .creation
            .take()
            .and_then(|picker| picker.previous)
            .map(|overlay| *overlay);
        if self.overlay.is_none() {
            self.submit_preset_workspace(outcome);
        }
        outcome.repaint = true;
        true
    }

    fn submit_preset_workspace(&mut self, outcome: &mut ClientShellInput) {
        let source_workspace_id = self
            .fork
            .creation
            .as_ref()
            .and_then(|choice| choice.source_workspace_id.clone());
        let cwd = self.fork.creation.as_mut().and_then(|choice| {
            choice.workspace_submitting = true;
            choice.target_cwd.clone()
        });
        self.push_endpoint_method(
            Method::WorkspaceCreate(WorkspaceCreateParams {
                source_workspace_id,
                cwd,
                focus: true,
                label: None,
                env: Default::default(),
            }),
            outcome,
        );
        self.fork.creation = None;
    }

    pub(in crate::client::shell) fn route_creation_preset_key(
        &mut self,
        key: &crate::input::TerminalKey,
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some(choice) = self
            .fork
            .creation
            .as_ref()
            .filter(|choice| self.creation_form_matches(choice))
        else {
            return false;
        };
        if matches!(self.overlay, Some(ClientShellOverlay::Presets(_))) {
            return false;
        }
        if key.code == KeyCode::Esc && !self.creation_busy() {
            self.fork.creation = None;
            return false;
        }
        if key.code == KeyCode::Enter && choice.loading {
            outcome.repaint = true;
            return true;
        }
        if key.code == KeyCode::Enter && matches!(self.overlay, Some(ClientShellOverlay::Rename(_)))
        {
            if let Some(choice) = self.fork.creation.as_mut() {
                choice.workspace_submitting = true;
            }
        }
        if key.code == KeyCode::Tab && !self.creation_busy() {
            self.open_creation_picker(outcome);
            return true;
        }
        false
    }

    pub(in crate::client::shell) fn handle_creation_preset_mouse(
        &mut self,
        mouse: &crossterm::event::MouseEvent,
        point: (u16, u16),
        outcome: &mut ClientShellInput,
    ) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};
        let Some(choice) = self
            .fork
            .creation
            .as_ref()
            .filter(|choice| self.creation_form_matches(choice))
        else {
            return false;
        };
        if mouse.kind != MouseEventKind::Down(MouseButton::Left)
            || matches!(self.overlay, Some(ClientShellOverlay::Presets(_)))
        {
            return false;
        }
        if !self.creation_busy() && super::super::contains(self.hits.overlay_cancel, point) {
            self.fork.creation = None;
            return false;
        }
        if choice.loading
            && (super::super::contains(self.hits.overlay_primary, point)
                || self
                    .hits
                    .worktree_rows
                    .iter()
                    .any(|(rect, _)| super::super::contains(*rect, point)))
        {
            return true;
        }
        if matches!(self.overlay, Some(ClientShellOverlay::Rename(_)))
            && super::super::contains(self.hits.overlay_primary, point)
        {
            if let Some(choice) = self.fork.creation.as_mut() {
                choice.workspace_submitting = true;
            }
        }
        if !self.creation_busy() && super::super::contains(self.hits.fork.creation_preset, point) {
            self.open_creation_picker(outcome);
            return true;
        }
        false
    }

    /// Runs only for the one visible modal, with no work added to pane loops.
    pub(in crate::client::shell) fn render_creation_preset(
        &self,
        b: &mut Buffer,
        area: Rect,
    ) -> Rect {
        let Some(choice) = self.fork.creation.as_ref().filter(|choice| {
            self.creation_form_matches(choice) && (choice.loading || !choice.entries.is_empty())
        }) else {
            return Rect::default();
        };
        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        let y = match self.overlay.as_ref() {
            Some(ClientShellOverlay::Rename(_)) => inner.y + 1,
            Some(ClientShellOverlay::WorktreeCreate(_)) => inner.y + 7,
            Some(ClientShellOverlay::WorktreeOpen(_)) => inner.bottom().saturating_sub(2),
            _ => return Rect::default(),
        };
        if y >= inner.bottom() {
            return Rect::default();
        }
        let rect = Rect::new(inner.x, y, inner.width, 1);
        let label = if choice.loading {
            " preset: loading…".to_owned()
        } else {
            format!(
                " preset: {} ▾  (Tab)",
                choice.selected.as_deref().unwrap_or("None")
            )
        };
        let p = &self.config.palette;
        let style = Style::default()
            .fg(p.subtext0)
            .bg(p.surface0)
            .remove_modifier(Modifier::DIM);
        b.set_style(rect, style);
        render::put_text(b, rect.x, rect.y, rect.width, &label, style);
        rect
    }

    fn clear_creation_generation(&mut self, generation: u64) -> bool {
        let owns = self
            .fork
            .creation
            .as_ref()
            .is_some_and(|choice| choice.generation == generation);
        if owns {
            self.fork.creation = None;
        }
        owns
    }

    pub(in crate::client::shell) fn creation_preset_request(
        &mut self,
        method: &Method,
        kind: PendingEndpointKind,
    ) -> PendingEndpointKind {
        let Some(choice) = self.fork.creation.as_ref() else {
            return kind;
        };
        if choice.endpoint_id != self.active_endpoint_id
            || (!self.creation_form_matches(choice) && !choice.workspace_submitting)
        {
            self.fork.creation = None;
            return kind;
        }
        if choice.loading {
            return kind;
        }
        let matches = match (choice.kind, method, &kind) {
            (
                CreationKind::Workspace,
                Method::WorkspaceCreate(params),
                PendingEndpointKind::Generic,
            ) => {
                choice.workspace_submitting
                    && params.source_workspace_id == choice.source_workspace_id
            }
            (
                CreationKind::WorktreeCreate,
                Method::WorktreeCreate(params),
                PendingEndpointKind::WorktreeCreate,
            ) => params.workspace_id == choice.source_workspace_id,
            (
                CreationKind::WorktreeOpen,
                Method::WorktreeOpen(params),
                PendingEndpointKind::WorktreeOpen,
            ) => params.workspace_id == choice.source_workspace_id,
            _ => false,
        };
        if !matches {
            return kind;
        }
        let generation = choice.generation;
        let creation_kind = choice.kind;
        let request = CreationRequest::Created {
            generation,
            kind: creation_kind,
            preset: choice.selected.clone(),
        };
        // The name form or standalone picker has already been submitted. The
        // pending request owns the selection now, not a future creation form.
        if matches!(creation_kind, CreationKind::Workspace) {
            self.clear_creation_generation(generation);
        }
        PendingEndpointKind::Presets(PresetsRequest::Creation(request))
    }

    /// Only successful creates with a preset need the custom apply callback.
    /// Every other result keeps the base endpoint notices and form handling.
    pub(in crate::client::shell) fn reconcile_creation_result(
        &mut self,
        mut pending: PendingEndpointRequest,
        result: &Result<ResponseResult, ClientShellEndpointError>,
    ) -> PendingEndpointRequest {
        pending.kind = match pending.kind {
            PendingEndpointKind::Presets(PresetsRequest::Creation(CreationRequest::Created {
                generation,
                kind,
                preset,
            })) => {
                if preset.is_some() && creation_result_tab(kind, result).is_some() {
                    PendingEndpointKind::Presets(PresetsRequest::Creation(
                        CreationRequest::Created {
                            generation,
                            kind,
                            preset,
                        },
                    ))
                } else {
                    let owns = self
                        .fork
                        .creation
                        .as_ref()
                        .is_some_and(|choice| choice.generation == generation);
                    if result.is_ok() || matches!(kind, CreationKind::Workspace) {
                        self.clear_creation_generation(generation);
                    }
                    match kind {
                        CreationKind::WorktreeCreate if owns => PendingEndpointKind::WorktreeCreate,
                        CreationKind::WorktreeOpen if owns => PendingEndpointKind::WorktreeOpen,
                        _ => PendingEndpointKind::Generic,
                    }
                }
            }
            kind => kind,
        };
        pending
    }

    fn finish_creation_choices(
        &mut self,
        entries: Vec<PresetSummary>,
        outcome: &mut ClientShellInput,
    ) {
        let Some(choice) = self.fork.creation.as_mut() else {
            return;
        };
        choice.loading = false;
        choice.entries = entries
            .into_iter()
            .filter(|entry| {
                entry.error.is_none()
                    && entry
                        .params
                        .iter()
                        .all(|param| !param.required || param.default.is_some())
            })
            .collect();
        if !choice
            .entries
            .iter()
            .any(|entry| Some(&entry.name) == choice.selected.as_ref())
        {
            choice.selected = None;
        }
        let standalone = matches!(self.overlay.as_ref(), Some(ClientShellOverlay::Presets(list)) if list.creation.as_ref().is_some_and(|picker| picker.previous.is_none()));
        if standalone {
            self.overlay = None;
            if choice.entries.is_empty() {
                self.submit_preset_workspace(outcome);
            } else {
                // open_creation_picker needs a live form; keep the loading
                // picker's context while replacing it with the populated one.
                let mut list = PresetsOverlay::new(choice.source_workspace_id.clone());
                list.creation = Some(CreationPicker { previous: None });
                self.overlay = Some(ClientShellOverlay::Presets(list));
                self.open_creation_picker(outcome);
                if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                    if let Some(picker) = list.creation.as_mut() {
                        picker.previous = None;
                    }
                }
            }
        }
        outcome.repaint = true;
    }

    pub(in crate::client::shell) fn receive_creation_preset_result(
        &mut self,
        request: CreationRequest,
        result: Result<ResponseResult, ClientShellEndpointError>,
        outcome: &mut ClientShellInput,
    ) -> bool {
        match request {
            CreationRequest::Default(generation) | CreationRequest::List(generation) => {
                if !self.fork.creation.as_ref().is_some_and(|choice| {
                    choice.generation == generation && self.creation_form_matches(choice)
                }) {
                    return false;
                }
                match (request, result) {
                    (CreationRequest::Default(_), Ok(ResponseResult::PresetDefault { name })) => {
                        if let Some(choice) = self.fork.creation.as_mut() {
                            choice.selected = name;
                        }
                        if !self.push_endpoint_method_with_kind(
                            Method::PresetList(EmptyParams {}),
                            PendingEndpointKind::Presets(PresetsRequest::Creation(
                                CreationRequest::List(generation),
                            )),
                            outcome,
                        ) {
                            self.finish_creation_choices(Vec::new(), outcome);
                        }
                    }
                    (CreationRequest::List(_), Ok(ResponseResult::PresetList { presets })) => {
                        self.finish_creation_choices(presets, outcome)
                    }
                    (_, Err(error))
                        if matches!(
                            error.code.as_deref(),
                            Some("unknown_method" | "method_not_found" | "unsupported_method")
                        ) =>
                    {
                        self.finish_creation_choices(Vec::new(), outcome)
                    }
                    (_, Err(error)) if error.code.as_deref() == Some("endpoint_cancelled") => {
                        self.clear_creation_generation(generation);
                        if matches!(self.overlay, Some(ClientShellOverlay::Presets(_))) {
                            self.overlay = None;
                        }
                    }
                    (_, Err(error)) => {
                        self.push_endpoint_notice(
                            ClientEndpointNoticeKind::Rejected,
                            "preset.default".to_owned(),
                            "Preset list unavailable",
                            error.message,
                        );
                        self.finish_creation_choices(Vec::new(), outcome);
                    }
                    _ => {
                        self.fork.creation = None;
                        if matches!(self.overlay, Some(ClientShellOverlay::Presets(_))) {
                            self.overlay = None;
                        }
                        self.set_endpoint_error("endpoint returned an unexpected preset result");
                    }
                }
                true
            }
            CreationRequest::Created {
                generation,
                kind,
                preset,
            } => {
                let Some(tab_id) = creation_result_tab(kind, &result).map(str::to_owned) else {
                    return false; // Non-applied results use the base handler.
                };
                let Some(preset) = preset else {
                    return false;
                };
                let owns = self.clear_creation_generation(generation);
                if owns && !matches!(kind, CreationKind::Workspace) {
                    self.overlay = None;
                }
                self.push_endpoint_method_with_kind(
                    Method::PresetApply(PresetApplyParams {
                        name: preset,
                        tab_id,
                        focus: true,
                        params: BTreeMap::new(),
                    }),
                    PendingEndpointKind::Presets(PresetsRequest::Creation(CreationRequest::Apply)),
                    outcome,
                );
                true
            }
            CreationRequest::Apply => {
                match result {
                    Ok(ResponseResult::LayoutApply { .. }) => {}
                    Err(error) => {
                        self.push_endpoint_notice(
                            ClientEndpointNoticeKind::Rejected,
                            "preset.apply".to_owned(),
                            "Preset not applied",
                            error.message,
                        );
                    }
                    _ => self.set_endpoint_error("endpoint returned an unexpected preset result"),
                }
                true
            }
        }
    }
}

fn creation_result_tab(
    kind: CreationKind,
    result: &Result<ResponseResult, ClientShellEndpointError>,
) -> Option<&str> {
    match (kind, result.as_ref().ok()?) {
        (CreationKind::Workspace, ResponseResult::WorkspaceCreated { tab, .. })
        | (CreationKind::WorktreeCreate, ResponseResult::WorktreeCreated { tab, .. })
        | (
            CreationKind::WorktreeOpen,
            ResponseResult::WorktreeOpened {
                tab,
                already_open: false,
                ..
            },
        ) => Some(&tab.tab_id),
        _ => None,
    }
}
