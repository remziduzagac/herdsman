//! Tab presets: a tab's layout saved under a name in the server's config
//! folder, and opened again as a new tab in any workspace.
//!
//! Saving records each pane's label and what to start in it: a detected
//! agent's command, or the command the pane was launched with. Folders are not
//! recorded; loading starts every pane in the workspace's folder, or in the
//! folder a pane's `cwd` names, then types each pane's command into its shell.
//!
//! herdsman's own module. `layouts.rs` and `app/api.rs` reach it through hooks
//! marked `// fork: presets`.

mod file;
mod store;

use bytes::Bytes;

use super::*;
use crate::api::schema::{
    PresetDescription, PresetLoadParams, PresetNameParams, PresetNode, PresetPane,
    PresetRenameParams, PresetSaveParams, PresetSummary, SuccessResponse,
};
use file::PresetFile;
use store::{PresetStore, StoreError};

fn preset_store() -> PresetStore {
    #[cfg(test)]
    {
        PresetStore::new(tests::preset_dir())
    }
    #[cfg(not(test))]
    {
        PresetStore::new(crate::config::config_dir().join("presets"))
    }
}

fn store_error(id: String, err: StoreError) -> String {
    encode_error(id, err.code(), err.message())
}

fn describe(store: &PresetStore, name: &str, preset: PresetFile) -> PresetDescription {
    PresetDescription {
        name: name.to_string(),
        label: preset.label,
        path: store.path(name).display().to_string(),
        root: preset.root,
    }
}

impl App {
    pub(in crate::app::api) fn handle_preset_list(&mut self, id: String) -> String {
        let presets = match preset_store().list() {
            Ok(presets) => presets,
            Err(err) => return store_error(id, err),
        };
        let presets = presets
            .into_iter()
            .map(|(name, preset)| match preset {
                Ok(preset) => PresetSummary {
                    name,
                    panes: preset.root.pane_count(),
                    label: preset.label,
                    error: None,
                },
                Err(error) => PresetSummary {
                    name,
                    label: None,
                    panes: 0,
                    error: Some(error),
                },
            })
            .collect();
        encode_success(id, ResponseResult::PresetList { presets })
    }

    pub(in crate::app::api) fn handle_preset_get(
        &mut self,
        id: String,
        params: PresetNameParams,
    ) -> String {
        let store = preset_store();
        match store.read(&params.name) {
            Ok(preset) => {
                let preset = describe(&store, &params.name, preset);
                encode_success(id, ResponseResult::PresetInfo { preset })
            }
            Err(err) => store_error(id, err),
        }
    }

    pub(in crate::app::api) fn handle_preset_save(
        &mut self,
        id: String,
        params: PresetSaveParams,
    ) -> String {
        let target = match params.tab_id.as_deref() {
            Some(tab_id) => self.parse_tab_id(tab_id),
            None => self
                .state
                .active
                .and_then(|ws_idx| Some((ws_idx, self.state.workspaces.get(ws_idx)?.active_tab))),
        };
        let Some((ws_idx, tab_idx)) = target else {
            return encode_error(id, "tab_not_found", "tab not found");
        };
        let Some(layout) = self.layout_description(ws_idx, tab_idx) else {
            return encode_error(id, "layout_not_found", "layout unavailable");
        };
        let preset = PresetFile {
            label: self.state.workspaces[ws_idx].tabs[tab_idx]
                .custom_name
                .clone(),
            root: self.preset_node(&layout.root),
        };

        let store = preset_store();
        if let Err(err) = store.write(&params.name, &preset, params.overwrite) {
            return store_error(id, err);
        }
        let preset = describe(&store, &params.name, preset);
        encode_success(id, ResponseResult::PresetInfo { preset })
    }

    /// Opens the preset through `layout.apply` and answers as it does, then
    /// types each pane's command into the shell that started there.
    pub(in crate::app::api) fn handle_preset_load(
        &mut self,
        id: String,
        params: PresetLoadParams,
    ) -> String {
        let preset = match preset_store().read(&params.name) {
            Ok(preset) => preset,
            Err(err) => return store_error(id, err),
        };
        let ws_idx = match params.workspace_id.as_deref() {
            Some(workspace_id) => match self.parse_workspace_id(workspace_id) {
                Some(ws_idx) => ws_idx,
                None => {
                    return encode_error(
                        id,
                        "workspace_not_found",
                        format!("workspace {workspace_id} not found"),
                    )
                }
            },
            None => match self.state.active {
                Some(ws_idx) => ws_idx,
                None => return encode_error(id, "workspace_not_found", "no active workspace"),
            },
        };
        let folder = self
            .seed_cwd_from_workspace(ws_idx)
            .unwrap_or_else(|| self.state.workspaces[ws_idx].identity_cwd.clone());

        let response = self.handle_layout_apply(
            id,
            LayoutApplyParams {
                workspace_id: Some(self.public_workspace_id(ws_idx)),
                tab_id: None,
                tab_label: preset.label.clone(),
                focus: params.focus,
                root: layout_node(&preset.root, &folder),
            },
        );
        if let Ok(SuccessResponse {
            result: ResponseResult::LayoutApply { layout },
            ..
        }) = serde_json::from_str(&response)
        {
            self.type_preset_commands(&preset.root, &layout.root);
        }
        response
    }

    pub(in crate::app::api) fn handle_preset_rename(
        &mut self,
        id: String,
        params: PresetRenameParams,
    ) -> String {
        let store = preset_store();
        if let Err(err) = store.rename(&params.name, &params.new_name) {
            return store_error(id, err);
        }
        match store.read(&params.new_name) {
            Ok(preset) => {
                let preset = describe(&store, &params.new_name, preset);
                encode_success(id, ResponseResult::PresetInfo { preset })
            }
            Err(err) => store_error(id, err),
        }
    }

    pub(in crate::app::api) fn handle_preset_delete(
        &mut self,
        id: String,
        params: PresetNameParams,
    ) -> String {
        match preset_store().delete(&params.name) {
            Ok(()) => encode_success(id, ResponseResult::PresetDeleted { name: params.name }),
            Err(err) => store_error(id, err),
        }
    }

    fn preset_node(&self, node: &LayoutNode) -> PresetNode {
        match node {
            LayoutNode::Pane { pane } => PresetNode::Pane {
                pane: self.preset_pane(pane),
            },
            LayoutNode::Split {
                direction,
                ratio,
                first,
                second,
            } => PresetNode::Split {
                direction: direction.clone(),
                ratio: *ratio,
                first: Box::new(self.preset_node(first)),
                second: Box::new(self.preset_node(second)),
            },
            LayoutNode::Stack { panes, active } => PresetNode::Stack {
                panes: panes.iter().map(|pane| self.preset_pane(pane)).collect(),
                active: *active,
            },
        }
    }

    fn preset_pane(&self, pane: &LayoutPane) -> PresetPane {
        let agent = pane
            .pane_id
            .as_deref()
            .and_then(|pane_id| self.pane_agent(pane_id));
        PresetPane {
            label: pane.label.clone(),
            cwd: None,
            command: match agent {
                Some(agent) => Some(crate::detect::interactive_agent_executable(agent).to_string()),
                None => pane.command.as_deref().map(command_line),
            },
            env: Default::default(),
        }
    }

    fn pane_agent(&self, public_pane_id: &str) -> Option<crate::detect::Agent> {
        let (ws_idx, pane_id) = self.parse_pane_id(public_pane_id)?;
        let ws = self.state.workspaces.get(ws_idx)?;
        let tab = ws.tabs.get(ws.find_tab_index_for_pane(pane_id)?)?;
        let terminal = self.state.terminals.get(tab.terminal_id(pane_id)?)?;
        crate::detect::parse_agent_label(terminal.effective_agent_label()?)
    }

    /// Walks the preset and the tab `layout.apply` made from it together; both
    /// have the same shape.
    fn type_preset_commands(&self, preset: &PresetNode, created: &LayoutNode) {
        match (preset, created) {
            (PresetNode::Pane { pane }, LayoutNode::Pane { pane: created }) => {
                self.type_preset_command(pane, created);
            }
            (
                PresetNode::Split { first, second, .. },
                LayoutNode::Split {
                    first: created_first,
                    second: created_second,
                    ..
                },
            ) => {
                self.type_preset_commands(first, created_first);
                self.type_preset_commands(second, created_second);
            }
            (PresetNode::Stack { panes, .. }, LayoutNode::Stack { panes: created, .. }) => {
                for (pane, created) in panes.iter().zip(created) {
                    self.type_preset_command(pane, created);
                }
            }
            _ => tracing::warn!("preset and the tab made from it differ in shape"),
        }
    }

    fn type_preset_command(&self, pane: &PresetPane, created: &LayoutPane) {
        let Some(command) = pane
            .command
            .as_deref()
            .filter(|command| !command.trim().is_empty())
        else {
            return;
        };
        let Some((ws_idx, pane_id)) = created
            .pane_id
            .as_deref()
            .and_then(|pane_id| self.parse_pane_id(pane_id))
        else {
            return;
        };
        let Some(runtime) = self.lookup_runtime_sender(ws_idx, pane_id) else {
            return;
        };
        let typed = crate::app::api_helpers::encode_api_input(runtime, command, &["enter".into()])
            .map_err(|key| format!("unsupported key {key}"))
            .and_then(|bytes| {
                runtime
                    .validate_input_submission(&bytes)
                    .map_err(|err| err.to_string())?;
                runtime
                    .try_send_bytes(Bytes::from(bytes))
                    .map_err(|err| err.to_string())
            });
        if let Err(err) = typed {
            tracing::warn!(%err, "could not type a preset command into its pane");
        }
    }
}

/// The tree `layout.apply` gets: every pane a shell, started in `folder` or
/// in the folder its `cwd` names. Commands are typed in afterwards.
fn layout_node(node: &PresetNode, folder: &std::path::Path) -> LayoutNode {
    match node {
        PresetNode::Pane { pane } => LayoutNode::Pane {
            pane: layout_pane(pane, folder),
        },
        PresetNode::Split {
            direction,
            ratio,
            first,
            second,
        } => LayoutNode::Split {
            direction: direction.clone(),
            ratio: *ratio,
            first: Box::new(layout_node(first, folder)),
            second: Box::new(layout_node(second, folder)),
        },
        PresetNode::Stack { panes, active } => LayoutNode::Stack {
            panes: panes.iter().map(|pane| layout_pane(pane, folder)).collect(),
            active: *active,
        },
    }
}

fn layout_pane(pane: &PresetPane, folder: &std::path::Path) -> LayoutPane {
    LayoutPane {
        pane_id: None,
        label: pane.label.clone(),
        cwd: Some(
            pane_folder(pane.cwd.as_deref(), folder)
                .display()
                .to_string(),
        ),
        command: None,
        env: pane.env.clone(),
    }
}

/// `cwd` against the workspace's folder: relative to it, absolute, or under
/// `~`. A folder that does not exist falls back to the workspace's.
fn pane_folder(cwd: Option<&str>, folder: &std::path::Path) -> PathBuf {
    let Some(cwd) = cwd.filter(|cwd| !cwd.is_empty()) else {
        return folder.to_path_buf();
    };
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let path = match cwd.strip_prefix("~/") {
        Some(rest) => home().map(|home| home.join(rest)),
        None if cwd == "~" => home(),
        None => Some(folder.join(cwd)),
    };
    match path {
        Some(path) if path.is_dir() => path,
        _ => {
            tracing::warn!(cwd, folder = %folder.display(), "preset folder missing; using the workspace's");
            folder.to_path_buf()
        }
    }
}

/// An argv as a POSIX shell command line, quoting only what needs it.
fn command_line(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            let plain = !arg.is_empty()
                && arg
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./_-".contains(c));
            if plain {
                arg.clone()
            } else {
                format!("'{}'", arg.replace('\'', r"'\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;

    use super::super::tests::app_with_workspace;
    use super::*;
    use crate::api::schema::{ErrorResponse, Method, Request, SuccessResponse};

    thread_local! {
        static PRESET_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    /// This thread's preset folder: a fresh one under the temp folder, so no
    /// test ever reads or writes the real config folder.
    pub(super) fn preset_dir() -> PathBuf {
        PRESET_DIR.with(|dir| {
            dir.borrow_mut()
                .get_or_insert_with(|| {
                    let stamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos();
                    std::env::temp_dir()
                        .join(format!("herdsman-presets-{}-{stamp}", std::process::id()))
                })
                .clone()
        })
    }

    /// Removes this thread's preset folder when the test ends.
    struct CleanUp;

    impl Drop for CleanUp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(preset_dir());
        }
    }

    fn call(app: &mut App, method: Method) -> String {
        app.handle_api_request(Request {
            id: "t".into(),
            method,
        })
    }

    fn success(response: &str) -> ResponseResult {
        serde_json::from_str::<SuccessResponse>(response)
            .unwrap_or_else(|err| panic!("{err}: {response}"))
            .result
    }

    fn error_code(response: &str) -> String {
        serde_json::from_str::<ErrorResponse>(response)
            .unwrap_or_else(|err| panic!("{err}: {response}"))
            .error
            .code
    }

    fn save(app: &mut App, name: &str) -> String {
        call(
            app,
            Method::PresetSave(PresetSaveParams {
                name: name.into(),
                tab_id: None,
                overwrite: false,
            }),
        )
    }

    #[test]
    fn save_records_shape_labels_and_launch_commands_but_not_folders() {
        let _clean = CleanUp;
        let mut app = app_with_workspace();
        app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        app.state.ensure_test_terminals();
        app.state.workspaces[0].tabs[0].custom_name = Some("dev".into());

        let ResponseResult::PresetInfo { preset } = success(&save(&mut app, "dev")) else {
            panic!("expected preset info");
        };
        assert_eq!(preset.label.as_deref(), Some("dev"));
        assert!(preset.path.ends_with("dev.toml"), "{}", preset.path);
        let PresetNode::Split { first, second, .. } = preset.root else {
            panic!("expected a split");
        };
        for node in [*first, *second] {
            let PresetNode::Pane { pane } = node else {
                panic!("expected panes");
            };
            assert_eq!(pane.cwd, None);
        }

        let text = std::fs::read_to_string(preset_dir().join("dev.toml")).unwrap();
        assert!(text.contains("[root.first]"), "{text}");
    }

    #[test]
    fn save_refuses_a_taken_name_and_a_bad_one() {
        let _clean = CleanUp;
        let mut app = app_with_workspace();
        success(&save(&mut app, "dev"));
        assert_eq!(error_code(&save(&mut app, "dev")), "preset_exists");
        assert_eq!(
            error_code(&save(&mut app, "Dev Tab")),
            "invalid_preset_name"
        );
    }

    #[test]
    fn list_get_rename_and_delete() {
        let _clean = CleanUp;
        let mut app = app_with_workspace();
        success(&save(&mut app, "one"));
        std::fs::write(preset_dir().join("broken.toml"), "[root]\nratio = 2\n").unwrap();

        let ResponseResult::PresetList { presets } =
            success(&call(&mut app, Method::PresetList(Default::default())))
        else {
            panic!("expected a list");
        };
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].name, "broken");
        assert!(presets[0].error.is_some());
        assert_eq!((presets[1].name.as_str(), presets[1].panes), ("one", 1));

        let renamed = call(
            &mut app,
            Method::PresetRename(PresetRenameParams {
                name: "one".into(),
                new_name: "two".into(),
            }),
        );
        assert!(
            matches!(success(&renamed), ResponseResult::PresetInfo { preset } if preset.name == "two")
        );
        let get = |app: &mut App, name: &str| {
            call(
                app,
                Method::PresetGet(PresetNameParams { name: name.into() }),
            )
        };
        assert_eq!(error_code(&get(&mut app, "one")), "preset_not_found");
        assert!(matches!(
            success(&get(&mut app, "two")),
            ResponseResult::PresetInfo { .. }
        ));
        assert_eq!(error_code(&get(&mut app, "broken")), "invalid_preset");

        let delete = call(
            &mut app,
            Method::PresetDelete(PresetNameParams { name: "two".into() }),
        );
        assert!(
            matches!(success(&delete), ResponseResult::PresetDeleted { name } if name == "two")
        );
        assert_eq!(error_code(&get(&mut app, "two")), "preset_not_found");
    }

    #[tokio::test]
    async fn load_opens_a_new_tab_with_the_presets_shape_in_the_workspace_folder() {
        let _clean = CleanUp;
        let mut app = app_with_workspace();
        let folder = preset_dir().join("repo");
        std::fs::create_dir_all(folder.join("docs")).unwrap();
        app.state.workspaces[0].identity_cwd = folder.clone();
        std::fs::write(
            preset_dir().join("dev.toml"),
            "label = \"dev\"\n[root]\nsplit = \"right\"\nratio = 0.6\n\
             [root.first]\nlabel = \"agent\"\n\
             [root.second]\ncwd = \"docs\"\n",
        )
        .unwrap();

        let response = call(
            &mut app,
            Method::PresetLoad(PresetLoadParams {
                name: "dev".into(),
                workspace_id: None,
                focus: true,
            }),
        );
        let ResponseResult::LayoutApply { layout } = success(&response) else {
            panic!("expected the layout.apply answer");
        };
        assert_eq!(app.state.workspaces[0].tabs.len(), 2);
        assert_eq!(
            app.state.workspaces[0].tabs[1].custom_name.as_deref(),
            Some("dev")
        );
        let LayoutNode::Split {
            ratio,
            first,
            second,
            ..
        } = layout.root
        else {
            panic!("expected a split");
        };
        assert!((ratio - 0.6).abs() < 0.01);
        let LayoutNode::Pane { pane: first } = *first else {
            panic!("expected a pane");
        };
        assert_eq!(first.label.as_deref(), Some("agent"));
        let LayoutNode::Pane { pane: second } = *second else {
            panic!("expected a pane");
        };
        // Compared as a path: Windows reports a live folder with a trailing separator.
        assert!(Path::new(&second.cwd.unwrap()).ends_with("docs"));
        super::super::super::test_support::shutdown_test_runtimes(&mut app);
    }

    #[test]
    fn load_reports_a_missing_or_broken_preset() {
        let _clean = CleanUp;
        let mut app = app_with_workspace();
        let load = |app: &mut App, name: &str| {
            call(
                app,
                Method::PresetLoad(PresetLoadParams {
                    name: name.into(),
                    workspace_id: None,
                    focus: true,
                }),
            )
        };
        assert_eq!(error_code(&load(&mut app, "nope")), "preset_not_found");
        std::fs::create_dir_all(preset_dir()).unwrap();
        std::fs::write(preset_dir().join("bad.toml"), "[root]\nsplit = \"up\"\n").unwrap();
        assert_eq!(error_code(&load(&mut app, "bad")), "invalid_preset");
        assert_eq!(app.state.workspaces[0].tabs.len(), 1);
    }

    #[test]
    fn pane_folders_resolve_against_the_workspace_folder() {
        let _clean = CleanUp;
        let folder = preset_dir().join("ws");
        std::fs::create_dir_all(folder.join("docs")).unwrap();

        assert_eq!(pane_folder(None, &folder), folder);
        assert_eq!(pane_folder(Some("docs"), &folder), folder.join("docs"));
        assert_eq!(pane_folder(Some("missing"), &folder), folder);
        let absolute = folder.join("docs");
        assert_eq!(
            pane_folder(Some(absolute.to_str().unwrap()), &folder),
            absolute
        );
        assert_eq!(pane_folder(Some("."), &folder), folder.join("."));
        assert!(Path::new(&pane_folder(Some("~"), &folder)).is_absolute());
    }

    #[test]
    fn command_lines_quote_only_what_needs_it() {
        let argv = |parts: &[&str]| {
            parts
                .iter()
                .map(|part| part.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(command_line(&argv(&["just", "test"])), "just test");
        assert_eq!(
            command_line(&argv(&["git", "log", "--format=%h"])),
            "git log --format=%h"
        );
        assert_eq!(
            command_line(&argv(&["echo", "a b", "it's", ""])),
            "echo 'a b' 'it'\\''s' ''"
        );
    }
}
