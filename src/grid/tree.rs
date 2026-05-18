use taffy::NodeId;

use super::operations::SplitDirection;
use super::panel::PanelId;

/// A recursive N-ary split tree node representing the layout structure.
///
/// Each node is either a Leaf (a panel) or a Branch (a container splitting
/// its children in a given direction). This replaces the flat
/// column_containers model with arbitrary nesting depth.
#[derive(Debug, Clone)]
pub enum SplitNode {
    /// A terminal node containing a single panel.
    Leaf {
        panel_id: PanelId,
        taffy_node: NodeId,
    },
    /// A container node splitting its children horizontally or vertically.
    Branch {
        direction: SplitDirection,
        children: Vec<SplitNode>,
        weights: Vec<f32>,
        taffy_node: NodeId,
    },
}

impl SplitNode {
    /// Returns the taffy NodeId associated with this node.
    pub fn taffy_node_id(&self) -> NodeId {
        match self {
            SplitNode::Leaf { taffy_node, .. } => *taffy_node,
            SplitNode::Branch { taffy_node, .. } => *taffy_node,
        }
    }

    /// Returns true if this node contains a leaf with the given panel ID.
    /// Searches recursively through branches.
    pub fn contains_panel(&self, target: PanelId) -> bool {
        match self {
            SplitNode::Leaf { panel_id, .. } => *panel_id == target,
            SplitNode::Branch { children, .. } => {
                children.iter().any(|child| child.contains_panel(target))
            }
        }
    }

    /// Returns true if this node is a Leaf variant.
    pub fn is_leaf(&self) -> bool {
        matches!(self, SplitNode::Leaf { .. })
    }

    /// Returns the number of leaf nodes in this subtree.
    /// A Leaf returns 1; a Branch returns the sum of its children's leaf counts.
    pub fn leaf_count(&self) -> usize {
        match self {
            SplitNode::Leaf { .. } => 1,
            SplitNode::Branch { children, .. } => {
                children.iter().map(|child| child.leaf_count()).sum()
            }
        }
    }

    /// Collects all (NodeId, PanelId) pairs from leaf nodes in tree order.
    pub fn collect_leaves(&self) -> Vec<(NodeId, PanelId)> {
        match self {
            SplitNode::Leaf {
                panel_id,
                taffy_node,
            } => vec![(*taffy_node, *panel_id)],
            SplitNode::Branch { children, .. } => {
                children.iter().flat_map(|child| child.collect_leaves()).collect()
            }
        }
    }

    /// Normalizes weights in a Branch so they sum to 1.0.
    /// No-op on Leaf nodes.
    pub fn normalize_weights(&mut self) {
        if let SplitNode::Branch { weights, .. } = self {
            let sum: f32 = weights.iter().sum();
            if sum > 0.0 {
                for w in weights.iter_mut() {
                    *w /= sum;
                }
            }
        }
    }

    /// Finds the parent Branch of a leaf with the given panel ID.
    /// Returns (parent_branch, child_index) or None if not found or target is root.
    pub fn find_parent_of(&self, target: PanelId) -> Option<(&SplitNode, usize)> {
        match self {
            SplitNode::Leaf { .. } => None,
            SplitNode::Branch { children, .. } => {
                // Check if any direct child is the target leaf
                for (i, child) in children.iter().enumerate() {
                    if let SplitNode::Leaf { panel_id, .. } = child {
                        if *panel_id == target {
                            return Some((self, i));
                        }
                    }
                }
                // Recurse into branch children
                for child in children {
                    if let result @ Some(_) = child.find_parent_of(target) {
                        return result;
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::prelude::*;

    /// Helper: creates a test tree using a real TaffyTree for valid NodeIds.
    ///
    /// Structure:
    ///   Root Branch(Horizontal)
    ///     |-- Leaf(PanelId(0))
    ///     |-- Branch(Vertical)
    ///           |-- Leaf(PanelId(1))
    ///           |-- Leaf(PanelId(2))
    fn make_test_tree() -> (SplitNode, TaffyTree<()>) {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let n0 = taffy.new_leaf(Style::default()).unwrap();
        let n1 = taffy.new_leaf(Style::default()).unwrap();
        let n2 = taffy.new_leaf(Style::default()).unwrap();
        let inner = taffy
            .new_with_children(Style::default(), &[n1, n2])
            .unwrap();
        let root = taffy
            .new_with_children(Style::default(), &[n0, inner])
            .unwrap();
        let _ = root; // root is used only for taffy tree structure

        let tree = SplitNode::Branch {
            direction: SplitDirection::Horizontal,
            children: vec![
                SplitNode::Leaf {
                    panel_id: PanelId(0),
                    taffy_node: n0,
                },
                SplitNode::Branch {
                    direction: SplitDirection::Vertical,
                    children: vec![
                        SplitNode::Leaf {
                            panel_id: PanelId(1),
                            taffy_node: n1,
                        },
                        SplitNode::Leaf {
                            panel_id: PanelId(2),
                            taffy_node: n2,
                        },
                    ],
                    weights: vec![0.5, 0.5],
                    taffy_node: inner,
                },
            ],
            weights: vec![0.5, 0.5],
            taffy_node: root,
        };

        (tree, taffy)
    }

    #[test]
    fn test_leaf_contains_panel_id_and_taffy_node() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let n = taffy.new_leaf(Style::default()).unwrap();
        let leaf = SplitNode::Leaf {
            panel_id: PanelId(42),
            taffy_node: n,
        };
        assert!(leaf.is_leaf());
        assert_eq!(leaf.taffy_node_id(), n);
    }

    #[test]
    fn test_branch_contains_direction_children_weights_taffy_node() {
        let (tree, _taffy) = make_test_tree();
        match &tree {
            SplitNode::Branch {
                direction,
                children,
                weights,
                taffy_node: _,
            } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
                assert_eq!(weights.len(), 2);
            }
            _ => panic!("Expected Branch"),
        }
    }

    #[test]
    fn test_contains_panel_leaf_match() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let n = taffy.new_leaf(Style::default()).unwrap();
        let leaf = SplitNode::Leaf {
            panel_id: PanelId(5),
            taffy_node: n,
        };
        assert!(leaf.contains_panel(PanelId(5)));
        assert!(!leaf.contains_panel(PanelId(99)));
    }

    #[test]
    fn test_contains_panel_recursive() {
        let (tree, _taffy) = make_test_tree();
        assert!(tree.contains_panel(PanelId(0)));
        assert!(tree.contains_panel(PanelId(1)));
        assert!(tree.contains_panel(PanelId(2)));
        assert!(!tree.contains_panel(PanelId(99)));
    }

    #[test]
    fn test_taffy_node_id_leaf_and_branch() {
        let (tree, _taffy) = make_test_tree();
        // Root branch has a taffy_node
        let _ = tree.taffy_node_id(); // should not panic

        // Inner leaf
        if let SplitNode::Branch { children, .. } = &tree {
            let leaf_node = children[0].taffy_node_id();
            let _ = leaf_node; // should not panic
        }
    }

    #[test]
    fn test_leaf_count_single() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let n = taffy.new_leaf(Style::default()).unwrap();
        let leaf = SplitNode::Leaf {
            panel_id: PanelId(0),
            taffy_node: n,
        };
        assert_eq!(leaf.leaf_count(), 1);
    }

    #[test]
    fn test_leaf_count_branch() {
        let (tree, _taffy) = make_test_tree();
        assert_eq!(tree.leaf_count(), 3);
    }

    #[test]
    fn test_collect_leaves_tree_order() {
        let (tree, _taffy) = make_test_tree();
        let leaves = tree.collect_leaves();
        assert_eq!(leaves.len(), 3);
        assert_eq!(leaves[0].1, PanelId(0));
        assert_eq!(leaves[1].1, PanelId(1));
        assert_eq!(leaves[2].1, PanelId(2));
    }

    #[test]
    fn test_normalize_weights_sums_to_one() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let n0 = taffy.new_leaf(Style::default()).unwrap();
        let n1 = taffy.new_leaf(Style::default()).unwrap();
        let nb = taffy
            .new_with_children(Style::default(), &[n0, n1])
            .unwrap();
        let mut branch = SplitNode::Branch {
            direction: SplitDirection::Horizontal,
            children: vec![
                SplitNode::Leaf {
                    panel_id: PanelId(0),
                    taffy_node: n0,
                },
                SplitNode::Leaf {
                    panel_id: PanelId(1),
                    taffy_node: n1,
                },
            ],
            weights: vec![3.0, 7.0],
            taffy_node: nb,
        };
        branch.normalize_weights();
        if let SplitNode::Branch { weights, .. } = &branch {
            let sum: f32 = weights.iter().sum();
            assert!((sum - 1.0).abs() < 0.001, "Sum should be ~1.0, got {}", sum);
            assert!(
                (weights[0] - 0.3).abs() < 0.001,
                "Expected ~0.3, got {}",
                weights[0]
            );
            assert!(
                (weights[1] - 0.7).abs() < 0.001,
                "Expected ~0.7, got {}",
                weights[1]
            );
        }
    }

    #[test]
    fn test_normalize_weights_uneven_three() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let n0 = taffy.new_leaf(Style::default()).unwrap();
        let n1 = taffy.new_leaf(Style::default()).unwrap();
        let n2 = taffy.new_leaf(Style::default()).unwrap();
        let nb = taffy
            .new_with_children(Style::default(), &[n0, n1, n2])
            .unwrap();
        let mut branch = SplitNode::Branch {
            direction: SplitDirection::Horizontal,
            children: vec![
                SplitNode::Leaf {
                    panel_id: PanelId(0),
                    taffy_node: n0,
                },
                SplitNode::Leaf {
                    panel_id: PanelId(1),
                    taffy_node: n1,
                },
                SplitNode::Leaf {
                    panel_id: PanelId(2),
                    taffy_node: n2,
                },
            ],
            weights: vec![0.5, 0.3, 0.7],
            taffy_node: nb,
        };
        branch.normalize_weights();
        if let SplitNode::Branch { weights, .. } = &branch {
            let sum: f32 = weights.iter().sum();
            assert!((sum - 1.0).abs() < 0.001, "Sum should be ~1.0, got {}", sum);
            assert!(
                (weights[0] - 0.333).abs() < 0.01,
                "Expected ~0.333, got {}",
                weights[0]
            );
            assert!(
                (weights[1] - 0.2).abs() < 0.01,
                "Expected ~0.2, got {}",
                weights[1]
            );
            assert!(
                (weights[2] - 0.467).abs() < 0.01,
                "Expected ~0.467, got {}",
                weights[2]
            );
        }
    }

    #[test]
    fn test_find_parent_of() {
        let (tree, _taffy) = make_test_tree();

        // PanelId(0) is a direct child of root branch
        let result = tree.find_parent_of(PanelId(0));
        assert!(result.is_some());
        let (parent, idx) = result.unwrap();
        assert_eq!(idx, 0);
        assert!(!parent.is_leaf());

        // PanelId(1) is inside the inner branch at index 0
        let result = tree.find_parent_of(PanelId(1));
        assert!(result.is_some());
        let (parent, idx) = result.unwrap();
        assert_eq!(idx, 0);
        if let SplitNode::Branch { direction, .. } = parent {
            assert_eq!(*direction, SplitDirection::Vertical);
        } else {
            panic!("Expected Branch parent for PanelId(1)");
        }

        // PanelId(2) is inside the inner branch at index 1
        let result = tree.find_parent_of(PanelId(2));
        assert!(result.is_some());
        let (_parent, idx) = result.unwrap();
        assert_eq!(idx, 1);

        // Non-existent panel
        let result = tree.find_parent_of(PanelId(99));
        assert!(result.is_none());
    }
}
