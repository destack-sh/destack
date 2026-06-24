use std::collections::{HashMap, HashSet};

use crate::{Block, ControlFlowGraph, Function, LocalNodeId, Tree};

/// Postdominator tree for one function.
#[derive(Debug, Clone)]
pub struct PostDominatorTree {
    /// Immediate postdominator indexed by block id.
    immediate_postdominators: Vec<Option<LocalNodeId<Block>>>,
    /// Preorder numbers indexed by block id.
    preorder: Vec<Option<u32>>,
    /// Maximum preorder number in each subtree indexed by block id.
    preorder_max: Vec<Option<u32>>,
}

impl PostDominatorTree {
    /// Build the postdominator tree for one function.
    pub fn build(function: &Function, tree: &Tree, cfg: &ControlFlowGraph) -> Self {
        let Some(entry) = function.entry else {
            return Self {
                immediate_postdominators: Vec::new(),
                preorder: Vec::new(),
                preorder_max: Vec::new(),
            };
        };

        // exit blocks
        let mut exit_blocks = Self::collect_exit_blocks(function, tree);
        let has_exits = !exit_blocks.is_empty();

        if !has_exits {
            exit_blocks = function.blocks.iter().copied().collect();
        }

        let mut block_index = HashMap::new();

        // block indices
        for (index, &block) in function.blocks.iter().enumerate() {
            block_index.insert(block, index);
        }

        let virtual_root = function.blocks.len();
        let mut successors = vec![Vec::new(); function.blocks.len() + 1];
        let mut predecessors = vec![Vec::new(); function.blocks.len() + 1];

        // reverse edges
        for &block in &function.blocks {
            let index = block_index[&block];

            for predecessor in cfg.predecessors(block) {
                let predecessor_index = block_index[predecessor];
                successors[index].push(predecessor_index);
                predecessors[predecessor_index].push(index);
            }
        }

        // connect exits to the virtual root
        for exit_block in exit_blocks {
            let exit_index = block_index[&exit_block];
            successors[virtual_root].push(exit_index);
            predecessors[exit_index].push(virtual_root);
        }

        let block_count = function.block_capacity();
        let mut immediate_postdominators = vec![None; block_count];

        // postdominator edges
        if has_exits {
            let result = super::dominator_tree::DominatorComputation::compute(
                &successors,
                &predecessors,
                virtual_root,
            );

            for (&block, &index) in &block_index {
                if let Some(ipdom_index) = result.immediate_dominators[index] {
                    if ipdom_index == virtual_root {
                        continue;
                    }

                    let ipdom_block = function.blocks[ipdom_index];
                    immediate_postdominators[block.id as usize] = Some(ipdom_block);
                }
            }
        }

        let (preorder, preorder_max) =
            Self::compute_preorder(&function.blocks, &immediate_postdominators, entry);

        Self {
            immediate_postdominators,
            preorder,
            preorder_max,
        }
    }

    /// Return the immediate postdominator of one block.
    pub fn immediate_postdominator(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        self.immediate_postdominators
            .get(block.id as usize)
            .copied()
            .flatten()
    }

    /// Return whether one block postdominates another.
    pub fn postdominates(&self, a: LocalNodeId<Block>, b: LocalNodeId<Block>) -> bool {
        if a == b {
            return true;
        }

        let Some(a_pre) = self.preorder.get(a.id as usize).copied().flatten() else {
            return false;
        };
        let Some(b_pre) = self.preorder.get(b.id as usize).copied().flatten() else {
            return false;
        };
        let Some(a_max) = self.preorder_max.get(a.id as usize).copied().flatten() else {
            return false;
        };

        a_pre <= b_pre && a_max >= b_pre
    }

    /// Collect blocks with no successors.
    fn collect_exit_blocks(function: &Function, tree: &Tree) -> HashSet<LocalNodeId<Block>> {
        let mut exits = HashSet::new();

        // terminators with no successors
        for &block_id in &function.blocks {
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
        immediate_postdominators: &[Option<LocalNodeId<Block>>],
        entry: LocalNodeId<Block>,
    ) -> (Vec<Option<u32>>, Vec<Option<u32>>) {
        let block_count = blocks
            .iter()
            .map(|block| block.id as usize + 1)
            .max()
            .unwrap_or(0);
        let mut children = vec![Vec::new(); block_count];

        // postdominator edges
        for &block in blocks {
            if let Some(ipdom) = immediate_postdominators[block.id as usize] {
                children[ipdom.id as usize].push(block);
            }
        }

        let mut roots: Vec<_> = blocks
            .iter()
            .copied()
            .filter(|block| immediate_postdominators[block.id as usize].is_none())
            .collect();
        roots.sort();

        // fully cyclic graphs still need one root for range numbering
        if roots.is_empty() {
            roots.push(entry);
        }

        let mut preorder = vec![None; block_count];
        let mut preorder_max = vec![None; block_count];
        let mut counter = 0u32;

        // forest walk
        for root in roots {
            if preorder[root.id as usize].is_some() {
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
        children: &[Vec<LocalNodeId<Block>>],
        preorder: &mut [Option<u32>],
        preorder_max: &mut [Option<u32>],
        counter: &mut u32,
    ) {
        *counter += 1;
        preorder[block.id as usize] = Some(*counter);

        let mut max = *counter;

        // subtree walk
        for &child in &children[block.id as usize] {
            Self::fill_preorder(child, children, preorder, preorder_max, counter);
            max = max.max(preorder_max[child.id as usize].unwrap_or_else(|| {
                unreachable!("missing postdominator preorder max for child: {child:?}")
            }));
        }

        preorder_max[block.id as usize] = Some(max);
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

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert_eq!(postdom.immediate_postdominator(block2), None);
        assert_eq!(postdom.immediate_postdominator(block1), Some(block2));
        assert_eq!(postdom.immediate_postdominator(block0), Some(block1));
        assert!(postdom.postdominates(block2, block0));
    }
}
