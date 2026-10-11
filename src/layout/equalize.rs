//! Equal widths or equal heights for every pane in a tab.
//!
//! herdsman's own module. `layout.rs` reaches it through a hook marked
//! `// fork: equalize`.

use super::*;

impl TileLayout {
    /// Make every column the same width (`Direction::Horizontal`) or every row
    /// the same height (`Direction::Vertical`). Splits along the other axis keep
    /// their ratios. Returns whether any ratio changed.
    ///
    /// A pane that spans several columns, such as one above a split row, counts
    /// as wide as the columns beneath it, so the narrowest columns all end up
    /// equal. A stack is one pane.
    pub fn equalize(&mut self, direction: Direction) -> bool {
        let mut changed = false;
        equalize_node(&mut self.root, direction, &mut changed);
        changed
    }
}

/// Set each split along `direction` in proportion to the panes side by side in
/// its two halves, and return how many panes sit side by side in `node`.
fn equalize_node(node: &mut Node, direction: Direction, changed: &mut bool) -> u32 {
    match node {
        Node::Split {
            direction: split_direction,
            ratio,
            first,
            second,
        } => {
            let first_span = equalize_node(first, direction, changed);
            let second_span = equalize_node(second, direction, changed);
            if *split_direction != direction {
                return first_span.max(second_span);
            }
            let equal = valid_split_ratio(first_span as f32 / (first_span + second_span) as f32);
            if *ratio != equal {
                *ratio = equal;
                *changed = true;
            }
            first_span + second_span
        }
        Node::Pane(_) | Node::Stack { .. } => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 60,
    };

    fn pane(id: u32) -> Box<Node> {
        Box::new(Node::Pane(PaneId::from_raw(id)))
    }

    fn split(direction: Direction, ratio: f32, first: Box<Node>, second: Box<Node>) -> Box<Node> {
        Box::new(Node::Split {
            direction,
            ratio,
            first,
            second,
        })
    }

    fn layout(root: Node) -> TileLayout {
        TileLayout::from_saved(root, PaneId::from_raw(1))
    }

    /// Each pane's width and height, by id.
    fn sizes(layout: &TileLayout) -> Vec<(u32, u16, u16)> {
        let mut sizes: Vec<_> = layout
            .panes(AREA)
            .iter()
            .map(|info| (info.id.raw(), info.rect.width, info.rect.height))
            .collect();
        sizes.sort();
        sizes
    }

    #[test]
    fn three_columns_get_equal_widths_however_they_were_split() {
        use Direction::Horizontal as H;
        // Splitting right twice nests the third column inside the second.
        let mut nested_right = layout(*split(H, 0.5, pane(1), split(H, 0.5, pane(2), pane(3))));
        // Splitting the first pane again nests it on the left instead.
        let mut nested_left = layout(*split(H, 0.7, split(H, 0.2, pane(1), pane(2)), pane(3)));

        for layout in [&mut nested_right, &mut nested_left] {
            assert!(layout.equalize(H));
            let widths: Vec<_> = sizes(layout).iter().map(|(_, w, _)| *w).collect();
            assert_eq!(widths, [40, 40, 40]);
        }
    }

    #[test]
    fn widths_leave_heights_alone_and_heights_leave_widths_alone() {
        use Direction::{Horizontal as H, Vertical as V};
        let mut layout = layout(*split(H, 0.7, pane(1), split(V, 0.25, pane(2), pane(3))));

        assert!(layout.equalize(H));
        assert_eq!(sizes(&layout), [(1, 60, 60), (2, 60, 15), (3, 60, 45)]);

        assert!(layout.equalize(V));
        assert_eq!(sizes(&layout), [(1, 60, 60), (2, 60, 30), (3, 60, 30)]);
    }

    #[test]
    fn a_pane_over_split_columns_spans_them() {
        use Direction::{Horizontal as H, Vertical as V};
        // 1 | (2 above 3|4): columns 1, 3 and 4 end up equal, and 2 spans 3 and 4.
        let mut layout = layout(*split(
            H,
            0.5,
            pane(1),
            split(V, 0.5, pane(2), split(H, 0.5, pane(3), pane(4))),
        ));

        assert!(layout.equalize(H));
        let widths: Vec<_> = sizes(&layout).iter().map(|(id, w, _)| (*id, *w)).collect();
        assert_eq!(widths, [(1, 40), (2, 80), (3, 40), (4, 40)]);
    }

    #[test]
    fn a_stack_counts_as_one_pane() {
        use Direction::Horizontal as H;
        let stack = Box::new(Node::Stack {
            panes: vec![PaneId::from_raw(2), PaneId::from_raw(3)],
            active: 0,
        });
        let mut layout = layout(*split(H, 0.5, pane(1), split(H, 0.3, stack, pane(4))));

        assert!(layout.equalize(H));
        let widths: Vec<_> = sizes(&layout).iter().map(|(id, w, _)| (*id, *w)).collect();
        assert_eq!(widths, [(1, 40), (2, 40), (4, 40)]);
    }

    #[test]
    fn equal_or_unsplit_layouts_report_no_change() {
        use Direction::{Horizontal as H, Vertical as V};
        let mut single = layout(*pane(1));
        assert!(!single.equalize(H));

        let mut even = layout(*split(H, 0.5, pane(1), pane(2)));
        assert!(!even.equalize(H));
        assert!(!even.equalize(V));
    }

    #[test]
    fn many_columns_keep_ratios_in_the_valid_range() {
        use Direction::Horizontal as H;
        let mut root = pane(12);
        for id in (1..12).rev() {
            root = split(H, 0.5, pane(id), root);
        }
        let mut layout = layout(*root);

        assert!(layout.equalize(H));
        let Node::Split { ratio, .. } = layout.root() else {
            panic!("root is a split");
        };
        assert_eq!(*ratio, 0.1);
    }
}
