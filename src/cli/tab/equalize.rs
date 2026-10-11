//! `herdsman tab equalize`: equal widths or equal heights for a tab's panes.
//!
//! herdsman's own module. `tab.rs` reaches it through hooks marked
//! `// fork: equalize`.

use crate::api::schema::{LayoutDimension, LayoutEqualizeParams, Method, Request};

pub(super) const USAGE: &str = "usage: herdsman tab equalize width|height [<tab_id>|--current]";

pub(super) fn tab_equalize(args: &[String]) -> std::io::Result<i32> {
    let env_pane_id = super::super::target::caller_pane_id();
    let params = match parse_tab_equalize_args(args, env_pane_id.as_deref()) {
        Ok(params) => params,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };
    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:tab:equalize".into(),
        method: Method::LayoutEqualize(params),
    })?)
}

/// Without a tab id or `--current`, the server's active tab.
fn parse_tab_equalize_args(
    args: &[String],
    env_pane_id: Option<&str>,
) -> Result<LayoutEqualizeParams, String> {
    let mut current = false;
    let mut positionals = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--current" => current = true,
            other if other.starts_with("--") => {
                return Err(format!("unknown option: {other}\n{USAGE}"))
            }
            other => positionals.push(other),
        }
    }
    let (dimension, tab_id) = match positionals.as_slice() {
        [dimension] => (*dimension, None),
        [dimension, tab_id] if !current => {
            (*dimension, Some(super::super::normalize_tab_id(tab_id)))
        }
        _ => return Err(USAGE.into()),
    };
    let dimension = match dimension {
        "width" => LayoutDimension::Width,
        "height" => LayoutDimension::Height,
        other => return Err(format!("unknown dimension: {other}\n{USAGE}")),
    };
    let pane_id = if current {
        Some(
            env_pane_id
                .map(super::super::normalize_pane_id)
                .ok_or("--current requires HERDSMAN_PANE_ID")?,
        )
    } else {
        None
    };
    Ok(LayoutEqualizeParams {
        tab_id,
        pane_id,
        dimension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(values: &[&str], env_pane_id: Option<&str>) -> Result<LayoutEqualizeParams, String> {
        let args: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
        parse_tab_equalize_args(&args, env_pane_id)
    }

    #[test]
    fn equalize_takes_a_dimension_and_an_optional_tab() {
        let params = parse(&["width"], None).unwrap();
        assert_eq!(params.dimension, LayoutDimension::Width);
        assert_eq!((params.tab_id, params.pane_id), (None, None));

        let params = parse(&["height", "w1:t2"], None).unwrap();
        assert_eq!(params.dimension, LayoutDimension::Height);
        assert_eq!(params.tab_id.as_deref(), Some("w1:t2"));
    }

    #[test]
    fn current_targets_the_callers_tab_through_its_pane() {
        let params = parse(&["--current", "width"], Some("w1:p3")).unwrap();
        assert_eq!(params.pane_id.as_deref(), Some("w1:p3"));
        assert_eq!(params.tab_id, None);

        assert!(parse(&["width", "--current"], None).is_err());
    }

    #[test]
    fn equalize_rejects_bad_arguments() {
        for bad in [
            &[][..],
            &["wide"][..],
            &["width", "w1:t1", "w1:t2"][..],
            &["width", "w1:t1", "--current"][..],
            &["width", "--tab", "w1:t1"][..],
        ] {
            assert!(parse(bad, Some("w1:p1")).is_err(), "{bad:?}");
        }
    }
}
