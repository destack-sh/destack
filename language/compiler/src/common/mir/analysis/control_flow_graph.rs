use destack_mir as mir;

use crate::common::mir::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

/// Control flow graph for a function.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// The shared MIR-level control flow graph.
    pub(super) graph: mir::ControlFlowGraph,
}

impl ControlFlowGraph {
    /// Build the control flow graph for one function.
    pub(crate) fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        Self {
            graph: mir::ControlFlowGraph::build(function, tree),
        }
    }

    /// Return the predecessors of one block.
    pub fn predecessors(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> &[mir::LocalNodeId<mir::Block>] {
        self.graph.predecessors(block)
    }

    /// Return whether one block is reachable from the entry block.
    pub fn is_reachable(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        entry: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        self.graph.is_reachable(block, entry)
    }
}

impl Analysis for ControlFlowGraph {
    const ID: AnalysisId = AnalysisId("cfg");
    const DEPENDENCIES: &'static [AnalysisId] = &[];
}

impl FunctionAnalysis for ControlFlowGraph {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        Self::build(function, tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_compute_cfg_through_function_analyses() {
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
        let cfg = analyses.get::<ControlFlowGraph>();

        let block0 = function.entry.unwrap();
        let block1 = function.blocks[1];

        assert!(cfg.predecessors(block0).is_empty());
        assert_eq!(cfg.predecessors(block1).len(), 1);
    }
}
