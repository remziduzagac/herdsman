//! Tab presets in the TUI: the presets list (`keys.presets`, `prefix+t`), the
//! save, replace and rename prompts, and the `preset.*` requests behind them.
//! The list opens a preset as a new tab, edits its file in a popup
//! (`preset.edit`), renames it, or deletes it once `y` confirms. A preset with
//! required parameters asks for each value first, one prompt at a time.
//!
//! herdsman's own module. `shell.rs`, `state.rs`, `actions.rs`,
//! `context_menu.rs`, `overlay_input.rs`, `input.rs`, `mouse.rs`,
//! `worktrees.rs` and `overlays.rs` reach it through hooks marked
//! `// fork: presets`.

use std::borrow::Cow;
use std::collections::BTreeMap;

use super::*;
use crate::api::schema::{
    EmptyParams, Method, PresetLoadParams, PresetNameParams, PresetParam, PresetRenameParams,
    PresetSaveParams, PresetSummary, ResponseResult,
};

const MAX_NAME_LEN: usize = 64;

/// The presets list.
#[derive(Debug)]
pub(super) struct PresetsOverlay {
    /// The workspace a preset opens in: the tab's when the list came from a
    /// tab's menu, else the focused one.
    pub(super) workspace_id: Option<String>,
    pub(super) entries: Vec<PresetSummary>,
    pub(super) selected: usize,
    pub(super) query: TextEditor,
    pub(super) search_focused: bool,
    /// Waiting for `preset.list`.
    pub(super) loading: bool,
    /// Waiting for a load or a delete; keys wait too.
    pub(super) busy: bool,
    pub(super) error: Option<String>,
    /// The preset `y` deletes; any other key keeps it.
    pub(super) confirm_delete: Option<String>,
    /// The preset to select once the list arrives, after a rename.
    pub(super) select_name: Option<String>,
}

impl PresetsOverlay {
    pub(super) fn new(workspace_id: Option<String>) -> Self {
        Self {
            workspace_id,
            entries: Vec::new(),
            selected: 0,
            query: TextEditor::default(),
            search_focused: false,
            loading: true,
            busy: false,
            error: None,
            confirm_delete: None,
            select_name: None,
        }
    }

    pub(super) fn filtered_indices(&self) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || format!(
                        "{} {} {}",
                        entry.name,
                        entry.label.as_deref().unwrap_or_default(),
                        entry.error.as_deref().unwrap_or_default()
                    )
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// The selected row among the filtered ones, or the first of them.
    pub(super) fn selected_index(&self) -> Option<usize> {
        let filtered = self.filtered_indices();
        filtered
            .contains(&self.selected)
            .then_some(self.selected)
            .or_else(|| filtered.first().copied())
    }

    pub(super) fn selected_entry(&self) -> Option<&PresetSummary> {
        self.selected_index()
            .and_then(|index| self.entries.get(index))
    }

    fn move_selection(&mut self, delta: isize) {
        let filtered = self.filtered_indices();
        let Some(last) = filtered.len().checked_sub(1) else {
            return;
        };
        let position = filtered
            .iter()
            .position(|index| *index == self.selected)
            .unwrap_or(0);
        let next = position.saturating_add_signed(delta).min(last);
        self.selected = filtered[next];
    }

    fn select_first_match(&mut self) {
        if let Some(first) = self.filtered_indices().first() {
            self.selected = *first;
        }
    }
}

/// What a `preset.*` request was for, so its answer can be handled.
#[derive(Debug)]
pub(super) enum PresetsRequest {
    List,
    Load,
    Delete,
    Save {
        tab_id: String,
        name: String,
        overwrite: bool,
    },
    Rename {
        workspace_id: Option<String>,
        name: String,
        new_name: String,
    },
}

/// What the one-line prompt does with its name.
#[derive(Debug)]
pub(super) enum PresetPrompt {
    /// Save the tab under a new name.
    Save { tab_id: String },
    /// The name was taken: Enter replaces that preset, unless the name was
    /// changed, which saves under the new name.
    Overwrite { tab_id: String, name: String },
    /// Rename a preset, then show the list again.
    Rename {
        name: String,
        workspace_id: Option<String>,
    },
    /// A value for `pending[0]`, a required parameter of `preset`; the rest
    /// of `pending` are asked next, then the preset opens with `given`.
    Value {
        preset: String,
        workspace_id: Option<String>,
        pending: Vec<PresetParam>,
        given: BTreeMap<String, String>,
    },
}

/// A prompt's title: a value prompt names the parameter and what it is for,
/// which a fixed title cannot.
pub(super) fn prompt_title(overlay: &ClientRenameOverlay) -> Cow<'static, str> {
    match &overlay.target {
        ClientRenameTarget::Preset(PresetPrompt::Value {
            preset, pending, ..
        }) => match pending.first() {
            Some(PresetParam {
                name,
                description: Some(description),
                ..
            }) => Cow::Owned(format!("{preset} · {name}: {description}")),
            Some(param) => Cow::Owned(format!("{preset} · {}", param.name)),
            None => Cow::Borrowed(overlay.title),
        },
        _ => Cow::Borrowed(overlay.title),
    }
}

/// The parameters a preset cannot open without: required, with no default.
fn values_to_ask(entry: &PresetSummary) -> Vec<PresetParam> {
    entry
        .params
        .iter()
        .filter(|param| param.required && param.default.is_none())
        .cloned()
        .collect()
}

/// A tab's name as a preset name: lowercase, with anything but letters,
/// digits, `-` and `_` turned into `-`.
pub(super) fn preset_name_from(label: &str) -> String {
    let name: String = label
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    name.trim_start_matches(['-', '_'])
        .chars()
        .take(MAX_NAME_LEN)
        .collect()
}

/// A key on the list, decided before acting so the list is no longer borrowed.
enum ListKey {
    Close,
    Load,
    Edit,
    Rename,
    Delete(String),
    Handled,
}

impl ClientShellState {
    /// Opens the presets list for `workspace_id`, or the focused workspace,
    /// and asks the server for its presets.
    pub(super) fn open_presets_list(
        &mut self,
        workspace_id: Option<String>,
        outcome: &mut ClientShellInput,
    ) {
        let workspace_id = workspace_id.or_else(|| {
            self.snapshot
                .as_deref()
                .and_then(|snapshot| snapshot.focused_workspace_id.clone())
        });
        self.overlay = Some(ClientShellOverlay::Presets(PresetsOverlay::new(
            workspace_id,
        )));
        if !self.request_preset_list(outcome) {
            // The endpoint's notice says why; an empty list would mislead.
            self.overlay = None;
        }
        outcome.repaint = true;
    }

    fn request_preset_list(&mut self, outcome: &mut ClientShellInput) -> bool {
        if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
            list.loading = true;
        }
        self.push_endpoint_method_with_kind(
            Method::PresetList(EmptyParams {}),
            PendingEndpointKind::Presets(PresetsRequest::List),
            outcome,
        )
    }

    /// "Save tab as preset...": a name prompt, suggesting the tab's own name
    /// when it has one.
    pub(super) fn open_save_preset_prompt(&mut self, tab_id: String) {
        let suggestion = self
            .snapshot
            .as_deref()
            .and_then(|snapshot| snapshot.tabs.iter().find(|tab| tab.tab_id == tab_id))
            .filter(|tab| tab.custom_label)
            .map(|tab| preset_name_from(&tab.label))
            .unwrap_or_default();
        self.open_preset_prompt(
            "save tab as preset",
            &suggestion,
            true,
            PresetPrompt::Save { tab_id },
        );
    }

    /// The one-line rename prompt, with a preset target.
    /// `replace_on_type` lets the first key replace a suggested name.
    fn open_preset_prompt(
        &mut self,
        title: &'static str,
        text: &str,
        replace_on_type: bool,
        prompt: PresetPrompt,
    ) {
        self.overlay = Some(ClientShellOverlay::Rename(ClientRenameOverlay {
            title,
            input: TextEditor::new(text, replace_on_type),
            target: ClientRenameTarget::Preset(prompt),
        }));
    }

    /// Enter in a preset prompt.
    pub(super) fn submit_preset_prompt(
        &mut self,
        prompt: PresetPrompt,
        name: &str,
        outcome: &mut ClientShellInput,
    ) {
        outcome.repaint = true;
        if name.is_empty() {
            // A required value cannot be empty: ask again. Esc leaves.
            if let PresetPrompt::Value {
                preset,
                workspace_id,
                pending,
                given,
            } = prompt
            {
                self.ask_preset_value(preset, workspace_id, pending, given);
            }
            return;
        }
        let name = name.to_owned();
        match prompt {
            PresetPrompt::Save { tab_id } => self.save_preset(tab_id, name, false, outcome),
            PresetPrompt::Overwrite {
                tab_id,
                name: taken,
            } => {
                let overwrite = name == taken;
                self.save_preset(tab_id, name, overwrite, outcome);
            }
            PresetPrompt::Value {
                preset,
                workspace_id,
                mut pending,
                mut given,
            } => {
                if pending.is_empty() {
                    return;
                }
                let param = pending.remove(0);
                given.insert(param.name, name);
                if pending.is_empty() {
                    self.load_preset(preset, workspace_id, given, outcome);
                } else {
                    self.ask_preset_value(preset, workspace_id, pending, given);
                }
            }
            PresetPrompt::Rename {
                name: old,
                workspace_id,
            } => {
                if old == name {
                    self.open_presets_list(workspace_id, outcome);
                    return;
                }
                self.push_endpoint_method_with_kind(
                    Method::PresetRename(PresetRenameParams {
                        name: old.clone(),
                        new_name: name.clone(),
                    }),
                    PendingEndpointKind::Presets(PresetsRequest::Rename {
                        workspace_id,
                        name: old,
                        new_name: name,
                    }),
                    outcome,
                );
            }
        }
    }

    fn save_preset(
        &mut self,
        tab_id: String,
        name: String,
        overwrite: bool,
        outcome: &mut ClientShellInput,
    ) {
        self.push_endpoint_method_with_kind(
            Method::PresetSave(PresetSaveParams {
                name: name.clone(),
                tab_id: Some(tab_id.clone()),
                pane_id: None,
                overwrite,
            }),
            PendingEndpointKind::Presets(PresetsRequest::Save {
                tab_id,
                name,
                overwrite,
            }),
            outcome,
        );
    }

    /// Keys on the presets list; false when the list is not open.
    pub(super) fn route_presets_key(
        &mut self,
        key: &crate::input::TerminalKey,
        outcome: &mut ClientShellInput,
    ) -> bool {
        use crossterm::event::KeyModifiers;

        let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() else {
            return false;
        };
        outcome.repaint = true;
        let (code, modifiers) = crate::config::normalize_key_combo((key.code, key.modifiers));
        if list.busy {
            return true;
        }
        if let Some(name) = list.confirm_delete.take() {
            if code == KeyCode::Char('y') && modifiers.is_empty() {
                list.busy = true;
                list.error = None;
                if !self.push_endpoint_method_with_kind(
                    Method::PresetDelete(PresetNameParams { name }),
                    PendingEndpointKind::Presets(PresetsRequest::Delete),
                    outcome,
                ) {
                    if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                        list.busy = false;
                    }
                }
            }
            return true;
        }
        if list.search_focused {
            if let Some(changed) = list.query.handle_key(key) {
                if changed {
                    list.select_first_match();
                }
                return true;
            }
        }
        let plain = !list.search_focused && modifiers.is_empty();
        let action = match code {
            KeyCode::Esc => ListKey::Close,
            KeyCode::Enter => ListKey::Load,
            KeyCode::Up => {
                list.move_selection(-1);
                ListKey::Handled
            }
            KeyCode::Down => {
                list.move_selection(1);
                ListKey::Handled
            }
            KeyCode::Char('n') if modifiers == KeyModifiers::CONTROL => {
                list.move_selection(1);
                ListKey::Handled
            }
            KeyCode::Char('p') if modifiers == KeyModifiers::CONTROL => {
                list.move_selection(-1);
                ListKey::Handled
            }
            KeyCode::Char('j') if plain => {
                list.move_selection(1);
                ListKey::Handled
            }
            KeyCode::Char('k') if plain => {
                list.move_selection(-1);
                ListKey::Handled
            }
            KeyCode::Char('/') if plain => {
                list.search_focused = true;
                ListKey::Handled
            }
            KeyCode::Char('e') if plain => ListKey::Edit,
            KeyCode::Char('r') if plain => ListKey::Rename,
            KeyCode::Char('d') if plain => match list.selected_entry() {
                Some(entry) => ListKey::Delete(entry.name.clone()),
                None => ListKey::Handled,
            },
            _ => ListKey::Handled,
        };
        match action {
            ListKey::Close => self.overlay = None,
            ListKey::Load => self.load_selected_preset(outcome),
            ListKey::Edit => self.edit_selected_preset(outcome),
            ListKey::Rename => self.rename_selected_preset(),
            ListKey::Delete(name) => {
                if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                    list.confirm_delete = Some(name);
                    list.error = None;
                }
            }
            ListKey::Handled => {}
        }
        true
    }

    /// Typed or pasted text for the list's search; false when the list is
    /// not open or not searching.
    pub(super) fn insert_presets_text(&mut self, text: &str) -> bool {
        match self.overlay.as_mut() {
            Some(ClientShellOverlay::Presets(list)) if list.search_focused && !list.busy => {
                if list.query.insert(text) {
                    list.select_first_match();
                }
                true
            }
            _ => false,
        }
    }

    /// Whether a paste goes to the list's search.
    pub(super) fn presets_accept_paste(&self) -> bool {
        matches!(
            self.overlay.as_ref(),
            Some(ClientShellOverlay::Presets(PresetsOverlay {
                search_focused: true,
                busy: false,
                ..
            }))
        )
    }

    fn load_selected_preset(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() else {
            return;
        };
        let Some(entry) = list.selected_entry() else {
            return;
        };
        if entry.error.is_some() {
            list.error = Some(format!("{} has a mistake; press e to fix it", entry.name));
            return;
        }
        let (name, workspace_id) = (entry.name.clone(), list.workspace_id.clone());
        let pending = values_to_ask(entry);
        if pending.is_empty() {
            self.load_preset(name, workspace_id, BTreeMap::new(), outcome);
        } else {
            outcome.repaint = true;
            self.ask_preset_value(name, workspace_id, pending, BTreeMap::new());
        }
    }

    /// The prompt for `pending[0]`, in place of the list.
    fn ask_preset_value(
        &mut self,
        preset: String,
        workspace_id: Option<String>,
        pending: Vec<PresetParam>,
        given: BTreeMap<String, String>,
    ) {
        self.open_preset_prompt(
            "preset value",
            "",
            false,
            PresetPrompt::Value {
                preset,
                workspace_id,
                pending,
                given,
            },
        );
    }

    fn load_preset(
        &mut self,
        name: String,
        workspace_id: Option<String>,
        given: BTreeMap<String, String>,
        outcome: &mut ClientShellInput,
    ) {
        let params = PresetLoadParams {
            name,
            workspace_id,
            pane_id: None,
            focus: true,
            params: given,
        };
        if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
            list.busy = true;
            list.error = None;
        }
        if !self.push_endpoint_method_with_kind(
            Method::PresetLoad(params),
            PendingEndpointKind::Presets(PresetsRequest::Load),
            outcome,
        ) {
            if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                list.busy = false;
            }
        }
    }

    /// Opens the preset's file in the editor in a popup on the server's
    /// machine. The list closes so the popup is in front; reopening it shows
    /// any mistake left in the file.
    fn edit_selected_preset(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_ref() else {
            return;
        };
        let Some(name) = list.selected_entry().map(|entry| entry.name.clone()) else {
            return;
        };
        self.overlay = None;
        self.popup_pending = true;
        self.popup_pending_deadline = None;
        if !self.push_endpoint_method_with_kind(
            Method::PresetEdit(PresetNameParams { name }),
            PendingEndpointKind::PopupCommand,
            outcome,
        ) {
            self.popup_pending = false;
        }
    }

    fn rename_selected_preset(&mut self) {
        let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_ref() else {
            return;
        };
        let Some(name) = list.selected_entry().map(|entry| entry.name.clone()) else {
            return;
        };
        let workspace_id = list.workspace_id.clone();
        self.open_preset_prompt(
            "rename preset",
            &name.clone(),
            false,
            PresetPrompt::Rename { name, workspace_id },
        );
    }

    /// Mouse on the list; false when the list is not open. A click selects a
    /// row, a click on the selected row or on the open button opens it, and
    /// the wheel moves the selection. The rows and the search box reuse the
    /// worktree picker's click targets, which only one list fills at a time.
    pub(super) fn handle_presets_mouse(
        &mut self,
        mouse: &crossterm::event::MouseEvent,
        point: (u16, u16),
        outcome: &mut ClientShellInput,
    ) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};

        let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() else {
            return false;
        };
        if list.busy {
            return true;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                list.move_selection(-1);
                outcome.repaint = true;
            }
            MouseEventKind::ScrollDown => {
                list.move_selection(1);
                outcome.repaint = true;
            }
            MouseEventKind::Down(MouseButton::Left) => {
                outcome.repaint = true;
                list.confirm_delete = None;
                if super::contains(self.hits.overlay_cancel, point) {
                    self.overlay = None;
                } else if super::contains(self.hits.worktree_search, point) {
                    list.search_focused = true;
                } else if let Some((_, index)) = self
                    .hits
                    .worktree_rows
                    .iter()
                    .find(|(rect, _)| super::contains(*rect, point))
                    .copied()
                {
                    if list.selected_index() == Some(index) {
                        self.load_selected_preset(outcome);
                    } else {
                        list.selected = index;
                    }
                } else if super::contains(self.hits.overlay_primary, point) {
                    self.load_selected_preset(outcome);
                }
            }
            _ => {}
        }
        true
    }

    /// The answer to a `preset.*` request. Errors are shown here rather than
    /// as the usual notice, where the list or a prompt can say what to do.
    pub(super) fn receive_presets_result(
        &mut self,
        request: PresetsRequest,
        result: Result<ResponseResult, ClientShellEndpointError>,
        outcome: &mut ClientShellInput,
    ) -> bool {
        match (request, result) {
            (PresetsRequest::List, Ok(ResponseResult::PresetList { presets })) => {
                let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() else {
                    return false;
                };
                list.loading = false;
                list.entries = presets;
                if let Some(name) = list.select_name.take() {
                    if let Some(index) = list.entries.iter().position(|entry| entry.name == name) {
                        list.selected = index;
                    }
                }
                if list.selected >= list.entries.len() {
                    list.selected = 0;
                }
                true
            }
            (PresetsRequest::Load, Ok(ResponseResult::LayoutApply { .. })) => {
                if matches!(self.overlay, Some(ClientShellOverlay::Presets(_))) {
                    self.overlay = None;
                }
                true
            }
            (PresetsRequest::Delete, Ok(ResponseResult::PresetDeleted { .. })) => {
                if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                    list.busy = false;
                }
                self.request_preset_list(outcome);
                true
            }
            (PresetsRequest::Save { .. }, Ok(ResponseResult::PresetInfo { .. })) => false,
            (
                PresetsRequest::Rename {
                    workspace_id,
                    new_name,
                    ..
                },
                Ok(ResponseResult::PresetInfo { .. }),
            ) => {
                self.open_presets_list(workspace_id, outcome);
                if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                    list.select_name = Some(new_name);
                }
                true
            }
            (
                PresetsRequest::Save {
                    tab_id,
                    name,
                    overwrite: false,
                },
                Err(error),
            ) if error.code.as_deref() == Some("preset_exists") => {
                self.open_preset_prompt(
                    "preset exists · ↵ replaces it",
                    &name,
                    false,
                    PresetPrompt::Overwrite {
                        tab_id,
                        name: name.clone(),
                    },
                );
                true
            }
            (PresetsRequest::Save { tab_id, name, .. }, Err(error))
                if error.code.as_deref() == Some("invalid_preset_name") =>
            {
                self.open_preset_prompt(
                    "name: a-z, 0-9, - and _",
                    &name,
                    false,
                    PresetPrompt::Save { tab_id },
                );
                true
            }
            (
                PresetsRequest::Rename {
                    workspace_id, name, ..
                },
                Err(error),
            ) => {
                self.open_presets_list(workspace_id, outcome);
                if let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() {
                    list.select_name = Some(name);
                    list.error = Some(error.message);
                }
                true
            }
            (PresetsRequest::Save { .. }, Err(error)) => self.push_endpoint_notice(
                ClientEndpointNoticeKind::Rejected,
                format!("preset.save:{}", error.code.as_deref().unwrap_or("failed")),
                "Preset not saved",
                error.message,
            ),
            (_, Err(error)) => {
                let Some(ClientShellOverlay::Presets(list)) = self.overlay.as_mut() else {
                    return false;
                };
                list.loading = false;
                list.busy = false;
                list.error = Some(error.message);
                true
            }
            (_, Ok(_)) => {
                self.set_endpoint_error("endpoint returned an unexpected preset result");
                true
            }
        }
    }
}
