//! A preset's parameters: the names its file may declare, and the environment
//! variables a load gives every pane from the values it was given.
//!
//! Values reach a pane's command as environment variables that its shell
//! expands, never as text spliced into the command, so a value is only ever
//! data: a branch named `x;rm -rf ~` stays a string.

use std::collections::BTreeMap;

use crate::api::schema::PresetParam;

/// The longest parameter name.
const MAX_NAME: usize = 32;

/// Variables a parameter may not replace, because the shell and the tools in
/// the pane rely on them.
const RESERVED: &[&str] = &[
    "PATH", "HOME", "SHELL", "TERM", "USER", "LOGNAME", "PWD", "OLDPWD", "LANG", "TMPDIR", "IFS",
    "PS1",
];
const RESERVED_PREFIXES: &[&str] = &["HERDSMAN_", "LC_"];

/// Why a load's values do not fit the preset.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ParamError {
    pub code: &'static str,
    pub message: String,
}

impl ParamError {
    fn new(code: &'static str, message: String) -> Self {
        Self { code, message }
    }
}

/// The variable a parameter's value is in.
pub(super) fn env_name(name: &str) -> String {
    name.to_ascii_uppercase()
}

/// A parameter name a preset file may declare: `[a-z][a-z0-9_]*`, at most 32
/// characters, whose variable is none the shell relies on.
pub(super) fn check_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let well_formed = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && name.len() <= MAX_NAME;
    if !well_formed {
        return Err(format!(
            "parameter `{name}`: a name is a lowercase letter, then lowercase letters, digits \
             and `_`, at most {MAX_NAME} characters"
        ));
    }
    let variable = env_name(name);
    if RESERVED.contains(&variable.as_str())
        || RESERVED_PREFIXES
            .iter()
            .any(|prefix| variable.starts_with(prefix))
    {
        return Err(format!(
            "parameter `{name}`: its variable, {variable}, is one the shell or herdsman uses"
        ));
    }
    Ok(())
}

/// The variables every pane gets from one load: each declared parameter's
/// value, given or default. Fails on a name the preset does not declare, a
/// required one left out, or a value an environment cannot hold.
pub(super) fn load_env(
    declared: &[PresetParam],
    given: &BTreeMap<String, String>,
) -> Result<Vec<(String, String)>, ParamError> {
    if let Some(unknown) = given
        .keys()
        .find(|name| !declared.iter().any(|param| &param.name == *name))
    {
        let known = declared
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>();
        let takes = if known.is_empty() {
            "takes no parameters".to_string()
        } else {
            format!("takes {}", known.join(", "))
        };
        return Err(ParamError::new(
            "unknown_preset_param",
            format!("the preset has no parameter `{unknown}`; it {takes}"),
        ));
    }
    let mut env = Vec::new();
    for param in declared {
        let value = match (given.get(&param.name), &param.default) {
            (Some(value), _) => value,
            (None, Some(default)) => default,
            (None, None) if param.required => {
                return Err(ParamError::new(
                    "missing_preset_param",
                    format!("the preset needs a value for `{}`", param.name),
                ))
            }
            (None, None) => continue,
        };
        env.push((env_name(&param.name), checked(&param.name, value)?));
    }
    Ok(env)
}

fn checked(name: &str, value: &str) -> Result<String, ParamError> {
    if value.contains('\0') {
        return Err(ParamError::new(
            "invalid_preset_param",
            format!("the value of `{name}` holds a NUL character"),
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(name: &str, required: bool, default: Option<&str>) -> PresetParam {
        PresetParam {
            name: name.into(),
            required,
            default: default.map(str::to_string),
            description: None,
        }
    }

    fn given(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn names_are_lowercase_words_whose_variables_are_free() {
        for name in ["pr", "base", "ticket_id", "a1"] {
            assert_eq!(check_name(name), Ok(()), "{name}");
        }
        for name in ["", "PR", "1st", "_x", "pr-number", &"x".repeat(33)] {
            assert!(check_name(name).is_err(), "{name}");
        }
        for name in ["path", "home", "herdsman_pane_id", "lc_all"] {
            let err = check_name(name).unwrap_err();
            assert!(err.contains("the shell or herdsman uses"), "{name}: {err}");
        }
    }

    #[test]
    fn values_given_and_defaults_become_upper_case_variables() {
        let declared = [param("pr", true, None), param("base", false, Some("dev"))];
        let env = load_env(&declared, &given(&[("pr", "42")])).unwrap();
        assert_eq!(
            env,
            [
                ("PR".to_string(), "42".to_string()),
                ("BASE".into(), "dev".into()),
            ]
        );
    }

    #[test]
    fn a_value_is_kept_as_it_is() {
        let declared = [param("branch", false, None)];
        let env = load_env(&declared, &given(&[("branch", "x;rm -rf ~ $HOME")])).unwrap();
        assert_eq!(
            env,
            [("BRANCH".to_string(), "x;rm -rf ~ $HOME".to_string())]
        );
    }

    #[test]
    fn an_optional_parameter_without_a_value_sets_nothing() {
        let declared = [param("note", false, None)];
        assert_eq!(load_env(&declared, &given(&[])), Ok(Vec::new()));
    }

    #[test]
    fn unknown_missing_and_unholdable_values_are_refused() {
        let declared = [param("pr", true, None)];
        let err = load_env(&declared, &given(&[("pr", "1"), ("prr", "1")])).unwrap_err();
        assert_eq!(err.code, "unknown_preset_param");
        assert!(err.message.contains("takes pr"), "{}", err.message);

        let err = load_env(&[], &given(&[("pr", "1")])).unwrap_err();
        assert!(
            err.message.contains("takes no parameters"),
            "{}",
            err.message
        );

        let err = load_env(&declared, &given(&[])).unwrap_err();
        assert_eq!(err.code, "missing_preset_param");

        let err = load_env(&declared, &given(&[("pr", "4\u{0}2")])).unwrap_err();
        assert_eq!(err.code, "invalid_preset_param");
    }
}
