use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_mir as mir;

use crate::optimize::{Analysis, AnalysisKind, OptimizationContext};

use super::ControlFlowGraph;

/// Dominator tree for a function.
///
/// A block A dominates block B if every path from the entry to B goes through A.
/// The immediate dominator of B is the closest dominator (other than B itself).
#[derive(Debug)]
pub struct DominatorTree {
    /// Immediate dominator for each block. Entry block has no idom.
    immediate_dominators: HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    /// Preorder numbers for fast dominance queries.
    preorder: HashMap<mir::LocalNodeId<mir::Block>, u32>,
    /// Maximum preorder number in subtree for fast dominance queries.
    preorder_max: HashMap<mir::LocalNodeId<mir::Block>, u32>,
}

impl DominatorTree {
    /// Build a dominator tree using the simple iterative algorithm.
    ///
    /// This is O(n²) in the worst case but simple and correct.
    /// For production, consider Semi-NCA (what LLVM/Cranelift use).
    fn build(function: &mir::Function, _tree: &mir::NodeTree, cfg: &ControlFlowGraph) -> Self {
        let entry = match function.entry {
            Some(entry) => entry,
            None => {
                // imported function, no body
                return Self {
                    immediate_dominators: HashMap::new(),
                    preorder: HashMap::new(),
                    preorder_max: HashMap::new(),
                };
            }
        };

        // initialize dominators: entry dominates only itself, others dominate all
        let all_blocks: HashSet<mir::LocalNodeId<mir::Block>> =
            function.blocks.iter().copied().collect();
        let mut dominators: HashMap<
            mir::LocalNodeId<mir::Block>,
            HashSet<mir::LocalNodeId<mir::Block>>,
        > = HashMap::new();

        for &block in &function.blocks {
            if block == entry {
                let mut dom_set = HashSet::new();
                dom_set.insert(entry);
                dominators.insert(block, dom_set);
            } else {
                dominators.insert(block, all_blocks.clone());
            }
        }

        // iterate until fixed point
        let mut changed = true;
        while changed {
            changed = false;
            for &block in &function.blocks {
                if block == entry {
                    continue;
                }

                // dom(block) = {block} ∪ (∩ dom(pred) for pred in predecessors)
                let preds = cfg.predecessors(block);
                if preds.is_empty() {
                    continue;
                }

                let mut new_dom: HashSet<mir::LocalNodeId<mir::Block>> =
                    dominators.get(&preds[0]).cloned().unwrap_or_default();

                for &pred in &preds[1..] {
                    if let Some(pred_dom) = dominators.get(&pred) {
                        new_dom = new_dom.intersection(pred_dom).copied().collect();
                    }
                }
                new_dom.insert(block);

                if new_dom != *dominators.get(&block).unwrap() {
                    dominators.insert(block, new_dom);
                    changed = true;
                }
            }
        }

        // compute immediate dominators from dominator sets
        let mut immediate_dominators = HashMap::new();
        for &block in &function.blocks {
            if block == entry {
                continue;
            }

            let dom_set = dominators.get(&block).unwrap();

            // idom is the dominator closest to block (has largest dominator set minus block)
            let mut idom = None;
            let mut idom_size = 0;

            for &candidate in dom_set {
                if candidate == block {
                    continue;
                }
                let candidate_dom = dominators.get(&candidate).unwrap();
                if candidate_dom.len() > idom_size {
                    idom = Some(candidate);
                    idom_size = candidate_dom.len();
                }
            }

            if let Some(idom_block) = idom {
                immediate_dominators.insert(block, idom_block);
            }
        }

        // compute preorder numbers for O(1) dominance queries
        let (preorder, preorder_max) =
            Self::compute_preorder(&function.blocks, entry, &immediate_dominators);

        Self {
            immediate_dominators,
            preorder,
            preorder_max,
        }
    }

    /// Compute preorder numbers for the dominator tree.
    fn compute_preorder(
        blocks: &[mir::LocalNodeId<mir::Block>],
        entry: mir::LocalNodeId<mir::Block>,
        idoms: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    ) -> (
        HashMap<mir::LocalNodeId<mir::Block>, u32>,
        HashMap<mir::LocalNodeId<mir::Block>, u32>,
    ) {
        // build children lists from idom relation
        let mut children: HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> =
            HashMap::new();
        for &block in blocks {
            children.insert(block, Vec::new());
        }
        for (&block, &idom) in idoms {
            children.get_mut(&idom).unwrap().push(block);
        }

        // DFS to assign preorder numbers
        let mut preorder = HashMap::new();
        let mut preorder_max = HashMap::new();
        let mut counter = 0u32;

        fn dfs(
            block: mir::LocalNodeId<mir::Block>,
            children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
            preorder: &mut HashMap<mir::LocalNodeId<mir::Block>, u32>,
            preorder_max: &mut HashMap<mir::LocalNodeId<mir::Block>, u32>,
            counter: &mut u32,
        ) {
            *counter += 1;
            preorder.insert(block, *counter);

            let mut max = *counter;
            if let Some(block_children) = children.get(&block) {
                for &child in block_children {
                    dfs(child, children, preorder, preorder_max, counter);
                    max = max.max(*preorder_max.get(&child).unwrap());
                }
            }
            preorder_max.insert(block, max);
        }

        dfs(
            entry,
            &children,
            &mut preorder,
            &mut preorder_max,
            &mut counter,
        );

        (preorder, preorder_max)
    }

    /// Get the immediate dominator of a block.
    pub fn immediate_dominator(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Option<mir::LocalNodeId<mir::Block>> {
        self.immediate_dominators.get(&block).copied()
    }

    /// Check if block A dominates block B.
    ///
    /// Uses preorder numbers for O(1) queries.
    pub fn dominates(
        &self,
        a: mir::LocalNodeId<mir::Block>,
        b: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        // a dominates b iff a's preorder <= b's preorder <= a's preorder_max
        let a_pre = match self.preorder.get(&a) {
            Some(&n) => n,
            None => return false,
        };
        let a_max = match self.preorder_max.get(&a) {
            Some(&n) => n,
            None => return false,
        };
        let b_pre = match self.preorder.get(&b) {
            Some(&n) => n,
            None => return false,
        };

        a_pre <= b_pre && b_pre <= a_max
    }

    /// Check if block A strictly dominates block B (A dominates B and A != B).
    pub fn strictly_dominates(
        &self,
        a: mir::LocalNodeId<mir::Block>,
        b: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        a != b && self.dominates(a, b)
    }
}

impl Analysis for DominatorTree {
    const KIND: AnalysisKind = AnalysisKind::DominatorTree;

    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> Arc<Self> {
        // depends on CFG
        let cfg = context
            .analyses
            .get::<ControlFlowGraph>(function, tree, context);

        Arc::new(Self::build(function, tree, &cfg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_domtree_linear() {
        // linear: block0 -> block1 -> block2
        // block0 dominates all, block1 dominates block2
        let program = TestProgram::new(
            r#"function @linear() -> void {
block0:
    jump block1
block1:
    jump block2
block2:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let context = program.context();
        let domtree = context
            .analyses
            .get::<DominatorTree>(function, &program.tree, &context);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        // block0 dominates all blocks
        assert!(domtree.dominates(block0, block0));
        assert!(domtree.dominates(block0, block1));
        assert!(domtree.dominates(block0, block2));

        // block1 dominates block2
        assert!(domtree.dominates(block1, block2));

        // block2 doesn't dominate anything but itself
        assert!(!domtree.dominates(block2, block0));
        assert!(!domtree.dominates(block2, block1));

        // check immediate dominators
        assert_eq!(domtree.immediate_dominator(block1), Some(block0));
        assert_eq!(domtree.immediate_dominator(block2), Some(block1));
    }

    #[test]
    fn test_domtree_diamond() {
        // diamond: block0 -> block1, block0 -> block2, block1 -> block3, block2 -> block3
        // block0 dominates all, neither block1 nor block2 dominates block3
        let program = TestProgram::new(
            r#"function @diamond(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let context = program.context();
        let domtree = context
            .analyses
            .get::<DominatorTree>(function, &program.tree, &context);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];

        // block0 dominates all
        assert!(domtree.dominates(block0, block0));
        assert!(domtree.dominates(block0, block1));
        assert!(domtree.dominates(block0, block2));
        assert!(domtree.dominates(block0, block3));

        // block1 doesn't dominate block3 (can reach via block2)
        assert!(!domtree.dominates(block1, block3));

        // block2 doesn't dominate block3 (can reach via block1)
        assert!(!domtree.dominates(block2, block3));

        // block3's immediate dominator is block0
        assert_eq!(domtree.immediate_dominator(block3), Some(block0));
    }

    #[test]
    fn test_domtree_strictly_dominates() {
        let program = TestProgram::new(
            r#"function @simple() -> void {
block0:
    jump block1
block1:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let context = program.context();
        let domtree = context
            .analyses
            .get::<DominatorTree>(function, &program.tree, &context);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];

        // block0 dominates but doesn't strictly dominate itself
        assert!(domtree.dominates(block0, block0));
        assert!(!domtree.strictly_dominates(block0, block0));

        // block0 strictly dominates block1
        assert!(domtree.strictly_dominates(block0, block1));
    }
}
