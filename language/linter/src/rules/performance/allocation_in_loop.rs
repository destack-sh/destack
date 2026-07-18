use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};
use destack_mir as mir;

declare_lint! {
    /// Warn when an allocating operation executes on a repeated path.
    pub ALLOCATION_IN_LOOP {
        id: "allocation-in-loop",
        code: "LP057",
        description: "Warn when an allocating operation executes on a repeated path",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check allocation-in-loop.
fn check(mut context: MirModuleContext<'_>) -> Result<(), LinterError> {
    let mir = context.module.mir.as_ref();

    // inspect only operations in natural loop bodies
    for (function_id, function) in mir.tree.iter_nodes::<mir::Function>() {
        let loops = context
            .analyses
            .get_function::<mir::LoopAnalysis>(function_id, &mir.tree);
        if loops.num_loops() == 0 {
            continue;
        }

        for block_id in function.blocks() {
            if !loops.is_in_loop(*block_id) {
                continue;
            }

            let block = mir.tree.get(*block_id);
            for instruction_id in &block.instructions {
                let instruction = mir.tree.get(*instruction_id);
                if !instruction_allocates(*instruction_id, instruction, mir)? {
                    continue;
                }

                report_allocation(&mut context, instruction_id.into_any());
            }

            let terminator = mir.tree.get(block.terminator);
            if terminator_allocates(*block_id, terminator, mir)? {
                report_allocation(&mut context, block.terminator.into_any());
            }
        }
    }

    Ok(())
}

/// Return whether one MIR instruction may allocate heap storage.
fn instruction_allocates(
    id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    module: &destack_artifact::MirLowered,
) -> Result<bool, LinterError> {
    match instruction {
        mir::Instruction::NewZeroed { .. }
        | mir::Instruction::NewUninit { .. }
        | mir::Instruction::NewSliceZeroed { .. }
        | mir::Instruction::NewSliceUninit { .. } => Ok(true),
        mir::Instruction::Call { .. } => {
            let callsite = mir::CallSite::Instruction(id);
            let effect = module
                .effects
                .call(callsite)
                .ok_or_else(|| LinterError::Internal {
                    message: format!("verified MIR call {id:?} has no effect entry"),
                })?;

            Ok(effect.behavior.allocates)
        }
        _ => Ok(false),
    }
}

/// Return whether one MIR terminator may allocate heap storage.
fn terminator_allocates(
    block: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
    module: &destack_artifact::MirLowered,
) -> Result<bool, LinterError> {
    match terminator {
        mir::Terminator::NewZeroedTry { .. }
        | mir::Terminator::NewUninitTry { .. }
        | mir::Terminator::NewSliceZeroedTry { .. }
        | mir::Terminator::NewSliceUninitTry { .. } => Ok(true),
        mir::Terminator::Invoke { .. } | mir::Terminator::TailCall { .. } => {
            let callsite = mir::CallSite::Terminator(block);
            let effect = module
                .effects
                .call(callsite)
                .ok_or_else(|| LinterError::Internal {
                    message: format!("verified MIR call terminator {block:?} has no effect entry"),
                })?;

            Ok(effect.behavior.allocates)
        }
        _ => Ok(false),
    }
}

/// Report one allocating operation inside a loop.
fn report_allocation(context: &mut MirModuleContext<'_>, node: mir::LocalNodeIdAny) {
    let anchor = context.module.anchor(node);
    let diagnostic = context
        .diagnostic("heap allocation executes inside a loop", anchor)
        .label("this allocation repeats with the loop");
    context.report(diagnostic);
}

#[cfg(test)]
mod tests {
    use destack_artifact::MirLowered;

    use super::*;

    /// Classify explicit heap allocation instructions without source heuristics.
    #[test]
    fn test_classify_explicit_heap_allocation() {
        let module = MirLowered::new();
        let type_id = mir::LocalNodeId::<mir::Type>::new(0);
        let instruction_id = mir::LocalNodeId::<mir::Instruction>::new(0);
        let instruction = mir::Instruction::NewZeroed {
            destination: mir::Value::new(0),
            layout: type_id,
            result_type: type_id,
        };

        let actual = instruction_allocates(instruction_id, &instruction, &module).unwrap();

        assert!(actual);
    }

    /// Reject verified calls whose mandatory effect entry is absent.
    #[test]
    fn test_require_call_effect() {
        let module = MirLowered::new();
        let function_id = mir::LocalNodeId::<mir::Function>::new(0);
        let type_id = mir::LocalNodeId::<mir::Type>::new(0);
        let instruction_id = mir::LocalNodeId::<mir::Instruction>::new(0);
        let call = mir::Call::new(
            mir::Callee::Direct {
                function: function_id,
            },
            mir::ValueSlice::default(),
            type_id,
        );
        let instruction = mir::Instruction::Call {
            destination: None,
            call,
        };

        let actual = instruction_allocates(instruction_id, &instruction, &module);
        let Err(LinterError::Internal { message }) = actual else {
            panic!("missing call effect should fail with an internal linter error");
        };

        assert_eq!(
            message,
            "verified MIR call LocalNodeId { id: 0 } has no effect entry"
        );
    }
}
