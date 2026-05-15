use std::collections::HashMap;

use destack_mir as mir;

use crate::common::mir::{Analysis, AnalysisId, CallGraph, ModuleAnalyses, ModuleAnalysis};

use super::call_graph::CallSiteRef;

/// Call target resolution for module-local callsites.
#[derive(Debug)]
pub struct CallTargetAnalysis {
    /// Resolved targets for instruction callsites.
    instruction_targets:
        HashMap<mir::LocalNodeId<mir::Instruction>, Vec<mir::LocalNodeId<mir::Function>>>,
}

impl CallTargetAnalysis {
    /// Return resolved targets for an instruction callsite.
    pub fn targets_for_instruction(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<&[mir::LocalNodeId<mir::Function>]> {
        self.instruction_targets
            .get(&instruction_id)
            .map(Vec::as_slice)
    }

    /// Build call target data from a call graph.
    fn build(tree: &mir::Tree, call_graph: &CallGraph) -> Self {
        let mut instruction_targets: HashMap<
            mir::LocalNodeId<mir::Instruction>,
            Vec<mir::LocalNodeId<mir::Function>>,
        > = HashMap::new();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            if function.entry.is_none() {
                continue;
            }

            for edge in call_graph.outgoing(function_id) {
                if !edge.is_direct() {
                    continue;
                }

                if let CallSiteRef::Instruction(instruction_id) = edge.callsite {
                    instruction_targets
                        .entry(instruction_id)
                        .or_default()
                        .push(edge.callee);
                }
            }
        }

        for targets in instruction_targets.values_mut() {
            targets.sort();
            targets.dedup();
        }

        Self {
            instruction_targets,
        }
    }
}

impl Analysis for CallTargetAnalysis {
    const ID: AnalysisId = AnalysisId("call-targets");
    const DEPENDENCIES: &'static [AnalysisId] = &[CallGraph::ID];
}

impl ModuleAnalysis for CallTargetAnalysis {
    fn compute(tree: &mir::Tree, analyses: &ModuleAnalyses<'_>) -> Self {
        let call_graph = analyses.get::<CallGraph>();
        Self::build(tree, call_graph.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use destack_mir as mir;

    use crate::optimize::ModuleAnalyses;
    use crate::optimize::common::tests::TestProgram;

    use super::CallTargetAnalysis;

    /// Find the first instruction in a function that matches a predicate.
    fn find_instruction_id<F>(
        tree: &mir::Tree,
        function_id: mir::LocalNodeId<mir::Function>,
        predicate: F,
    ) -> mir::LocalNodeId<mir::Instruction>
    where
        F: Fn(&mir::Instruction) -> bool,
    {
        let function = tree.get(function_id);

        // scan blocks for a matching instruction
        for block_id in &function.blocks {
            let block = tree.get(*block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                if predicate(instruction) {
                    return instruction_id;
                }
            }
        }

        panic!("missing instruction");
    }

    /// Direct calls resolve to a single target.
    #[test]
    fn test_call_targets_direct_call() {
        let test = TestProgram::new(
            r#"
function callee(): int32 {
b0:
    v0: int32 = 7int32
    return v0
}
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");
        let call_id = find_instruction_id(&test.tree, test_id, |inst| {
            matches!(inst, mir::Instruction::Call { .. })
        });

        let analyses = ModuleAnalyses::new(&test.tree);
        let call_targets = analyses.get::<CallTargetAnalysis>();
        let targets = call_targets
            .targets_for_instruction(call_id)
            .expect("missing call targets");

        assert_eq!(targets, &[callee_id]);
    }

    /// Class calls remain unresolved without devirtualization data.
    #[test]
    fn test_call_targets_virtual_declared() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call.class v0, int32, 1(v0): (int32) -> int32
    return v1
}"#,
        );

        let test_id = test.function_id_by_name("test");
        let call_id = find_instruction_id(&test.tree, test_id, |inst| {
            matches!(inst, mir::Instruction::CallClass { .. })
        });

        let analyses = ModuleAnalyses::new(&test.tree);
        let call_targets = analyses.get::<CallTargetAnalysis>();

        assert!(call_targets.targets_for_instruction(call_id).is_none());
    }

    /// Indirect calls without declared targets remain unresolved.
    #[test]
    fn test_call_targets_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: (int32) -> int32, v1: int32): int32  {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
        );

        let test_id = test.function_id_by_name("test");
        let call_id = find_instruction_id(&test.tree, test_id, |inst| {
            matches!(inst, mir::Instruction::CallIndirect { .. })
        });

        let analyses = ModuleAnalyses::new(&test.tree);
        let call_targets = analyses.get::<CallTargetAnalysis>();

        assert!(call_targets.targets_for_instruction(call_id).is_none());
    }
}
