//! The shapes of herdsman's own client-shell methods, pinned apart from the
//! frozen generation-1 fixture.
//!
//! herdsman's own module. `client_commands.rs` reaches it through hooks marked
//! `// fork:` with the feature names.

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
        actual.remove("layout.equalize").as_deref(),
        Some("94f7408bcd95f009d28c60cdf46adfb116d8ee0b37bab302a8871ab6dfb10674")
    );
    assert_eq!(
        actual.remove("preset.apply").as_deref(),
        Some("cd4db3562d26d948b9b821d216037dad9b7254d7f8b4c03015ab445c30dcb194")
    );
    assert_eq!(
        actual.remove("preset.default").as_deref(),
        Some("4197b2cbfbb1befa02a80db2981529966152c867905481c0e8c14ac0f9fbf36a")
    );
    assert_eq!(
        actual.remove("preset.delete").as_deref(),
        Some("425ec70376e6e7dacd55d0fa43dd36d529d1df227d1c80991df68b3e78007935")
    );
    assert_eq!(
        actual.remove("preset.edit").as_deref(),
        Some("432a6e8885dfa1868fd2fd6f098fd9c9b15213fbb91b81f6805603f46ce45f10")
    );
    assert_eq!(
        actual.remove("preset.get").as_deref(),
        Some("cf62a75e37f160d5b3b11d4b95bd6b7a7bfb73247df6fc422d7a4785db5f3905")
    );
    assert_eq!(
        actual.remove("preset.list").as_deref(),
        Some("e77d8c6471454e99c8b463d01cc07b877232aca292c0b779a151c4bbc98d91a4")
    );
    assert_eq!(
        actual.remove("preset.load").as_deref(),
        Some("3ba5e2d1211c67efb5b2ad48dff7170f1e3c7f3f62d01f9c6cf208f6c06df449")
    );
    assert_eq!(
        actual.remove("preset.rename").as_deref(),
        Some("4d72b4f8d3d2041e52aa94e3016c0018a04e354b79a7a3671d6dd9aa3f46b6b7")
    );
    assert_eq!(
        actual.remove("preset.save").as_deref(),
        Some("1ef7397d832adbbf8c68b921936c449376fd790ae0c1440240e1222fe22d5876")
    );
    assert_eq!(
        actual.remove("update.handoff").as_deref(),
        Some("f88eccac0ca78cf7fc6fc52ef26ff2935ffc458e7974af363141f31a37bd7a5d")
    );
    assert_eq!(
        actual.remove("update.install").as_deref(),
        Some("40c1849f9ace1e26dc0be5785aafcfe8f37acf7a7e38ce73c2cce83e5dbb369a")
    );
    assert_eq!(
        actual.remove("update.integrations").as_deref(),
        Some("840597ea2c27bdb84583c37a28b0735792f318b5353c0756198fddda821bb981")
    );
    assert_eq!(
        actual.remove("update.skip").as_deref(),
        Some("1ca82d7a492163dda19ca7b1fc46bf44307b4726663f93dfdecbcc3b91d3b961")
    );
    assert_eq!(
        actual.remove("update.status").as_deref(),
        Some("7f2f98ad414cbf6726bac4802fa0ed7c8f8297d3530fcdf02418a3164c6883f2")
    );
    assert_eq!(
        actual.remove("workspace.set_worktree_group").as_deref(),
        Some("321caf6a2508a073b2ad79f557b6f819d5bce7bd0e9618de913504a7531de73a")
    );
}
