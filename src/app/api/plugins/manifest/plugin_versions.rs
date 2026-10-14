//! A plugin's minimum versions. A manifest names the herdr release it needs
//! (`min_herdr_version`), the herdsman release it needs (`min_herdsman_version`),
//! or both, and herdsman refuses the plugin when either is newer than the herdr
//! release it is built on or than its own version. Plugins written for herdr give
//! only `min_herdr_version`.
//!
//! herdsman's own module, kept out of `upstream/rename` (KEEP) because it names
//! herdr. `manifest.rs` reaches it through hooks marked `// fork: plugin versions`.
//! Errors reuse the codes of herdr's `min_herdsman_version` check, so clients
//! and herdr's tests see the same codes whichever field is wrong.

use super::{non_empty_trimmed, validate_min_herdsman_version};
use crate::build_info::base::HERDR_VERSION;
use crate::update::Version;

const INVALID: &str = "invalid_plugin_min_herdsman_version";
const TOO_NEW: &str = "plugin_requires_newer_herdsman";

/// Check both minimum versions; at least one is required. Returns the herdsman
/// minimum, empty when the manifest gives only a herdr one, and the herdr minimum
/// when there is one.
pub(super) fn validate(
    min_herdsman_version: Option<&str>,
    min_herdr_version: Option<&str>,
) -> Result<(String, Option<String>), (&'static str, String)> {
    let min_herdr_version = min_herdr_version
        .map(validate_min_herdr_version)
        .transpose()?;
    let min_herdsman_version = match (min_herdsman_version, &min_herdr_version) {
        (None, Some(_)) => String::new(),
        (None, None) => {
            return Err((
                INVALID,
                "plugin min_herdr_version or min_herdsman_version is required".to_string(),
            ))
        }
        (value, _) => validate_min_herdsman_version(value)?,
    };
    Ok((min_herdsman_version, min_herdr_version))
}

fn validate_min_herdr_version(value: &str) -> Result<String, (&'static str, String)> {
    let value = non_empty_trimmed(value, INVALID, "plugin min_herdr_version is empty")?;
    let required = Version::parse(&value).ok_or_else(|| {
        (
            INVALID,
            format!("plugin min_herdr_version must be a semantic version like {HERDR_VERSION}"),
        )
    })?;
    let base = Version::parse(HERDR_VERSION).expect("HERDR_VERSION is X.Y.Z");
    if required > base {
        return Err((
            TOO_NEW,
            format!(
                "plugin requires herdr {required} or newer; herdsman {} is built on herdr {base}",
                crate::build_info::BASE_VERSION
            ),
        ));
    }
    Ok(required.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn next_minor(version: &str) -> String {
        let v = Version::parse(version).unwrap();
        format!("{}.{}.0", v.major, v.minor + 1)
    }

    #[test]
    fn herdr_minimum_is_checked_against_the_herdr_release() {
        let (herdsman, herdr) = validate(None, Some(HERDR_VERSION)).unwrap();
        assert_eq!(herdsman, "");
        assert_eq!(herdr.as_deref(), Some(HERDR_VERSION));

        let (code, message) = validate(None, Some(&next_minor(HERDR_VERSION))).unwrap_err();
        assert_eq!(code, TOO_NEW);
        assert!(message.contains(HERDR_VERSION), "{message}");
    }

    #[test]
    fn herdsman_minimum_is_checked_against_herdsman_itself() {
        let current = crate::build_info::BASE_VERSION;
        let (herdsman, herdr) = validate(Some(current), None).unwrap();
        assert_eq!(herdsman, current);
        assert_eq!(herdr, None);

        let (code, _) = validate(Some(&next_minor(current)), None).unwrap_err();
        assert_eq!(code, TOO_NEW);
    }

    #[test]
    fn both_minimums_must_hold() {
        let current = crate::build_info::BASE_VERSION;
        assert!(validate(Some(current), Some(HERDR_VERSION)).is_ok());
        assert_eq!(
            validate(Some(&next_minor(current)), Some(HERDR_VERSION))
                .unwrap_err()
                .0,
            TOO_NEW
        );
        assert_eq!(
            validate(Some(current), Some(&next_minor(HERDR_VERSION)))
                .unwrap_err()
                .0,
            TOO_NEW
        );
    }

    #[test]
    fn a_minimum_is_required_and_must_be_a_version() {
        let (code, message) = validate(None, None).unwrap_err();
        assert_eq!(code, INVALID);
        assert!(
            message.contains("min_herdr_version or min_herdsman_version"),
            "{message}"
        );
        assert_eq!(validate(None, Some("soon")).unwrap_err().0, INVALID);
        assert_eq!(validate(None, Some(" ")).unwrap_err().0, INVALID);
    }
}
