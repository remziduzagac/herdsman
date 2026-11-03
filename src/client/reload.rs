//! Coming back after a live handoff. When the server moves to a new binary it
//! disconnects every client; a local client then waits for the new server and
//! starts itself again from the binary on disk, with the same command line, so
//! the user is back in the same session on the new version.
//!
//! herdsman's own module. `client/mod.rs` reaches it through a hook marked
//! `// fork: update-handoff`.

use std::io::Write as _;
use std::time::{Duration, Instant};

use super::errors::ClientError;

/// The reason a server gives its clients when it hands off.
const HANDOFF_REASON: &str = "live update in progress";
const WAIT_FOR_SERVER: Duration = Duration::from_secs(30);

fn is_handoff(err: &ClientError) -> bool {
    matches!(
        err,
        ClientError::ServerShutdown { reason: Some(reason) } if reason.starts_with(HANDOFF_REASON)
    )
}

/// After the terminal is restored: when `err` is a handoff, wait for the new
/// server and replace this process with the binary on disk. Returns only when
/// that is not possible, so the caller reports `err` as before.
pub(super) fn after_handoff(err: &ClientError) {
    if !is_handoff(err) {
        return;
    }
    #[cfg(unix)]
    {
        let _ = writeln!(
            std::io::stderr(),
            "herdsman: server updated, reconnecting..."
        );
        if let Err(reason) = restart_on_new_binary() {
            tracing::warn!("cannot reconnect after handoff: {reason}");
            let _ = writeln!(std::io::stderr(), "herdsman: {reason}; run herdsman again");
        }
    }
}

#[cfg(unix)]
fn restart_on_new_binary() -> Result<(), String> {
    use std::os::unix::process::CommandExt as _;

    let exe = crate::update::handoff::running_exe().ok_or("cannot find the herdsman binary")?;
    let binary = crate::update::handoff::describe_binary(&exe)
        .ok_or("cannot run the new herdsman binary")?;
    wait_for_server(&binary.version)?;
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_else(|| exe.clone().into_os_string());
    let err = std::process::Command::new(&exe)
        .arg0(program)
        .args(args)
        .exec();
    Err(format!("cannot start {}: {err}", exe.display()))
}

/// Wait until the session's server answers as `version`, the binary it
/// handed off to.
#[cfg(unix)]
fn wait_for_server(version: &str) -> Result<(), String> {
    let socket = crate::api::socket_path();
    let deadline = Instant::now() + WAIT_FOR_SERVER;
    while Instant::now() < deadline {
        if let Ok(Some(status)) =
            crate::api::read_runtime_status_at(&socket, Duration::from_millis(500))
        {
            if status.version.as_deref() == Some(version) {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!("the server did not come back as {version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_handoff_shutdown_brings_the_client_back() {
        let handoff = ClientError::ServerShutdown {
            reason: Some("live update in progress; reconnect after handoff completes".into()),
        };
        assert!(is_handoff(&handoff));
        for other in [
            ClientError::ServerShutdown {
                reason: Some("detached".into()),
            },
            ClientError::ServerShutdown { reason: None },
        ] {
            assert!(!is_handoff(&other));
        }
    }
}
