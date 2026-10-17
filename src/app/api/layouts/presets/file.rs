//! A preset's TOML file: one table per node, `[root]`, `[root.first]`,
//! `[root.second]` and so on, read into a [`PresetNode`] tree.
//!
//! ```toml
//! label = "dev"
//!
//! [root]
//! split = "right"
//! ratio = 0.6
//!
//! [root.first]
//! label = "agent"
//! command = "claude"
//!
//! [root.second]
//! label = "docs"
//! cwd = "docs"
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::api::schema::{PresetNode, PresetPane, SplitDirection};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PresetFile {
    pub label: Option<String>,
    pub root: PresetNode,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileToml {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    root: NodeToml,
}

/// Every field a node may carry; which ones it has decides its kind. Plain
/// values come before tables, as TOML writes them.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeToml {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    split: Option<SplitDirection>,
    /// `f64`, so a written `0.6` stays `0.6` rather than `f32`'s widened digits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stack: Option<Vec<PaneToml>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    first: Option<Box<NodeToml>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    second: Option<Box<NodeToml>>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaneToml {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
}

/// Deeper trees are refused here, as `layout.apply` would refuse them.
use super::super::MAX_LAYOUT_DEPTH as MAX_DEPTH;

pub(super) fn parse(text: &str) -> Result<PresetFile, String> {
    let file: FileToml = toml::from_str(text).map_err(|err| toml_error(text, &err))?;
    Ok(PresetFile {
        label: file.label,
        root: node_from_toml(file.root, "root", 0)?,
    })
}

/// A TOML error on one line, `line L, column C: message`, which fits a list
/// row and a CLI message, rather than TOML's own drawing of the line.
fn toml_error(text: &str, err: &toml::de::Error) -> String {
    let message = err.message().trim();
    let Some(span) = err.span() else {
        return message.to_string();
    };
    let before = &text[..span.start.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.len() - before.rfind('\n').map_or(0, |newline| newline + 1) + 1;
    format!("line {line}, column {column}: {message}")
}

pub(super) fn render(preset: &PresetFile) -> Result<String, String> {
    let file = FileToml {
        label: preset.label.clone(),
        root: node_to_toml(&preset.root),
    };
    toml::to_string(&file).map_err(|err| err.to_string())
}

fn node_from_toml(node: NodeToml, path: &str, depth: usize) -> Result<PresetNode, String> {
    if depth > MAX_DEPTH {
        return Err(format!("{path}: the layout is nested too deeply"));
    }
    let has_pane_fields = node.label.is_some()
        || node.cwd.is_some()
        || node.command.is_some()
        || !node.env.is_empty();

    if let Some(direction) = node.split {
        if has_pane_fields || node.stack.is_some() || node.active.is_some() {
            return Err(format!(
                "{path}: a split holds only `split`, `ratio`, `first` and `second`"
            ));
        }
        let (Some(first), Some(second)) = (node.first, node.second) else {
            return Err(format!("{path}: a split needs both `first` and `second`"));
        };
        let ratio = node.ratio.unwrap_or(0.5);
        if !(ratio > 0.0 && ratio < 1.0) {
            return Err(format!("{path}: `ratio` must be between 0 and 1"));
        }
        return Ok(PresetNode::Split {
            direction,
            ratio: ratio as f32,
            first: Box::new(node_from_toml(*first, &format!("{path}.first"), depth + 1)?),
            second: Box::new(node_from_toml(
                *second,
                &format!("{path}.second"),
                depth + 1,
            )?),
        });
    }
    if node.first.is_some() || node.second.is_some() || node.ratio.is_some() {
        return Err(format!(
            "{path}: `first`, `second` and `ratio` belong to a split; add `split = \"right\"` \
             or `split = \"down\"`"
        ));
    }

    if let Some(stack) = node.stack {
        if has_pane_fields {
            return Err(format!(
                "{path}: a stack's panes go in `[[{path}.stack]]` tables"
            ));
        }
        if stack.is_empty() {
            return Err(format!("{path}: a stack needs at least one pane"));
        }
        let active = node.active.unwrap_or(0);
        if active >= stack.len() {
            return Err(format!(
                "{path}: `active` is {active}, but the stack has {} panes",
                stack.len()
            ));
        }
        return Ok(PresetNode::Stack {
            panes: stack.into_iter().map(pane_from_toml).collect(),
            active,
        });
    }
    if node.active.is_some() {
        return Err(format!("{path}: `active` belongs to a stack"));
    }

    Ok(PresetNode::Pane {
        pane: pane_from_toml(PaneToml {
            label: node.label,
            cwd: node.cwd,
            command: node.command,
            env: node.env,
        }),
    })
}

fn pane_from_toml(pane: PaneToml) -> PresetPane {
    PresetPane {
        label: pane.label,
        cwd: pane.cwd,
        command: pane.command,
        env: pane.env,
    }
}

/// A ratio to four decimals, which is finer than a terminal cell and reads
/// well in the file.
fn rounded_ratio(ratio: f32) -> f64 {
    (f64::from(ratio) * 10_000.0).round() / 10_000.0
}

fn node_to_toml(node: &PresetNode) -> NodeToml {
    match node {
        PresetNode::Pane { pane } => NodeToml {
            label: pane.label.clone(),
            cwd: pane.cwd.clone(),
            command: pane.command.clone(),
            env: pane.env.clone(),
            ..NodeToml::default()
        },
        PresetNode::Split {
            direction,
            ratio,
            first,
            second,
        } => NodeToml {
            split: Some(direction.clone()),
            ratio: Some(rounded_ratio(*ratio)),
            first: Some(Box::new(node_to_toml(first))),
            second: Some(Box::new(node_to_toml(second))),
            ..NodeToml::default()
        },
        PresetNode::Stack { panes, active } => NodeToml {
            active: (*active != 0).then_some(*active),
            stack: Some(
                panes
                    .iter()
                    .map(|pane| PaneToml {
                        label: pane.label.clone(),
                        cwd: pane.cwd.clone(),
                        command: pane.command.clone(),
                        env: pane.env.clone(),
                    })
                    .collect(),
            ),
            ..NodeToml::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(label: &str, command: Option<&str>) -> PresetNode {
        PresetNode::Pane {
            pane: PresetPane {
                label: Some(label.into()),
                command: command.map(str::to_string),
                ..PresetPane::default()
            },
        }
    }

    #[test]
    fn reads_a_split_with_its_panes() {
        let preset = parse(
            r#"
label = "dev"

[root]
split = "right"
ratio = 0.6

[root.first]
label = "agent"
command = "claude"

[root.second]
label = "docs"
cwd = "docs"
"#,
        )
        .unwrap();

        assert_eq!(preset.label.as_deref(), Some("dev"));
        let PresetNode::Split {
            direction,
            ratio,
            first,
            second,
        } = preset.root
        else {
            panic!("root should be a split");
        };
        assert_eq!(direction, SplitDirection::Right);
        assert_eq!(ratio, 0.6);
        assert_eq!(*first, pane("agent", Some("claude")));
        let PresetNode::Pane { pane } = *second else {
            panic!("second should be a pane");
        };
        assert_eq!(pane.cwd.as_deref(), Some("docs"));
    }

    #[test]
    fn a_split_without_a_ratio_is_halved() {
        let preset = parse(
            "[root]\nsplit = \"down\"\n[root.first]\n[root.second]\ncommand = \"just test\"\n",
        )
        .unwrap();
        assert!(matches!(preset.root, PresetNode::Split { ratio, .. } if ratio == 0.5));
    }

    #[test]
    fn reads_a_stack() {
        let preset = parse(
            "[root]\nactive = 1\n[[root.stack]]\nlabel = \"a\"\n[[root.stack]]\nlabel = \"b\"\n",
        )
        .unwrap();
        let PresetNode::Stack { panes, active } = preset.root else {
            panic!("root should be a stack");
        };
        assert_eq!(active, 1);
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[1].label.as_deref(), Some("b"));
    }

    #[test]
    fn an_empty_root_is_one_shell() {
        let preset = parse("[root]\n").unwrap();
        assert_eq!(
            preset.root,
            PresetNode::Pane {
                pane: PresetPane::default()
            }
        );
    }

    #[test]
    fn errors_name_the_node() {
        let cases = [
            (
                "[root]\nsplit = \"right\"\n[root.first]\n",
                "root: a split needs both",
            ),
            (
                "[root]\nsplit = \"right\"\n[root.first]\n[root.second]\nsplit = \"down\"\n\
                 [root.second.first]\n",
                "root.second: a split needs both",
            ),
            (
                "[root]\nsplit = \"right\"\nratio = 1.5\n[root.first]\n[root.second]\n",
                "between 0 and 1",
            ),
            ("[root]\nratio = 0.5\n", "belong to a split"),
            (
                "[root]\nsplit = \"right\"\nlabel = \"x\"\n[root.first]\n[root.second]\n",
                "a split holds only",
            ),
            ("[root]\nstack = []\n", "at least one pane"),
            ("[root]\nactive = 2\n[[root.stack]]\n", "`active` is 2"),
            ("[root]\nactive = 0\n", "`active` belongs to a stack"),
        ];
        for (text, expected) in cases {
            let err = parse(text).unwrap_err();
            assert!(err.contains(expected), "{text:?}: {err}");
        }
    }

    #[test]
    fn unknown_fields_and_bad_values_are_refused_with_a_line() {
        let err = parse("[root]\ncomand = \"claude\"\n").unwrap_err();
        assert!(err.contains("comand"), "{err}");
        assert!(err.contains("line 2"), "{err}");

        let err = parse("[root]\nsplit = \"left\"\n").unwrap_err();
        assert!(err.contains("left"), "{err}");
    }

    #[test]
    fn toml_errors_fit_on_one_line() {
        let err = parse("[root]\nsplit = \"up\"\n").unwrap_err();
        assert!(!err.contains('\n'), "{err}");
        assert!(err.starts_with("line 2, column 9: "), "{err}");
        assert!(err.contains("up"), "{err}");
    }

    #[test]
    fn a_missing_root_is_refused() {
        let err = parse("label = \"dev\"\n").unwrap_err();
        assert!(err.contains("root"), "{err}");
    }

    #[test]
    fn render_writes_one_table_per_node_and_reads_back() {
        let preset = PresetFile {
            label: Some("dev".into()),
            root: PresetNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.6,
                first: Box::new(pane("agent", Some("claude"))),
                second: Box::new(PresetNode::Stack {
                    panes: vec![
                        PresetPane {
                            label: Some("tests".into()),
                            command: Some("just test".into()),
                            ..PresetPane::default()
                        },
                        PresetPane::default(),
                    ],
                    active: 1,
                }),
            },
        };

        let text = render(&preset).unwrap();
        assert!(text.starts_with("label = \"dev\"\n"), "{text}");
        assert!(
            text.contains("[root]\nsplit = \"right\"\nratio = 0.6\n"),
            "{text}"
        );
        assert!(
            text.contains("[root.first]\nlabel = \"agent\"\ncommand = \"claude\"\n"),
            "{text}"
        );
        assert!(
            text.contains("[[root.second.stack]]\nlabel = \"tests\"\n"),
            "{text}"
        );
        assert_eq!(parse(&text).unwrap(), preset);
    }

    #[test]
    fn ratios_are_written_to_four_decimals() {
        let preset = PresetFile {
            label: None,
            root: PresetNode::Split {
                direction: SplitDirection::Down,
                ratio: 1.0 / 3.0,
                first: Box::new(PresetNode::Pane {
                    pane: PresetPane::default(),
                }),
                second: Box::new(PresetNode::Pane {
                    pane: PresetPane::default(),
                }),
            },
        };
        let text = render(&preset).unwrap();
        assert!(text.contains("ratio = 0.3333\n"), "{text}");
    }

    #[test]
    fn too_deep_a_tree_is_refused() {
        let mut text = String::new();
        let mut path = String::from("root");
        for _ in 0..=MAX_DEPTH {
            text.push_str(&format!("[{path}]\nsplit = \"right\"\n[{path}.first]\n"));
            path.push_str(".second");
        }
        text.push_str(&format!("[{path}]\n"));
        let err = parse(&text).unwrap_err();
        assert!(err.contains("nested too deeply"), "{err}");
    }
}
