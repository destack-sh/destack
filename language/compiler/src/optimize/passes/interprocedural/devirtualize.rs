use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext};
use mir::{DispatchAnalysis, Mutation};

declare_pass! {
    /// Rewrite statically proven virtual and dynamic calls to direct calls.
    #[pass(id = "devirtualize")]
    pub Devirtualize,
    "Devirtualize proven calls"
}

impl ModulePass for Devirtualize {
    /// Run definitive devirtualization.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::TreeAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;

        let dispatch = analyses.get::<DispatchAnalysis>(tree);
        let rewrites = Devirtualization::collect(tree, &dispatch);
        let changed = rewrites.apply(tree);

        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "Devirtualize"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "devirtualize"
    }
}

/// Definitive devirtualization rewrite set.
#[derive(Debug, Default)]
struct Devirtualization {
    /// Instruction replacements.
    instructions: Vec<(mir::LocalNodeId<mir::Instruction>, mir::Instruction)>,
    /// Terminator replacements keyed by owning block.
    terminators: Vec<(mir::BlockId, mir::Terminator)>,
}

impl Devirtualization {
    /// Collect all statically proven call rewrites.
    fn collect(tree: &mir::Tree, dispatch: &DispatchAnalysis) -> Self {
        let mut devirtualization = Self::default();

        // scan defined functions
        for (_, function) in tree.iter_nodes::<mir::Function>() {
            if function.is_defined() {
                devirtualization.collect_function(tree, dispatch, function);
            }
        }

        devirtualization
    }

    /// Apply every collected rewrite.
    fn apply(self, tree: &mut mir::Tree) -> bool {
        let changed = !self.instructions.is_empty() || !self.terminators.is_empty();

        // replace instruction calls
        for (instruction, replacement) in self.instructions {
            tree.set(instruction, replacement);
        }

        // replace terminator calls
        for (block, replacement) in self.terminators {
            let terminator = tree.get(block).terminator;
            tree.set(terminator, replacement);
        }

        changed
    }

    /// Collect rewrites in one function.
    fn collect_function(
        &mut self,
        tree: &mir::Tree,
        dispatch: &DispatchAnalysis,
        function: &mir::Function,
    ) {
        for &block in function.blocks() {
            self.collect_block(tree, dispatch, block);
        }
    }

    /// Collect rewrites in one block.
    fn collect_block(
        &mut self,
        tree: &mir::Tree,
        dispatch: &DispatchAnalysis,
        block: mir::BlockId,
    ) {
        let node = tree.get(block);

        // collect instruction calls
        for &instruction in &node.instructions {
            let callsite = mir::CallSite::Instruction(instruction);
            let Some(function) = dispatch.target(callsite) else {
                continue;
            };

            if let Some(replacement) = Self::direct_instruction(tree.get(instruction), function) {
                self.instructions.push((instruction, replacement));
            }
        }

        // collect terminator calls
        let callsite = mir::CallSite::Terminator(block);
        let Some(function) = dispatch.target(callsite) else {
            return;
        };

        if let Some(replacement) = Self::direct_terminator(tree.get(node.terminator), function) {
            self.terminators.push((block, replacement));
        }
    }

    /// Return a direct instruction call for a proven virtual or dynamic call.
    fn direct_instruction(
        instruction: &mir::Instruction,
        function: mir::FunctionId,
    ) -> Option<mir::Instruction> {
        match instruction {
            mir::Instruction::Call { destination, call }
                if matches!(
                    call.callee,
                    mir::Callee::Virtual { .. } | mir::Callee::Dynamic { .. }
                ) =>
            {
                Some(mir::Instruction::Call {
                    destination: *destination,
                    call: mir::Call {
                        callee: mir::Callee::Direct { function },
                        ..call.clone()
                    },
                })
            }
            _ => None,
        }
    }

    /// Return a direct terminator call for a proven virtual or dynamic call.
    fn direct_terminator(
        terminator: &mir::Terminator,
        function: mir::FunctionId,
    ) -> Option<mir::Terminator> {
        match terminator {
            mir::Terminator::Invoke {
                call,
                target,
                unwind,
                ..
            } if matches!(
                call.callee,
                mir::Callee::Virtual { .. } | mir::Callee::Dynamic { .. }
            ) =>
            {
                Some(mir::Terminator::Invoke {
                    call: mir::Call {
                        callee: mir::Callee::Direct { function },
                        ..call.clone()
                    },
                    target: target.clone(),
                    unwind: unwind.clone(),
                })
            }
            mir::Terminator::TailCall { call }
                if matches!(
                    call.callee,
                    mir::Callee::Virtual { .. } | mir::Callee::Dynamic { .. }
                ) =>
            {
                Some(mir::Terminator::TailCall {
                    call: mir::Call {
                        callee: mir::Callee::Direct { function },
                        ..call.clone()
                    },
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use destack_mir as mir;

    use super::Devirtualize;

    /// Virtual instruction calls become direct calls when the table proves a method.
    #[test]
    fn test_devirtualize_virtual_instruction_call() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#;
        let mut test = TestProgram::new(input);
        let callee = test.function_id_by_name("callee");
        let class = int32_type(&test);
        test.add_virtual_method_table(class, callee);

        test.run_module_pass(&Devirtualize);

        test.assert_output(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call callee(v0): (int32) => int32
    return v1
}
"#,
        );
    }

    /// Virtual invokes become direct invokes.
    #[test]
    fn test_devirtualize_virtual_invoke() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    invoke.virtual v0, int32, 0(v0): (int32) => int32 => b1 | cleanup
b1(v1: int32):
    return v1

cleanup:
    unwind.resume
}
"#;
        let mut test = TestProgram::new(input);
        let callee = test.function_id_by_name("callee");
        let class = int32_type(&test);
        test.add_virtual_method_table(class, callee);

        test.run_module_pass(&Devirtualize);

        test.assert_output(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    invoke callee(v0): (int32) => int32 => b1 | cleanup

b1(v1: int32):
    return v1

cleanup:
    unwind.resume
}
"#,
        );
    }

    /// Dynamic tail calls become direct tail calls when the dynamic table proves a method.
    #[test]
    fn test_devirtualize_dynamic_tail_call() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    tail.call.dynamic v0, int32, 0(v0): (int32) => int32
}
"#;
        let mut test = TestProgram::new(input);
        let callee = test.function_id_by_name("callee");
        let constraint = int32_type(&test);
        test.add_dynamic_method_table(constraint, constraint, callee);

        test.run_module_pass(&Devirtualize);

        test.assert_output(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    tail.call callee(v0): (int32) => int32
}
"#,
        );
    }

    /// Open dispatch stays unchanged without table proof.
    #[test]
    fn test_devirtualize_preserves_open_call() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#;
        let mut test = TestProgram::new(input);

        test.run_module_pass(&Devirtualize);

        test.assert_unchanged(input);
    }

    /// Return the canonical int32 type id.
    fn int32_type(test: &TestProgram) -> mir::TypeId {
        test.optimized
            .tree
            .iter_nodes::<mir::Type>()
            .find(|(_, ty)| **ty == mir::Type::INT32)
            .expect("missing int32 type")
            .0
    }
}
