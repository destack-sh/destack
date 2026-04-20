use std::collections::{HashMap, HashSet};

use crate::{Block, ControlFlowGraph, Function, LocalNodeId, NodeTree};

/// Postdominator tree for one function.
#[derive(Debug, Clone)]
pub struct PostDominatorTree {
    /// Immediate postdominator for each block.
    immediate_postdominators: HashMap<LocalNodeId<Block>, LocalNodeId<Block>>,
    /// Preorder numbers for fast postdominance queries.
    preorder: HashMap<LocalNodeId<Block>, u32>,
    /// Maximum preorder number in each subtree.
    preorder_max: HashMap<LocalNodeId<Block>, u32>,
}

impl PostDominatorTree {
    /// Build the postdominator tree for one function.
    pub fn build(function: &Function, tree: &NodeTree, cfg: &ControlFlowGraph) -> Self {
        let Some(entry) = function.entry else {
            return Self {
                immediate_postdominators: HashMap::new(),
                preorder: HashMap::new(),
                preorder_max: HashMap::new(),
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

        let mut immediate_postdominators = HashMap::new();

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
                    immediate_postdominators.insert(block, ipdom_block);
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
        self.immediate_postdominators.get(&block).copied()
    }

    /// Return whether one block postdominates another.
    pub fn postdominates(&self, a: LocalNodeId<Block>, b: LocalNodeId<Block>) -> bool {
        if a == b {
            return true;
        }

        let Some(a_pre) = self.preorder.get(&a).copied() else {
            return false;
        };
        let Some(b_pre) = self.preorder.get(&b).copied() else {
            return false;
        };
        let Some(a_max) = self.preorder_max.get(&a).copied() else {
            return false;
        };

        a_pre <= b_pre && a_max >= b_pre
    }

    /// Collect blocks with no successors.
    fn collect_exit_blocks(function: &Function, tree: &NodeTree) -> HashSet<LocalNodeId<Block>> {
        let mut exits = HashSet::new();

        // terminators with no successors
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            if terminator.successors().is_empty() {
                exits.insert(block_id);
            }
        }

        exits
    }

    /// Compute preorder ranges for the postdominator forest.
    fn compute_preorder(
        blocks: &[LocalNodeId<Block>],
        immediate_postdominators: &HashMap<LocalNodeId<Block>, LocalNodeId<Block>>,
        entry: LocalNodeId<Block>,
    ) -> (
        HashMap<LocalNodeId<Block>, u32>,
        HashMap<LocalNodeId<Block>, u32>,
    ) {
        let mut children = HashMap::new();

        // child lists
        for &block in blocks {
            children.insert(block, Vec::new());
        }

        // postdominator edges
        for (&block, &ipdom) in immediate_postdominators {
            children
                .get_mut(&ipdom)
                .unwrap_or_else(|| panic!("missing postdominator children for block: {ipdom:?}"))
                .push(block);
        }

        let mut roots: Vec<_> = blocks
            .iter()
            .copied()
            .filter(|block| !immediate_postdominators.contains_key(block))
            .collect();
        roots.sort();

        // fully cyclic graphs still need one root for range numbering
        if roots.is_empty() {
            roots.push(entry);
        }

        let mut preorder = HashMap::new();
        let mut preorder_max = HashMap::new();
        let mut counter = 0u32;

        // forest walk
        for root in roots {
            if preorder.contains_key(&root) {
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
        children: &HashMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
        preorder: &mut HashMap<LocalNodeId<Block>, u32>,
        preorder_max: &mut HashMap<LocalNodeId<Block>, u32>,
        counter: &mut u32,
    ) {
        *counter += 1;
        preorder.insert(block, *counter);

        let mut max = *counter;

        // subtree walk
        if let Some(block_children) = children.get(&block) {
            for &child in block_children {
                Self::fill_preorder(child, children, preorder, preorder_max, counter);
                max = max.max(*preorder_max.get(&child).unwrap_or_else(|| {
                    panic!("missing postdominator preorder max for child: {child:?}")
                }));
            }
        }

        preorder_max.insert(block, max);
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
b0:
    jump b1
b1:
    jump b2
b2:
    return
}"#,
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
