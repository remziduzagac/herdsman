//! Installs from herdsman's apt, dnf and pacman packages, which live in
//! /usr/bin and update through the system package manager.
//!
//! herdsman's own module. `update.rs` reaches it through hooks marked
//! `// fork: system packages`.

use super::*;

pub(super) fn is_system_package_install() -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return false;
    };

    is_system_package_exe_path_following_links(&current_exe)
}

pub(super) fn is_system_package_exe_path_following_links(path: &Path) -> bool {
    if is_system_package_exe_path(path) {
        return true;
    }

    path.canonicalize()
        .is_ok_and(|path| is_system_package_exe_path(&path))
}

/// The apt, dnf and pacman packages install to /usr/bin; Herdsman's own
/// installer never does.
fn is_system_package_exe_path(path: &Path) -> bool {
    path.starts_with("/usr/bin") || path.starts_with("/usr/sbin")
}

/// Why `self_update` must not replace a system-package install, if this is one.
pub(super) fn self_update_refusal(channel: UpdateChannel) -> Option<String> {
    if !is_system_package_install() {
        return None;
    }
    if channel == UpdateChannel::Preview {
        return Some(
            "self-update is disabled for system-package installs; preview is only available for direct Herdsman installs".into(),
        );
    }
    Some(
        "self-update is disabled for system-package installs; update with apt, dnf or pacman"
            .into(),
    )
}

// Unix only, like the tests in `update.rs`: these paths mean nothing on Windows.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn system_package_path_is_detected() {
        let path = Path::new("/usr/bin/herdsman");

        assert!(is_system_package_exe_path(path));
        assert!(is_package_manager_managed_exe_path(path));
        assert!(preview_channel_rejection_for_exe_path(path)
            .is_some_and(|message| message.contains("system-package")));
    }

    #[test]
    fn user_install_paths_are_not_system_packages() {
        for path in ["/usr/local/bin/herdsman", "/home/user/.local/bin/herdsman"] {
            assert!(!is_system_package_exe_path(Path::new(path)), "{path}");
        }
    }
}
