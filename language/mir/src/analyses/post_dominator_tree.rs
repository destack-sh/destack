use std::collections::HashSet;

use crate::{Block, ControlFlowGraph, Function, LocalNodeId, NodeTable, Tree};

/// Postdominator tree for one function.
#[derive(Debug, Clone)]
pub struct PostDominatorTree {
    /// Immediate postdominator indexed by block id.
    immediate_postdominators: NodeTable<Block, Option<LocalNodeId<Block>>>,
    /// Preorder numbers indexed by block id.
    preorder: NodeTable<Block, Option<u32>>,
    /// Maximum preorder number in each subtree indexed by block id.
    preorder_max: NodeTable<Block, Option<u32>>,
}

impl PostDominatorTree {
    /// Build the postdominator tree for one function.
    pub fn build(function: &Function, tree: &Tree, cfg: &ControlFlowGraph) -> Self {
        let Some(entry) = function.entry() else {
            return Self {
                immediate_postdominators: NodeTable::new(),
                preorder: NodeTable::new(),
                preorder_max: NodeTable::new(),
            };
        };

        // exit blocks
        let mut exit_blocks = Self::collect_exit_blocks(function, tree);
        let has_exits = !exit_blocks.is_empty();

        if !has_exits {
            exit_blocks = function.blocks().iter().copied().collect();
        }

        let mut block_index = NodeTable::from_nodes(function.blocks(), || None);

        // block indices
        for (index, &block) in function.blocks().iter().enumerate() {
            *block_index.get_mut(block) = Some(index);
        }

        let virtual_root = function.blocks().len();
        let mut successors = vec![Vec::new(); function.blocks().len() + 1];
        let mut predecessors = vec![Vec::new(); function.blocks().len() + 1];

        // reverse edges
        for &block in function.blocks() {
            let index = block_index.expect(block);

            for predecessor in cfg.predecessors(block) {
                let predecessor_index = block_index.expect(*predecessor);
                successors[index].push(predecessor_index);
                predecessors[predecessor_index].push(index);
            }
        }

        // connect exits to the virtual root
        for exit_block in exit_blocks {
            let exit_index = block_index.expect(exit_block);
            successors[virtual_root].push(exit_index);
            predecessors[exit_index].push(virtual_root);
        }

        let mut immediate_postdominators = NodeTable::from_nodes(function.blocks(), || None);

        // postdominator edges
        if has_exits {
            let result = super::dominator_tree::DominatorComputation::compute(
                &successors,
                &predecessors,
                virtual_root,
            );

            for &block in function.blocks() {
                let index = block_index.expect(block);
                if let Some(ipdom_index) = result.immediate_dominators[index] {
                    if ipdom_index == virtual_root {
                        continue;
                    }

                    let ipdom_block = function.block(ipdom_index);
                    *immediate_postdominators.get_mut(block) = Some(ipdom_block);
                }
            }
        }

        let (preorder, preorder_max) =
            Self::compute_preorder(function.blocks(), &immediate_postdominators, entry);

        Self {
            immediate_postdominators,
            preorder,
            preorder_max,
        }
    }

    /// Return the immediate postdominator of one block.
    pub fn immediate_postdominator(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        *self.immediate_postdominators.get(block)
    }

    /// Return whether one block postdominates another.
    pub fn postdominates(&self, a: LocalNodeId<Block>, b: LocalNodeId<Block>) -> bool {
        if a == b {
            return true;
        }

        let Some(a_pre) = *self.preorder.get(a) else {
            return false;
        };
        let Some(b_pre) = *self.preorder.get(b) else {
            return false;
        };
        let Some(a_max) = *self.preorder_max.get(a) else {
            return false;
        };

        a_pre <= b_pre && a_max >= b_pre
    }

    /// Collect blocks with no successors.
    fn collect_exit_blocks(function: &Function, tree: &Tree) -> HashSet<LocalNodeId<Block>> {
        let mut exits = HashSet::new();

        // terminators with no successors
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            if terminator.successors(tree).is_empty() {
                exits.insert(block_id);
            }
        }

        exits
    }

    /// Compute preorder ranges for the postdominator forest.
    fn compute_preorder(
        blocks: &[LocalNodeId<Block>],
        immediate_postdominators: &NodeTable<Block, Option<LocalNodeId<Block>>>,
        entry: LocalNodeId<Block>,
    ) -> (NodeTable<Block, Option<u32>>, NodeTable<Block, Option<u32>>) {
        let mut children = NodeTable::from_nodes(blocks, Vec::new);

        // postdominator edges
        for &block in blocks {
            if let Some(ipdom) = *immediate_postdominators.get(block) {
                children.get_mut(ipdom).push(block);
            }
        }

        let mut roots: Vec<_> = blocks
            .iter()
            .copied()
            .filter(|block| immediate_postdominators.get(*block).is_none())
            .collect();
        roots.sort();

        // fully cyclic graphs still need one root for range numbering
        if roots.is_empty() {
            roots.push(entry);
        }

        let mut preorder = NodeTable::from_nodes(blocks, || None);
        let mut preorder_max = NodeTable::from_nodes(blocks, || None);
        let mut counter = 0u32;

        // forest walk
        for root in roots {
            if preorder.get(root).is_some() {
                continue;
            }

            Self::fill_preorder(
                root,
                &children,
                &mut preorder,
                &mut preorder_max,
                &mut counter,
            );
        }

        (preorder, preorder_max)
    }

    /// Fill preorder ranges for one postdominator subtree.
    fn fill_preorder(
        block: LocalNodeId<Block>,
        children: &NodeTable<Block, Vec<LocalNodeId<Block>>>,
        preorder: &mut NodeTable<Block, Option<u32>>,
        preorder_max: &mut NodeTable<Block, Option<u32>>,
        counter: &mut u32,
    ) {
        *counter += 1;
        *preorder.get_mut(block) = Some(*counter);

        let mut max = *counter;

        // subtree walk
        for &child in children.get(block) {
            Self::fill_preorder(child, children, preorder, preorder_max, counter);
            max = max.max(preorder_max.get(child).unwrap_or_else(|| {
                unreachable!("missing postdominator preorder max for child: {child:?}")
            }));
        }

        *preorder_max.get_mut(block) = Some(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::parse_test_function;

    #[test]
    fn test_build_postdominators_for_linear_flow() {
        let (tree, function_id) = parse_test_function(
            r#"
function linear(): void {
entry:
    jump b1

b1:
    jump b2

b2:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);
        let postdom = PostDominatorTree::build(function, &tree, &cfg);

        let block0 = function.block(0);
        let block1 = function.block(1);
        let block2 = function.block(2);

        assert_eq!(postdom.immediate_postdominator(block2), None);
        assert_eq!(postdom.immediate_postdominator(block1), Some(block2));
        assert_eq!(postdom.immediate_postdominator(block0), Some(block1));
        assert!(postdom.postdominates(block2, block0));
    }
}
