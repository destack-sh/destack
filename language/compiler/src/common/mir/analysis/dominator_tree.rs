use destack_mir as mir;

use crate::common::mir::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

use super::ControlFlowGraph;

/// Dominator tree for a function.
#[derive(Debug, Clone)]
pub struct DominatorTree {
    /// The shared MIR-level dominator tree.
    tree: mir::DominatorTree,
}

impl DominatorTree {
    /// Return the immediate dominator of one block.
    pub fn immediate_dominator(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Option<mir::LocalNodeId<mir::Block>> {
        self.tree.immediate_dominator(block)
    }

    /// Return whether one block dominates another.
    pub fn dominates(
        &self,
        a: mir::LocalNodeId<mir::Block>,
        b: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        self.tree.dominates(a, b)
    }

    /// Return whether one block strictly dominates another.
    pub fn strictly_dominates(
        &self,
        a: mir::LocalNodeId<mir::Block>,
        b: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        self.tree.strictly_dominates(a, b)
    }
}

impl Analysis for DominatorTree {
    const ID: AnalysisId = AnalysisId("domtree");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for DominatorTree {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();

        Self {
            tree: mir::DominatorTree::build(function, tree, &cfg.graph),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_compute_dominators_through_function_analyses() {
        let test = TestProgram::new(
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

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let domtree = analyses.get::<DominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert!(domtree.dominates(block0, block0));
        assert!(domtree.dominates(block0, block1));
        assert!(domtree.dominates(block0, block2));
        assert!(domtree.dominates(block1, block2));
        assert!(!domtree.dominates(block2, block0));
        assert!(!domtree.dominates(block2, block1));
        assert_eq!(domtree.immediate_dominator(block1), Some(block0));
        assert_eq!(domtree.immediate_dominator(block2), Some(block1));
    }
}
