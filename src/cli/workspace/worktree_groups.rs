//! `herdsman workspace group`: put a linked worktree's workspace into a named
//! group, or take it out.
//!
//! herdsman's own module. `workspace.rs` reaches it through hooks marked
//! `// fork: worktree groups`.

use crate::api::schema::{Method, Request, WorkspaceSetWorktreeGroupParams};

const USAGE: &str = "usage: herdsman workspace group <workspace_id> <name>|--clear";

pub(super) fn workspace_group(args: &[String]) -> std::io::Result<i32> {
    let params = match parse_workspace_group_args(args) {
        Ok(params) => params,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };
    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:workspace:group".into(),
        method: Method::WorkspaceSetWorktreeGroup(params),
    })?)
}

fn parse_workspace_group_args(args: &[String]) -> Result<WorkspaceSetWorktreeGroupParams, String> {
    let [workspace_id, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    let group = match rest {
        [flag] if flag == "--clear" => None,
        [] => return Err(USAGE.into()),
        name if name.iter().any(|part| part.starts_with("--")) => {
            return Err(format!("unknown option in {name:?}\n{USAGE}"))
        }
        name => Some(name.join(" ")),
    };
    Ok(WorkspaceSetWorktreeGroupParams {
        workspace_id: crate::cli::normalize_workspace_id(workspace_id),
        group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn group_takes_a_name_of_several_words_or_clear() {
        let params = parse_workspace_group_args(&args(&["w1", "code", "review"])).unwrap();
        assert_eq!(params.group.as_deref(), Some("code review"));

        let params = parse_workspace_group_args(&args(&["w1", "--clear"])).unwrap();
        assert_eq!(params.group, None);

        for bad in [&["w1"][..], &[][..], &["w1", "--nope"][..]] {
            assert!(parse_workspace_group_args(&args(bad)).is_err(), "{bad:?}");
        }
    }
}
