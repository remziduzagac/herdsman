//! Saving and restoring stacks with a session.
//!
//! herdsman's own module. `restore.rs` reaches it through hooks marked
//! `// fork: stacks`.

use super::*;

/// `prune_restored_node` for a stack: keep the surviving members, and keep
/// showing the visible one when it survived.
pub(super) fn prune(
    panes: Vec<PaneId>,
    active: usize,
    surviving: &HashSet<PaneId>,
) -> Option<Node> {
    let visible = panes.get(active).copied();
    let panes: Vec<PaneId> = panes
        .into_iter()
        .filter(|id| surviving.contains(id))
        .collect();
    let active = visible
        .and_then(|visible| panes.iter().position(|id| *id == visible))
        .unwrap_or(0);
    crate::layout::stack::stack_node(panes, active)
}

/// `remap_inner` for a stack: fresh ids for every member. Kept as saved;
/// pruning normalizes the member count and visible index.
pub(super) fn remap(panes: &[u32], active: usize, id_map: &mut HashMap<u32, PaneId>) -> Node {
    Node::Stack {
        panes: panes
            .iter()
            .map(|old_id| {
                let new_id = PaneId::alloc();
                id_map.insert(*old_id, new_id);
                new_id
            })
            .collect(),
        active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_round_trips_through_snapshot_json_with_fresh_ids() {
        let node = Node::Split {
            direction: Direction::Horizontal,
            ratio: 0.4,
            first: Box::new(Node::Pane(PaneId::from_raw(0))),
            second: Box::new(Node::Stack {
                panes: vec![
                    PaneId::from_raw(1),
                    PaneId::from_raw(2),
                    PaneId::from_raw(3),
                ],
                active: 2,
            }),
        };

        let json = serde_json::to_string(&crate::persist::snapshot::capture_node(&node)).unwrap();
        assert!(
            json.contains(r#"{"Stack":{"panes":[1,2,3],"active":2}}"#),
            "{json}"
        );
        let snap: LayoutSnapshot = serde_json::from_str(&json).unwrap();
        let (restored, id_map) = restore_node_remapped(&snap);

        let Node::Split { second, .. } = restored else {
            panic!("expected split root");
        };
        let Node::Stack { panes, active } = *second else {
            panic!("expected restored stack");
        };
        assert_eq!(panes, [1, 2, 3].map(|old| id_map[&old]));
        assert_eq!(active, 2);
    }

    #[test]
    fn prune_keeps_the_visible_member_and_collapses_stacks_that_lose_members() {
        let ids = [31, 32, 33].map(PaneId::from_raw);
        let stacked = |active| Node::Stack {
            panes: ids.to_vec(),
            active,
        };

        let pruned = prune_restored_node(stacked(2), &HashSet::from([ids[1], ids[2]]))
            .expect("two members survive");
        assert!(matches!(pruned, Node::Stack { ref panes, active: 1 } if panes == &ids[1..]));

        let pruned = prune_restored_node(stacked(1), &HashSet::from([ids[0], ids[2]]))
            .expect("two members survive");
        assert!(matches!(pruned, Node::Stack { active: 0, .. }));

        let pruned =
            prune_restored_node(stacked(0), &HashSet::from([ids[2]])).expect("one member survives");
        assert!(matches!(pruned, Node::Pane(id) if id == ids[2]));

        assert!(prune_restored_node(stacked(0), &HashSet::new()).is_none());
    }

    #[test]
    fn malformed_saved_stacks_restore_as_valid_layouts() {
        let lone: LayoutSnapshot =
            serde_json::from_str(r#"{"Stack":{"panes":[7],"active":5}}"#).unwrap();
        let (node, id_map) = restore_node_remapped(&lone);
        let surviving = id_map.values().copied().collect();
        assert!(matches!(
            prune_restored_node(node, &surviving),
            Some(Node::Pane(id)) if id == id_map[&7]
        ));

        let out_of_range: LayoutSnapshot =
            serde_json::from_str(r#"{"Stack":{"panes":[7,8],"active":9}}"#).unwrap();
        let (node, id_map) = restore_node_remapped(&out_of_range);
        let surviving = id_map.values().copied().collect();
        assert!(matches!(
            prune_restored_node(node, &surviving),
            Some(Node::Stack { active: 0, .. })
        ));

        let empty: LayoutSnapshot =
            serde_json::from_str(r#"{"Stack":{"panes":[],"active":0}}"#).unwrap();
        let (node, _) = restore_node_remapped(&empty);
        assert!(prune_restored_node(node, &HashSet::new()).is_none());
    }
}
