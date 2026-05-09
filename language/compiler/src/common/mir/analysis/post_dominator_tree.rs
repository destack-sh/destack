use destack_mir as mir;

use crate::common::mir::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

use super::ControlFlowGraph;

/// Postdominator tree for a function.
#[derive(Debug, Clone)]
pub struct PostDominatorTree {
    /// The shared MIR-level postdominator tree.
    tree: mir::PostDominatorTree,
}

impl PostDominatorTree {
    /// Return the immediate postdominator of one block.
    pub fn immediate_postdominator(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Option<mir::LocalNodeId<mir::Block>> {
        self.tree.immediate_postdominator(block)
    }

    /// Return whether one block postdominates another.
    pub fn postdominates(
        &self,
        a: mir::LocalNodeId<mir::Block>,
        b: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        self.tree.postdominates(a, b)
    }
}

impl Analysis for PostDominatorTree {
    const ID: AnalysisId = AnalysisId("postdomtree");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for PostDominatorTree {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();

        Self {
            tree: mir::PostDominatorTree::build(function, tree, &cfg.graph),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_compute_postdominators_through_function_analyses() {
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
        let postdom = analyses.get::<PostDominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert_eq!(postdom.immediate_postdominator(block2), None);
        assert_eq!(postdom.immediate_postdominator(block1), Some(block2));
        assert_eq!(postdom.immediate_postdominator(block0), Some(block1));
        assert!(postdom.postdominates(block2, block0));
    }
}
