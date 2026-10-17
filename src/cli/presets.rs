//! `herdsman preset`: save a tab as a preset, open presets as new tabs, and
//! manage the saved ones.
//!
//! herdsman's own module. `cli.rs`, `cli/spec.rs` and `cli/target.rs` reach it
//! through hooks marked `// fork: presets`.

use std::path::Path;

use crate::api::schema::{
    EmptyParams, Method, PresetLoadParams, PresetNameParams, PresetRenameParams, PresetSaveParams,
    Request, ResponseResult, SuccessResponse,
};

const USAGE: &str = "\
herdsman preset commands:
  herdsman preset list
  herdsman preset show <name>
  herdsman preset save <name> [<tab_id>|--current] [--overwrite]
  herdsman preset load <name> [--workspace <workspace_id>|--current] [--no-focus]
  herdsman preset edit <name>
  herdsman preset rename <name> <new_name>
  herdsman preset delete <name>";

pub(super) fn run_preset_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        eprintln!("{USAGE}");
        return Ok(2);
    };
    match subcommand {
        "help" | "--help" | "-h" => {
            eprintln!("{USAGE}");
            Ok(0)
        }
        "edit" => match args[1..] {
            [ref name] => edit(name),
            _ => {
                eprintln!("{USAGE}");
                Ok(2)
            }
        },
        _ => {
            let env_pane_id = super::target::caller_pane_id();
            match parse(subcommand, &args[1..], env_pane_id.as_deref()) {
                Ok(method) => crate::cli::print_response(&crate::cli::send_request(&Request {
                    id: format!("cli:preset:{subcommand}"),
                    method,
                })?),
                Err(message) => {
                    eprintln!("{message}");
                    Ok(2)
                }
            }
        }
    }
}

/// The request for every subcommand but `edit`. `--current` targets the
/// calling pane's tab (save) or workspace (load).
fn parse(subcommand: &str, args: &[String], env_pane_id: Option<&str>) -> Result<Method, String> {
    let mut flags = Vec::new();
    let mut workspace = None;
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--workspace" if subcommand == "load" => {
                index += 1;
                let value = args.get(index).ok_or("missing value for --workspace")?;
                workspace = Some(super::normalize_workspace_id(value));
            }
            "--current" if matches!(subcommand, "save" | "load") => flags.push(arg),
            "--overwrite" if subcommand == "save" => flags.push(arg),
            "--no-focus" if subcommand == "load" => flags.push(arg),
            other if other.starts_with("--") => {
                return Err(format!("unknown option: {other}\n{USAGE}"))
            }
            other => positionals.push(other),
        }
        index += 1;
    }
    let current = flags.contains(&"--current");
    let caller_pane = || {
        env_pane_id
            .map(super::normalize_pane_id)
            .ok_or_else(|| "--current requires HERDSMAN_PANE_ID".to_string())
    };

    let method = match (subcommand, positionals.as_slice()) {
        ("list", []) => Method::PresetList(EmptyParams {}),
        ("show", [name]) => Method::PresetGet(PresetNameParams {
            name: (*name).into(),
        }),
        ("delete", [name]) => Method::PresetDelete(PresetNameParams {
            name: (*name).into(),
        }),
        ("rename", [name, new_name]) => Method::PresetRename(PresetRenameParams {
            name: (*name).into(),
            new_name: (*new_name).into(),
        }),
        ("save", [name, tab_id]) if !current => Method::PresetSave(PresetSaveParams {
            name: (*name).into(),
            tab_id: Some(super::normalize_tab_id(tab_id)),
            pane_id: None,
            overwrite: flags.contains(&"--overwrite"),
        }),
        ("save", [name]) => Method::PresetSave(PresetSaveParams {
            name: (*name).into(),
            tab_id: None,
            pane_id: if current { Some(caller_pane()?) } else { None },
            overwrite: flags.contains(&"--overwrite"),
        }),
        ("load", [name]) => {
            if current && workspace.is_some() {
                return Err(format!("use either --workspace or --current\n{USAGE}"));
            }
            Method::PresetLoad(PresetLoadParams {
                name: (*name).into(),
                workspace_id: workspace,
                pane_id: if current { Some(caller_pane()?) } else { None },
                focus: !flags.contains(&"--no-focus"),
            })
        }
        ("list" | "show" | "delete" | "rename" | "save" | "load", _) => return Err(USAGE.into()),
        (other, _) => return Err(format!("unknown preset command: {other}\n{USAGE}")),
    };
    Ok(method)
}

/// Opens the preset's file in the user's editor, then asks the server to read
/// it again so a mistake shows at once. The file is found through
/// `preset.list`, which lists broken presets too.
fn edit(name: &str) -> std::io::Result<i32> {
    let response = crate::cli::send_request(&Request {
        id: "cli:preset:edit".into(),
        method: Method::PresetList(EmptyParams {}),
    })?;
    if response.get("error").is_some() {
        return crate::cli::print_response(&response);
    }
    let listed: SuccessResponse =
        serde_json::from_value(response).map_err(std::io::Error::other)?;
    let ResponseResult::PresetList { presets } = listed.result else {
        eprintln!("unexpected response to preset.list");
        return Ok(1);
    };
    let Some(preset) = presets.into_iter().find(|preset| preset.name == name) else {
        eprintln!("preset {name} not found; save a tab as `{name}` first");
        return Ok(1);
    };

    let path = Path::new(&preset.path);
    let status = editor_command(path).status()?;
    if !status.success() {
        eprintln!(
            "the editor exited with {status}; {} is unchanged or partly saved",
            path.display()
        );
        return Ok(1);
    }

    let checked = crate::cli::send_request(&Request {
        id: "cli:preset:edit".into(),
        method: Method::PresetGet(PresetNameParams { name: name.into() }),
    })?;
    if let Some(error) = checked.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("invalid preset");
        eprintln!("{message}\nrun `herdsman preset edit {name}` again to fix it");
        return Ok(1);
    }
    eprintln!("preset {name} is valid");
    Ok(0)
}

/// `$VISUAL`, then `$EDITOR`, split on whitespace so values such as
/// `code --wait` work; `vi`, or Notepad on Windows, when neither is set.
fn editor_command(path: &Path) -> std::process::Command {
    let editor = editor_words(
        std::env::var("VISUAL").ok().as_deref(),
        std::env::var("EDITOR").ok().as_deref(),
    );
    let mut command = std::process::Command::new(&editor[0]);
    command.args(&editor[1..]).arg(path);
    command
}

/// The editor command as words: the first non-empty of `visual` and `editor`
/// split on whitespace, else `vi`, or Notepad on Windows. The server's
/// `preset.edit` uses it too, with the server's environment.
pub(crate) fn editor_words(visual: Option<&str>, editor: Option<&str>) -> Vec<String> {
    let fallback = if cfg!(windows) { "notepad" } else { "vi" };
    let words: Vec<String> = [visual, editor]
        .into_iter()
        .flatten()
        .map(|value| {
            value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .find(|words| !words.is_empty())
        .unwrap_or_default();
    if words.is_empty() {
        vec![fallback.to_string()]
    } else {
        words
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(
        subcommand: &str,
        values: &[&str],
        env_pane_id: Option<&str>,
    ) -> Result<Method, String> {
        let args: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
        parse(subcommand, &args, env_pane_id)
    }

    #[test]
    fn list_show_rename_and_delete() {
        assert!(matches!(
            parse_args("list", &[], None),
            Ok(Method::PresetList(_))
        ));
        assert!(matches!(
            parse_args("show", &["dev"], None),
            Ok(Method::PresetGet(PresetNameParams { name })) if name == "dev"
        ));
        assert!(matches!(
            parse_args("rename", &["dev", "api"], None),
            Ok(Method::PresetRename(PresetRenameParams { name, new_name }))
                if name == "dev" && new_name == "api"
        ));
        assert!(matches!(
            parse_args("delete", &["dev"], None),
            Ok(Method::PresetDelete(PresetNameParams { name })) if name == "dev"
        ));
        assert!(parse_args("list", &["extra"], None).is_err());
        assert!(parse_args("rename", &["dev"], None).is_err());
    }

    #[test]
    fn save_targets_a_tab_the_calling_pane_or_the_active_tab() {
        let Ok(Method::PresetSave(params)) = parse_args("save", &["dev", "w1:t2"], None) else {
            panic!("expected save");
        };
        assert_eq!(
            (params.tab_id.as_deref(), params.pane_id),
            (Some("w1:t2"), None)
        );

        let Ok(Method::PresetSave(params)) =
            parse_args("save", &["dev", "--current", "--overwrite"], Some("w1:p3"))
        else {
            panic!("expected save");
        };
        assert_eq!(
            (params.tab_id, params.pane_id.as_deref()),
            (None, Some("w1:p3"))
        );
        assert!(params.overwrite);

        let Ok(Method::PresetSave(params)) = parse_args("save", &["dev"], Some("w1:p3")) else {
            panic!("expected save");
        };
        assert_eq!((params.tab_id, params.pane_id), (None, None));

        assert!(parse_args("save", &["dev", "--current"], None).is_err());
        assert!(parse_args("save", &["dev", "w1:t2", "--current"], Some("w1:p3")).is_err());
    }

    #[test]
    fn load_targets_a_workspace_the_calling_pane_or_the_active_workspace() {
        let Ok(Method::PresetLoad(params)) =
            parse_args("load", &["dev", "--workspace", "w2", "--no-focus"], None)
        else {
            panic!("expected load");
        };
        assert_eq!(params.workspace_id.as_deref(), Some("w2"));
        assert!(!params.focus);

        let Ok(Method::PresetLoad(params)) =
            parse_args("load", &["dev", "--current"], Some("w1:p3"))
        else {
            panic!("expected load");
        };
        assert_eq!(
            (params.workspace_id, params.pane_id.as_deref()),
            (None, Some("w1:p3"))
        );
        assert!(params.focus);

        assert!(parse_args(
            "load",
            &["dev", "--workspace", "w2", "--current"],
            Some("w1:p3")
        )
        .is_err());
        assert!(parse_args("load", &["dev", "--workspace"], None).is_err());
    }

    #[test]
    fn options_belong_to_their_subcommands() {
        assert!(parse_args("show", &["dev", "--overwrite"], None).is_err());
        assert!(parse_args("save", &["dev", "--no-focus"], None).is_err());
        assert!(parse_args("load", &["dev", "--overwrite"], None).is_err());
        assert!(parse_args("nope", &[], None)
            .unwrap_err()
            .contains("unknown preset command"));
    }

    #[test]
    fn the_editor_comes_from_visual_then_editor() {
        assert_eq!(
            editor_words(Some("code --wait"), Some("vim")),
            ["code", "--wait"]
        );
        assert_eq!(editor_words(None, Some("nvim")), ["nvim"]);
        assert_eq!(editor_words(Some("  "), Some("nvim")), ["nvim"]);
        let fallback = if cfg!(windows) { "notepad" } else { "vi" };
        assert_eq!(editor_words(None, None), [fallback]);
    }
}
