//! Stacks: one layout slot holding several panes, only one of them visible.
//!
//! herdsman's own module. `layout.rs` reaches it through hooks marked
//! `// fork: stacks`.

use ratatui::layout::{Direction, Rect};

use super::{collect_panes, find_pane_mut, remove_pane, Node, PaneId, PaneInfo, TileLayout};

impl TileLayout {
    /// Move `pane` out of its slot and append it to `target`'s slot as a stack
    /// member. Only a focused `pane` becomes the visible member; otherwise the
    /// slot keeps showing what it showed. False when either pane is missing,
    /// they are the same pane, or `pane` already shares `target`'s stack.
    pub fn move_into_stack(&mut self, target: PaneId, pane: PaneId) -> bool {
        let ids = self.pane_ids();
        if target == pane || !ids.contains(&target) || !ids.contains(&pane) {
            return false;
        }
        if self
            .stack_members(target)
            .is_some_and(|(members, _)| members.contains(&pane))
        {
            return false;
        }
        // `target` is in the tree and is not the pane leaving it, so removal
        // never empties the tree and the target's slot is always found.
        let placeholder = PaneId::from_raw(0);
        let old = std::mem::replace(&mut self.root, Node::Pane(placeholder));
        self.root = remove_pane(old, pane).unwrap_or(Node::Pane(target));
        if let Some(slot) = find_pane_mut(&mut self.root, target) {
            match slot {
                Node::Pane(id) => {
                    let id = *id;
                    *slot = Node::Stack {
                        panes: vec![id, pane],
                        active: 0,
                    };
                }
                Node::Stack { panes, .. } => panes.push(pane),
                Node::Split { .. } => {}
            }
        }
        reveal_pane(&mut self.root, self.focus);
        true
    }

    /// Make `pane` its stack's visible member. Focus follows only when it sat
    /// on the member being hidden. False when `pane` is not stacked.
    pub fn show_in_stack(&mut self, pane: PaneId) -> bool {
        let Some((members, active)) = self.stack_members(pane) else {
            return false;
        };
        let hides_focus = members.get(active) == Some(&self.focus);
        reveal_pane(&mut self.root, pane);
        if hides_focus {
            self.set_focus(pane);
        }
        true
    }

    /// Members of the stack holding `pane`, and the index of the visible one.
    pub fn stack_members(&self, pane: PaneId) -> Option<(&[PaneId], usize)> {
        find_stack(&self.root, pane)
    }

    /// Whether `pane` is drawn: it is not stacked, or it is its stack's visible
    /// member. Hidden members keep running but are off screen.
    pub fn is_visible(&self, pane: PaneId) -> bool {
        self.stack_members(pane)
            .is_none_or(|(members, active)| members.get(active) == Some(&pane))
    }

    pub(super) fn visible_in_slot(&self, pane: PaneId) -> PaneId {
        self.stack_members(pane)
            .and_then(|(members, active)| members.get(active).copied())
            .unwrap_or(pane)
    }
}

/// The node for a set of stack members: nothing when empty, a plain pane for
/// one, otherwise a stack whose visible index is clamped into range.
pub fn stack_node(panes: Vec<PaneId>, active: usize) -> Option<Node> {
    match panes.as_slice() {
        [] => None,
        [only] => Some(Node::Pane(*only)),
        _ => Some(Node::Stack {
            active: active.min(panes.len() - 1),
            panes,
        }),
    }
}

fn find_stack(node: &Node, pane: PaneId) -> Option<(&[PaneId], usize)> {
    match node {
        Node::Pane(_) => None,
        Node::Stack { panes, active } => {
            panes.contains(&pane).then_some((panes.as_slice(), *active))
        }
        Node::Split { first, second, .. } => {
            find_stack(first, pane).or_else(|| find_stack(second, pane))
        }
    }
}

/// Make `pane` its stack's visible member. False when it is not stacked.
/// `layout.rs` calls this wherever focus moves, because the focused pane is
/// always visible: focusing a hidden member shows it.
pub(super) fn reveal_pane(node: &mut Node, pane: PaneId) -> bool {
    match node {
        Node::Pane(_) => false,
        Node::Stack { panes, active } => match panes.iter().position(|id| *id == pane) {
            Some(index) => {
                *active = index;
                true
            }
            None => false,
        },
        Node::Split { first, second, .. } => reveal_pane(first, pane) || reveal_pane(second, pane),
    }
}

/// Split a whole slot, so splitting a stack member splits beside its stack.
pub(super) fn split_slot(slot: &mut Node, direction: Direction, new_id: PaneId, split_ratio: f32) {
    let first = std::mem::replace(slot, Node::Pane(new_id));
    *slot = Node::Split {
        direction,
        ratio: split_ratio,
        first: Box::new(first),
        second: Box::new(Node::Pane(new_id)),
    };
}

/// `remove_pane` for a stack: drop `target` from the members, if it is one.
pub(super) fn remove_member(mut panes: Vec<PaneId>, active: usize, target: PaneId) -> Option<Node> {
    let Some(index) = panes.iter().position(|id| *id == target) else {
        return Some(Node::Stack { panes, active });
    };
    panes.remove(index);
    // The member after the removed one takes its place when it was visible.
    let active = if index < active { active - 1 } else { active };
    stack_node(panes, active)
}

/// `swap_pane_ids` for a stack's members.
pub(super) fn swap_member_ids(panes: &mut [PaneId], first: PaneId, second: PaneId) {
    for id in panes {
        if *id == first {
            *id = second;
        } else if *id == second {
            *id = first;
        }
    }
}

/// `collect_panes` for a stack: only the visible member is drawn, across the
/// whole slot.
pub(super) fn collect_visible(
    panes: &[PaneId],
    active: usize,
    area: Rect,
    focus: PaneId,
    result: &mut Vec<PaneInfo>,
) {
    if let Some(visible) = panes.get(active) {
        collect_panes(&Node::Pane(*visible), area, focus, result);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{pane_rect, split_snapshot};
    use super::super::*;
    use super::*;

    fn pane(id: u32) -> PaneId {
        PaneId::from_raw(id)
    }

    /// Tree shape as text: `H(..)`/`V(..)` splits, `S[..]` stacks with the
    /// visible member starred.
    fn shape(layout: &TileLayout) -> String {
        fn describe(node: &Node) -> String {
            match node {
                Node::Pane(id) => id.raw().to_string(),
                Node::Split {
                    direction,
                    first,
                    second,
                    ..
                } => {
                    let tag = if *direction == Direction::Horizontal {
                        "H"
                    } else {
                        "V"
                    };
                    format!("{tag}({},{})", describe(first), describe(second))
                }
                Node::Stack { panes, active } => {
                    let members = panes
                        .iter()
                        .enumerate()
                        .map(|(index, id)| {
                            let star = if index == *active { "*" } else { "" };
                            format!("{star}{}", id.raw())
                        })
                        .collect::<Vec<_>>();
                    format!("S[{}]", members.join(","))
                }
            }
        }
        describe(layout.root())
    }

    fn stack(ids: &[u32], active: usize) -> Node {
        Node::Stack {
            panes: ids.iter().copied().map(pane).collect(),
            active,
        }
    }

    /// `1 | S[2,3,4] | 5` with 3 visible, focus on 1.
    fn three_column_stack_layout() -> TileLayout {
        TileLayout::from_saved(
            Node::Split {
                direction: Direction::Horizontal,
                ratio: 0.3,
                first: Box::new(Node::Pane(pane(1))),
                second: Box::new(Node::Split {
                    direction: Direction::Horizontal,
                    ratio: 0.5,
                    first: Box::new(stack(&[2, 3, 4], 1)),
                    second: Box::new(Node::Pane(pane(5))),
                }),
            },
            pane(1),
        )
    }

    #[test]
    fn stack_draws_only_its_visible_member_across_the_whole_slot() {
        let layout = three_column_stack_layout();
        let area = Rect::new(0, 0, 100, 40);

        let visible = layout.panes(area);
        assert_eq!(
            visible.iter().map(|info| info.id).collect::<Vec<_>>(),
            [pane(1), pane(3), pane(5)]
        );
        assert_eq!(pane_rect(&layout, pane(3)), Rect::new(30, 0, 35, 40));
        assert_eq!(layout.splits(area).len(), 2);
        assert_eq!(layout.pane_count(), 5);
        assert_eq!(
            layout.pane_ids(),
            [pane(1), pane(2), pane(3), pane(4), pane(5)]
        );
        assert!(layout.is_visible(pane(3)));
        assert!(!layout.is_visible(pane(2)));
        assert!(layout.is_visible(pane(1)));
        let (members, active) = layout.stack_members(pane(4)).expect("stacked");
        assert_eq!((members, active), (&[pane(2), pane(3), pane(4)][..], 1));
        assert!(layout.stack_members(pane(1)).is_none());
    }

    #[test]
    fn focusing_a_hidden_member_makes_it_visible() {
        let mut layout = three_column_stack_layout();

        layout.focus_pane(pane(4));

        assert_eq!(shape(&layout), "H(1,H(S[2,3,*4],5))");
        assert_eq!(layout.focused(), pane(4));
        assert!(layout.close_focused());
        assert_eq!(layout.focused(), pane(1));
        assert_eq!(shape(&layout), "H(1,H(S[2,*3],5))");
    }

    #[test]
    fn directional_navigation_enters_and_leaves_a_stack_through_its_slot() {
        let mut layout = three_column_stack_layout();
        let area = Rect::new(0, 0, 100, 40);
        let from = |layout: &TileLayout, id: PaneId, direction| {
            let panes = layout.panes(area);
            let focused = panes.iter().find(|info| info.id == id).unwrap().clone();
            find_in_direction(&focused, direction, &panes)
        };

        assert_eq!(from(&layout, pane(1), NavDirection::Right), Some(pane(3)));
        layout.focus_pane(pane(2));
        assert_eq!(from(&layout, pane(2), NavDirection::Left), Some(pane(1)));
        assert_eq!(from(&layout, pane(2), NavDirection::Right), Some(pane(5)));
        assert_eq!(from(&layout, pane(5), NavDirection::Left), Some(pane(2)));
    }

    #[test]
    fn moving_into_a_stack_keeps_the_slot_unless_the_moved_pane_has_focus() {
        let mut layout = TileLayout::from_saved(
            Node::Split {
                direction: Direction::Horizontal,
                ratio: 0.5,
                first: Box::new(Node::Pane(pane(1))),
                second: Box::new(Node::Split {
                    direction: Direction::Vertical,
                    ratio: 0.5,
                    first: Box::new(Node::Pane(pane(2))),
                    second: Box::new(Node::Pane(pane(3))),
                }),
            },
            pane(1),
        );

        assert!(layout.move_into_stack(pane(1), pane(2)));
        assert_eq!(shape(&layout), "H(S[*1,2],3)");
        assert_eq!(layout.focused(), pane(1));

        layout.focus_pane(pane(3));
        assert!(layout.move_into_stack(pane(1), pane(3)));
        assert_eq!(shape(&layout), "S[1,2,*3]");
        assert_eq!(layout.focused(), pane(3));
        assert_eq!(layout.panes(Rect::new(0, 0, 100, 40)).len(), 1);
    }

    #[test]
    fn rejected_stack_moves_change_nothing() {
        let mut layout = three_column_stack_layout();
        let before = shape(&layout);

        assert!(!layout.move_into_stack(pane(1), pane(1)));
        assert!(!layout.move_into_stack(pane(1), pane(99)));
        assert!(!layout.move_into_stack(pane(99), pane(1)));
        assert!(!layout.move_into_stack(pane(2), pane(4)));

        assert_eq!(shape(&layout), before);
    }

    #[test]
    fn a_member_moved_out_of_one_stack_can_join_another() {
        let mut layout = three_column_stack_layout();

        assert!(layout.move_into_stack(pane(5), pane(3)));

        assert_eq!(shape(&layout), "H(1,H(S[2,*4],S[*5,3]))");
    }

    #[test]
    fn closing_members_keeps_the_visible_one_and_collapses_a_stack_of_one() {
        let mut layout = three_column_stack_layout();

        assert!(layout.close_pane(pane(2)));
        assert_eq!(shape(&layout), "H(1,H(S[*3,4],5))");
        assert!(layout.close_pane(pane(3)));
        assert_eq!(shape(&layout), "H(1,H(4,5))");
        assert_eq!(layout.focused(), pane(1));
    }

    #[test]
    fn closing_the_visible_last_member_shows_the_one_before_it() {
        let mut layout = TileLayout::from_saved(
            Node::Split {
                direction: Direction::Horizontal,
                ratio: 0.5,
                first: Box::new(Node::Pane(pane(9))),
                second: Box::new(stack(&[1, 2, 3], 2)),
            },
            pane(9),
        );

        assert!(layout.close_pane(pane(3)));

        assert_eq!(shape(&layout), "H(9,S[1,*2])");
        assert_eq!(layout.focused(), pane(9));
    }

    #[test]
    fn closing_the_focused_member_moves_focus_to_a_visible_pane() {
        let mut layout = TileLayout::from_saved(stack(&[1, 2, 3], 0), pane(1));

        assert!(layout.close_focused());

        assert_eq!(layout.focused(), pane(2));
        assert_eq!(shape(&layout), "S[*2,3]");
    }

    #[test]
    fn focus_returning_to_a_hidden_member_reveals_it() {
        let mut layout = three_column_stack_layout();
        layout.focus_pane(pane(3));
        layout.focus_pane(pane(1));
        // Swapping puts 2 in 3's place, leaving the remembered pane 3 hidden.
        assert!(layout.swap_panes(pane(2), pane(3)));
        assert_eq!(shape(&layout), "H(1,H(S[3,*2,4],5))");

        assert!(layout.close_focused());

        assert_eq!(layout.focused(), pane(3));
        assert_eq!(shape(&layout), "H(S[*3,2,4],5)");
    }

    #[test]
    fn swapping_the_focused_pane_into_a_stack_keeps_it_visible() {
        let mut layout = three_column_stack_layout();

        assert!(layout.swap_panes(pane(1), pane(4)));

        assert_eq!(layout.focused(), pane(1));
        assert_eq!(shape(&layout), "H(4,H(S[2,3,*1],5))");
    }

    #[test]
    fn splitting_a_member_splits_beside_the_whole_stack() {
        // Allocated ids only, so split_pane cannot collide with from_raw ids.
        let (mut layout, root) = TileLayout::new();
        let member = layout
            .split_pane(root, Direction::Horizontal, 0.5)
            .expect("root is in the layout");
        assert!(layout.move_into_stack(root, member));

        let beside = layout
            .split_pane(member, Direction::Vertical, 0.5)
            .expect("stacked target is in the layout");

        assert_eq!(
            shape(&layout),
            format!(
                "V(S[*{}],{})",
                [root, member].map(|id| id.raw().to_string()).join(","),
                beside.raw()
            )
        );
        assert_eq!(layout.focused(), root);
    }

    #[test]
    fn inserting_near_a_member_inserts_beside_the_whole_stack() {
        let mut layout = three_column_stack_layout();

        assert!(layout.insert_pane_near(pane(4), pane(9), Direction::Horizontal, 0.5, false));

        assert_eq!(shape(&layout), "H(1,H(H(S[2,*3,4],9),5))");
        assert_eq!(layout.focused(), pane(1));
    }

    #[test]
    fn resizing_a_hidden_member_resizes_its_slot() {
        let mut layout = TileLayout::from_saved(
            Node::Split {
                direction: Direction::Horizontal,
                ratio: 0.5,
                first: Box::new(Node::Pane(pane(1))),
                second: Box::new(stack(&[2, 3], 0)),
            },
            pane(1),
        );

        assert!(layout.resize_pane(pane(3), NavDirection::Left, 0.05, Rect::new(0, 0, 100, 40)));

        assert!((split_snapshot(&layout)[0].1 - 0.45).abs() < f32::EPSILON);
        assert_eq!(layout.focused(), pane(1));
        assert_eq!(shape(&layout), "H(1,S[*2,3])");
    }

    #[test]
    fn restored_focus_is_revealed_in_its_stack() {
        let layout = TileLayout::from_saved(stack(&[1, 2], 0), pane(2));

        assert_eq!(shape(&layout), "S[1,*2]");
    }

    #[test]
    fn stack_node_collapses_small_member_sets_and_clamps_the_visible_index() {
        let describe =
            |node: Option<Node>| node.map(|node| shape(&TileLayout::from_saved(node, pane(0))));

        assert_eq!(describe(stack_node(Vec::new(), 0)), None);
        assert_eq!(describe(stack_node(vec![pane(1)], 4)).as_deref(), Some("1"));
        assert_eq!(
            describe(stack_node(vec![pane(1), pane(2)], 9)).as_deref(),
            Some("S[1,*2]")
        );
    }
}
