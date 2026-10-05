//! `herdsman pane stack`, `pane stacks`, `pane focus-stacked`, `pane focus <id>`
//! and `pane move --stack`.
//!
//! herdsman's own module. `pane.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;
use crate::api::schema::PaneStackParams;

/// `pane focus <id>`: a lone pane id focuses that pane, revealing it when it
/// is a hidden stack member. None for every other form of `pane focus`.
pub(super) fn focus_lone_pane_id(args: &[String]) -> Option<std::io::Result<i32>> {
    match args {
        [pane_id] if !pane_id.starts_with('-') => Some(crate::cli::runtime::stack::pane_focus_id(
            crate::cli::normalize_pane_id(pane_id),
        )),
        _ => None,
    }
}

pub(super) fn pane_stack(args: &[String]) -> std::io::Result<i32> {
    let env_pane_id = super::super::target::caller_pane_id();
    let params = match parse_pane_stack_args(args, env_pane_id.as_deref()) {
        Ok(params) => params,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };

    super::super::runtime::stack::pane_stack(params)
}

const PANE_MOVE_TO_STACK_USAGE: &str =
    "usage: herdsman pane move <pane_id> --stack <target_pane_id> [--focus|--no-focus]";

/// `pane move <pane_id> --stack <target>`: join the target's stack.
pub(super) fn parse_pane_move_to_stack_args(args: &[String]) -> Result<PaneMoveParams, String> {
    let Some(pane_id) = args.first().filter(|arg| !arg.starts_with('-')) else {
        return Err(PANE_MOVE_TO_STACK_USAGE.into());
    };
    let mut target = None;
    let mut focus = true;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--stack" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --stack".into());
                };
                target = Some(super::super::normalize_pane_id(value));
                index += 2;
            }
            "--focus" => {
                focus = true;
                index += 1;
            }
            "--no-focus" => {
                focus = false;
                index += 1;
            }
            other => return Err(format!("{other} does not combine with --stack")),
        }
    }
    let Some(target_pane_id) = target else {
        return Err(PANE_MOVE_TO_STACK_USAGE.into());
    };
    Ok(PaneMoveParams {
        pane_id: super::super::normalize_pane_id(pane_id),
        destination: PaneMoveDestination::Stack { target_pane_id },
        focus,
    })
}

const PANE_FOCUS_STACKED_USAGE: &str =
    "usage: herdsman pane focus-stacked [<pane_id>|--pane ID|--current] <N|next|previous>";

pub(super) fn pane_focus_stacked(args: &[String]) -> std::io::Result<i32> {
    let env_pane_id = super::super::target::caller_pane_id();
    match parse_pane_focus_stacked_args(args, env_pane_id.as_deref()) {
        Ok(params) => super::super::runtime::stack::pane_focus_stacked(params),
        Err(message) => {
            eprintln!("{message}");
            Ok(2)
        }
    }
}

/// Members count from 1, as the strip numbers them; the API counts from 0.
fn parse_pane_focus_stacked_args(
    args: &[String],
    env_pane_id: Option<&str>,
) -> Result<crate::api::schema::PaneFocusStackedParams, String> {
    let mut pane_id = None;
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--pane" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --pane".into());
                };
                pane_id = Some(super::super::normalize_pane_id(value));
                index += 2;
            }
            "--current" => {
                pane_id = Some(
                    env_pane_id
                        .map(super::super::normalize_pane_id)
                        .ok_or("--current requires HERDSMAN_PANE_ID")?,
                );
                index += 1;
            }
            other if other.starts_with("--") => return Err(format!("unknown option: {other}")),
            other => {
                positionals.push(other);
                index += 1;
            }
        }
    }
    let member = match positionals.as_slice() {
        [member] => member,
        [pane, member] if pane_id.is_none() => {
            pane_id = Some(super::super::normalize_pane_id(pane));
            member
        }
        _ => return Err(PANE_FOCUS_STACKED_USAGE.into()),
    };
    let (index, step) = match *member {
        "next" => (None, Some(1)),
        "previous" | "prev" => (None, Some(-1)),
        number => match number.parse::<usize>() {
            Ok(number) if number > 0 => (Some(number - 1), None),
            _ => {
                return Err(format!(
                    "expected a member number, next or previous: {number}"
                ))
            }
        },
    };
    Ok(crate::api::schema::PaneFocusStackedParams {
        pane_id,
        index,
        step,
    })
}

pub(super) fn pane_stacks(args: &[String]) -> std::io::Result<i32> {
    let workspace_id = match args {
        [] => None,
        [flag, value] if flag == "--workspace" => Some(super::super::normalize_workspace_id(value)),
        _ => {
            eprintln!("usage: herdsman pane stacks [--workspace ID]");
            return Ok(2);
        }
    };
    super::super::runtime::stack::pane_stacks(crate::api::schema::PaneStacksParams { workspace_id })
}

/// Stack takes split's options minus the geometry ones.
fn parse_pane_stack_args(
    args: &[String],
    env_pane_id: Option<&str>,
) -> Result<PaneStackParams, String> {
    if let Some(geometry) = args
        .iter()
        .find(|arg| matches!(arg.as_str(), "--direction" | "--ratio"))
    {
        return Err(format!("unknown option: {geometry}"));
    }
    let mut split_args = args.to_vec();
    split_args.extend(["--direction".into(), "right".into()]);
    let split = parse_pane_split_args(&split_args, env_pane_id)?;
    Ok(PaneStackParams {
        workspace_id: split.workspace_id,
        target_pane_id: split.target_pane_id,
        cwd: split.cwd,
        focus: split.focus,
        right_click: split.right_click,
        env: split.env,
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::args;
    use super::*;

    #[test]
    fn parse_pane_move_args_routes_stack_destinations() {
        let params = parse_pane_move_args(&args(&[
            "issue-1:p3",
            "--stack",
            "issue-1:p1",
            "--no-focus",
        ]))
        .unwrap();

        assert_eq!(params.pane_id, "issue-1:p3");
        assert_eq!(
            params.destination,
            PaneMoveDestination::Stack {
                target_pane_id: "issue-1:p1".into()
            }
        );
        assert!(!params.focus);
        assert!(
            parse_pane_move_args(&args(&["issue-1:p3", "--stack", "p1", "--tab", "t1"]))
                .unwrap_err()
                .contains("does not combine with --stack")
        );
        assert!(parse_pane_move_args(&args(&["--stack", "issue-1:p1"])).is_err());
    }

    #[test]
    fn parse_pane_focus_stacked_args_counts_members_from_one() {
        let parse = |values: &[&str], env| parse_pane_focus_stacked_args(&args(values), env);

        let params = parse(&["2"], None).unwrap();
        assert_eq!(
            (params.pane_id, params.index, params.step),
            (None, Some(1), None)
        );
        let params = parse(&["issue-1:p3", "next"], None).unwrap();
        assert_eq!(
            (params.pane_id.as_deref(), params.index, params.step),
            (Some("issue-1:p3"), None, Some(1))
        );
        let params = parse(&["--current", "previous"], Some("issue-1:p4")).unwrap();
        assert_eq!(
            (params.pane_id.as_deref(), params.step),
            (Some("issue-1:p4"), Some(-1))
        );
        assert!(parse(&["0"], None).is_err());
        assert!(parse(&[], None).is_err());
        assert!(parse(&["--bogus", "1"], None).is_err());
    }

    #[test]
    fn parse_pane_stack_args_takes_split_options_without_geometry() {
        let params = parse_pane_stack_args(
            &args(&[
                "--current",
                "--cwd",
                "/repo",
                "--env",
                "ROLE=agent",
                "--focus",
            ]),
            Some("issue-1:p1"),
        )
        .unwrap();

        assert_eq!(params.target_pane_id, Some("issue-1:p1".into()));
        assert_eq!(params.cwd.as_deref(), Some("/repo"));
        assert_eq!(params.env.get("ROLE").map(String::as_str), Some("agent"));
        assert!(params.focus);
        assert_eq!(
            parse_pane_stack_args(&args(&["issue-1", "--no-focus"]), None)
                .unwrap()
                .target_pane_id,
            Some("issue-1".into())
        );
        for geometry in ["--direction", "--ratio"] {
            assert_eq!(
                parse_pane_stack_args(&args(&[geometry, "right"]), None).unwrap_err(),
                format!("unknown option: {geometry}")
            );
        }
    }
}
