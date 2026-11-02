//! `--preset <name>` on `workspace create`, `worktree create` and
//! `worktree open`: the new workspace's first tab becomes the preset.
//!
//! The options are taken out before the base project's parser sees the rest,
//! the preset and its values are checked before anything is created, and once
//! the create request answers, `preset.apply` replaces the new workspace's
//! shell tab. A worktree that was already open keeps its layout.
//!
//! herdsman's own module. `cli.rs` and `cli/runtime.rs` reach it through hooks
//! marked `// fork: presets`.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::api::schema::{Method, PresetApplyParams, Request, TabTarget, WorkspaceTarget};

/// A tab id no server has. `preset.apply` checks the preset and its values
/// before the tab, so applying to it answers `tab_not_found` exactly when the
/// preset would open, and an older server without the method says so.
const NO_TAB: &str = "";

/// The preset the running create command asked for, and its values.
struct Pending {
    name: String,
    params: BTreeMap<String, String>,
}

thread_local! {
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
}

/// Whether `args` (the whole command line) is a create command with
/// `--preset`.
pub(in crate::cli) fn requested(args: &[String]) -> bool {
    let creates = matches!(
        (
            args.get(1).map(String::as_str),
            args.get(2).map(String::as_str)
        ),
        (Some("workspace"), Some("create")) | (Some("worktree"), Some("create" | "open"))
    );
    creates && args[3..].iter().any(|arg| arg == "--preset")
}

/// Run the create command without `--preset`, then open the preset in it.
pub(in crate::cli) fn run(args: &[String]) -> std::io::Result<i32> {
    let (rest, pending) = match take_preset(&args[2..]) {
        Ok(taken) => taken,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };
    let checked = crate::cli::send_request(&apply_request(&pending, NO_TAB.into()))?;
    if checked["error"]["code"] != "tab_not_found" {
        return crate::cli::print_response(&checked);
    }

    PENDING.with(|slot| *slot.borrow_mut() = Some(pending));
    let code = match args[1].as_str() {
        "workspace" => crate::cli::workspace::run_workspace_command(&rest)?,
        _ => crate::cli::worktree::run_worktree_command(&rest)?,
    };
    PENDING.with(|slot| slot.borrow_mut().take());
    Ok(code)
}

/// `args` without `--preset <name>` and `--param <name>=<value>`, and what
/// they asked for.
fn take_preset(args: &[String]) -> Result<(Vec<String>, Pending), String> {
    let mut rest = Vec::with_capacity(args.len());
    let mut name = None;
    let mut params = BTreeMap::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--preset" => {
                let value = args.next().ok_or("missing value for --preset")?;
                if name.replace(value.clone()).is_some() {
                    return Err("--preset is given twice".into());
                }
            }
            "--param" => {
                let value = args.next().ok_or("missing value for --param")?;
                let (key, value) = value
                    .split_once('=')
                    .filter(|(key, _)| !key.is_empty())
                    .ok_or_else(|| format!("--param takes <name>=<value>, not {value}"))?;
                if params.insert(key.to_owned(), value.to_owned()).is_some() {
                    return Err(format!("--param {key} is given twice"));
                }
            }
            // The preset's panes replace the one shell that would get these.
            "--env" => return Err("--env cannot be combined with --preset".into()),
            _ => rest.push(arg.clone()),
        }
    }
    let name = name.ok_or("missing --preset")?;
    Ok((rest, Pending { name, params }))
}

fn apply_request(pending: &Pending, tab_id: String) -> Request {
    Request {
        id: "cli:preset:apply".into(),
        method: Method::PresetApply(PresetApplyParams {
            name: pending.name.clone(),
            tab_id,
            // Replacing keeps the tab active if it was; a --no-focus create
            // must not take focus here.
            focus: false,
            params: pending.params.clone(),
        }),
    }
}

/// Called with every response the base project's create commands print. With
/// a preset pending and a new workspace in `response`, open the preset in its
/// first tab: the result then describes the workspace and tab as they are
/// afterwards, and carries the preset's panes as `preset_layout` in place of
/// the replaced `root_pane`. When the preset cannot be applied, the create
/// result still goes to stdout, so the new workspace can be found, and the
/// error is returned.
pub(in crate::cli) fn after_create(
    mut response: serde_json::Value,
) -> std::io::Result<serde_json::Value> {
    let Some(pending) = PENDING.with(|slot| slot.borrow_mut().take()) else {
        return Ok(response);
    };
    let Some(tab_id) = new_workspace_tab(&response) else {
        return Ok(response);
    };
    let applied = crate::cli::send_request(&apply_request(&pending, tab_id))?;
    if applied.get("error").is_some() {
        println!("{response}");
        return Ok(applied);
    }
    let layout = &applied["result"]["layout"];
    let workspace = get_result(
        "cli:preset:workspace",
        Method::WorkspaceGet(WorkspaceTarget {
            workspace_id: layout["workspace_id"].as_str().unwrap_or_default().into(),
        }),
        "workspace",
    )?;
    let tab = get_result(
        "cli:preset:tab",
        Method::TabGet(TabTarget {
            tab_id: layout["tab_id"].as_str().unwrap_or_default().into(),
        }),
        "tab",
    )?;
    if let Some(result) = response
        .get_mut("result")
        .and_then(serde_json::Value::as_object_mut)
    {
        result.remove("root_pane");
        result.extend(workspace.map(|value| ("workspace".to_owned(), value)));
        result.extend(tab.map(|value| ("tab".to_owned(), value)));
        result.insert("preset_layout".into(), layout.clone());
    }
    Ok(response)
}

/// `field` of a get request's result, or None when it fails.
fn get_result(id: &str, method: Method, field: &str) -> std::io::Result<Option<serde_json::Value>> {
    let response = crate::cli::send_request(&Request {
        id: id.into(),
        method,
    })?;
    Ok(response["result"].get(field).cloned())
}

/// The first tab of the workspace a create response opened. None for an error,
/// another response, or a worktree whose workspace was already open.
fn new_workspace_tab(response: &serde_json::Value) -> Option<String> {
    let result = response.get("result")?;
    let new = match result.get("type")?.as_str()? {
        "workspace_created" | "worktree_created" => true,
        "worktree_opened" => !result.get("already_open")?.as_bool()?,
        _ => false,
    };
    if !new {
        return None;
    }
    result
        .get("tab")?
        .get("tab_id")?
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn only_create_commands_with_the_option_are_taken() {
        assert!(requested(&args("herdsman workspace create --preset dev")));
        assert!(requested(&args(
            "herdsman worktree create --branch x --preset dev"
        )));
        assert!(requested(&args(
            "herdsman worktree open --path /x --preset dev"
        )));
        assert!(!requested(&args("herdsman workspace create --label dev")));
        assert!(!requested(&args(
            "herdsman worktree remove w2 --preset dev"
        )));
        assert!(!requested(&args("herdsman preset load dev")));
    }

    #[test]
    fn the_options_are_taken_out_and_the_rest_kept_in_order() {
        let (rest, pending) = take_preset(&args(
            "create --cwd /x --preset pr --param pr=42 --no-focus --param note=a=b",
        ))
        .expect("taken");
        assert_eq!(rest, args("create --cwd /x --no-focus"));
        assert_eq!(pending.name, "pr");
        assert_eq!(pending.params.get("pr").map(String::as_str), Some("42"));
        assert_eq!(pending.params.get("note").map(String::as_str), Some("a=b"));
        for bad in [
            "create --preset",
            "create --preset a --preset b",
            "create --preset a --param",
            "create --preset a --param =1",
            "create --preset a --param x=1 --param x=2",
            "create --preset a --env FOO=bar",
        ] {
            assert!(take_preset(&args(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_new_workspace_gets_the_preset_and_an_open_one_does_not() {
        let created = json!({"result": {"type": "worktree_created", "tab": {"tab_id": "w2:t1"}}});
        assert_eq!(new_workspace_tab(&created).as_deref(), Some("w2:t1"));
        let reopened = json!({"result": {
            "type": "worktree_opened", "already_open": true, "tab": {"tab_id": "w2:t1"}
        }});
        assert_eq!(new_workspace_tab(&reopened), None);
        let opened = json!({"result": {
            "type": "worktree_opened", "already_open": false, "tab": {"tab_id": "w3:t1"}
        }});
        assert_eq!(new_workspace_tab(&opened).as_deref(), Some("w3:t1"));
        let failed = json!({"error": {"code": "x", "message": "y"}});
        assert_eq!(new_workspace_tab(&failed), None);
    }
}
