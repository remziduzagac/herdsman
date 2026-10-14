//! The herdr release herdsman is built on.
//!
//! herdsman's own module, kept out of `upstream/rename` (KEEP) because it names
//! herdr. `build_info.rs` reaches it through a hook marked
//! `// fork: plugin versions`. Each herdr merge updates [`HERDR_VERSION`], and a
//! test checks it against the newest "Based on herdr" line in `CHANGELOG.md`.

/// The herdr release this herdsman is built on.
pub const HERDR_VERSION: &str = "0.9.4";

/// The line `herdsman --version` prints below the version. The version line
/// itself stays `herdsman X.Y.Z`, which install checks match exactly.
pub fn based_on() -> String {
    format!("based on herdr {HERDR_VERSION}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn herdr_version_matches_the_changelog() {
        let changelog =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/CHANGELOG.md"))
                .expect("read CHANGELOG.md");
        let named = changelog
            .lines()
            .find_map(|line| line.strip_prefix("Based on herdr "))
            .and_then(|rest| rest.split_whitespace().next())
            .map(|version| version.trim_end_matches(['.', ',']))
            .expect("CHANGELOG.md has a \"Based on herdr\" line");
        assert_eq!(
            super::HERDR_VERSION,
            named,
            "HERDR_VERSION must be the herdr release CHANGELOG.md's newest entry is based on"
        );
        assert!(crate::update::Version::parse(super::HERDR_VERSION).is_some());
    }

    #[test]
    fn version_detail_names_the_herdr_release() {
        assert_eq!(
            super::based_on(),
            format!("based on herdr {}", super::HERDR_VERSION)
        );
    }
}
