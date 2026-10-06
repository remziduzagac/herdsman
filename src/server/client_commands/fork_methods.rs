//! The shapes of herdsman's own client-shell methods, pinned apart from the
//! frozen generation-1 fixture.
//!
//! herdsman's own module. `client_commands.rs` reaches it through hooks marked
//! `// fork: stacks, worktree groups`.

use std::collections::BTreeMap;

/// Check and drop herdsman's methods from the advertised shapes, leaving the
/// rest for the generation-1 fixture. A failure here means one of them, or a
/// parameter type it shares, changed shape.
pub(super) fn remove_pinned(actual: &mut BTreeMap<String, String>) {
    assert_eq!(
        actual.remove("pane.stack").as_deref(),
        Some("ea12f3ce7056d68bf76e3dc99ad7d9a3d7385616743ba46ac082270c9d2d40f3")
    );
    assert_eq!(
        actual.remove("pane.focus_stacked").as_deref(),
        Some("01d2d20ff540b72125aeed4aa91ce3d73d92ba5594e403aef98099a3186af5dc")
    );
    assert_eq!(
        actual.remove("pane.focus_stacked_at").as_deref(),
        Some("ec4aa0fc311d09a9c46e1550000180ed5c6887b6b9d4b614dfb2aabe12599bb6")
    );
    assert_eq!(
        actual.remove("pane.focus_stack_strip").as_deref(),
        Some("2ce96fad327371c1442b5d1e84c9f09628e38833d47c3b93dd53ba295004a283")
    );
    assert_eq!(
        actual.remove("pane.stacks").as_deref(),
        Some("c6d7c7e43e70e2f17108eff0bc5ee132c729b580b4cb9b266232243b1864b798")
    );
    assert_eq!(
        actual.remove("workspace.set_worktree_group").as_deref(),
        Some("321caf6a2508a073b2ad79f557b6f819d5bce7bd0e9618de913504a7531de73a")
    );
}
