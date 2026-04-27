use std::collections::HashSet;

use destack_mir as mir;

use crate::optimize::{Analysis, AnalysisId, ControlFlowGraph, FunctionAnalyses, FunctionAnalysis};

/// Liveness analysis for SSA values.
#[derive(Debug, Clone)]
pub struct LivenessAnalysis {
    /// The shared MIR-level liveness state.
    liveness: mir::FunctionLiveness,
}

impl LivenessAnalysis {
    /// Get values live at entry to a block.
    pub fn live_in(&self, block: mir::LocalNodeId<mir::Block>) -> &HashSet<mir::Value> {
        self.liveness.value_live_in(block)
    }

    /// Get values live at exit of a block.
    pub fn live_out(&self, block: mir::LocalNodeId<mir::Block>) -> &HashSet<mir::Value> {
        self.liveness.value_live_out(block)
    }

    /// Check if a value is live at block entry.
    pub fn is_live_in(&self, block: mir::LocalNodeId<mir::Block>, value: mir::Value) -> bool {
        self.liveness.is_value_live_in(block, value)
    }

    /// Check if a value is live at block exit.
    pub fn is_live_out(&self, block: mir::LocalNodeId<mir::Block>, value: mir::Value) -> bool {
        self.liveness.is_value_live_out(block, value)
    }

    /// Check if a value is live after a specific instruction.
    pub fn is_live_after_instruction(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        value: mir::Value,
        tree: &mir::Tree,
    ) -> bool {
        self.liveness
            .is_value_live_after_instruction(block_id, instruction_index, value, tree)
    }

    /// Get all values that are live at some point in the function.
    pub fn all_live_values(&self) -> HashSet<mir::Value> {
        self.liveness.all_live_values()
    }
}

impl Analysis for LivenessAnalysis {
    const ID: AnalysisId = AnalysisId("liveness");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for LivenessAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let _cfg = analyses.get::<ControlFlowGraph>();

        Self {
            liveness: mir::FunctionLiveness::build(function, tree),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_compute_liveness_through_function_analyses() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = int.add v0, v1
    return v2
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        assert!(!liveness.is_live_in(entry, mir::Value::new(0)));
        assert!(!liveness.is_live_in(entry, mir::Value::new(1)));
        assert!(!liveness.is_live_in(entry, mir::Value::new(2)));

        assert!(liveness.live_out(entry).is_empty());
        assert!(liveness.is_live_after_instruction(entry, 0, mir::Value::new(0), &test.tree));
        assert!(liveness.is_live_after_instruction(entry, 2, mir::Value::new(2), &test.tree));
    }
}
