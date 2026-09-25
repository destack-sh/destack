use tspp_core::FxIndexSet;
use tspp_mir as mir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintOutput, LintResult, MirModule};

declare_lint! {
    /// Require a processor hint in atomic busy-wait loops.
    pub MISSING_SPIN_LOOP {
        id: "missing-spin-loop",
        summary: "Require a processor hint in atomic busy-wait loops",
        explanation: r#"
An atomic polling loop that performs no other work repeatedly reloads the atomic value without informing the processor that it is spinning.
Instead, you SHOULD call `spinLoop` during bounded optimistic spinning or use a blocking synchronization operation for longer waits.
"#,
        example: {
            reported: r#"
import { Atomic, MemoryOrdering } from "tspp:sync";

function wait(ready: &readonly Atomic<boolean>): void {
    while (!ready.load(MemoryOrdering.Acquire)) {}
}
"#,
            accepted: r#"
import { spinLoop } from "tspp:hint";
import { Atomic, MemoryOrdering } from "tspp:sync";

function wait(ready: &readonly Atomic<boolean>): void {
    while (!ready.load(MemoryOrdering.Acquire)) {
        spinLoop();
    }
}
"#,
        },
        provenance: [Clippy("missing_spin_loop")],
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Report atomic polling loops without a processor hint.
fn check(module: &mut MirModule<'_>, lint: &Lint) -> LintResult {
    let tree = &module.lowered.tree;
    let mut output = LintOutput::default();

    // inspect every natural loop in defined functions
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        let Some(entry) = function.entry() else {
            continue;
        };
        let loops = module.analyses.loops(function_id, tree);
        let definitions = module.analyses.definition(function_id, tree);
        let dominators = module.analyses.dominator(function_id, tree);

        for natural_loop in loops.loops() {
            let Some(poll) =
                find_atomic_poll(natural_loop, entry, &loops, &definitions, &dominators, tree)?
            else {
                continue;
            };

            // report the atomic poll
            let anchor = module.anchor(poll.into_any())?;
            let diagnostic = lint
                .diagnostic("atomic busy-wait loop has no processor hint", anchor)
                .help("call spinLoop() during bounded spinning or use a blocking wait");
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return the atomic poll controlling one unhinted empty loop.
fn find_atomic_poll(
    natural_loop: &mir::Loop,
    entry: mir::LocalNodeId<mir::Block>,
    loops: &mir::LoopTable,
    definitions: &mir::DefinitionTable,
    dominators: &mir::DominatorTable,
    tree: &mir::Tree,
) -> Result<Option<mir::LocalNodeId<mir::Instruction>>, ProviderError> {
    // treat a nested loop as substantive loop work
    for block in natural_loop.blocks.iter().copied() {
        let owner = loops.innermost_loop(block).ok_or_else(|| {
            ProviderError::internal(format!("loop block {block:?} has no natural-loop owner"))
        })?;
        if owner.header != natural_loop.header {
            return Ok(None);
        }
    }

    let mut hint_blocks = Vec::new();

    // reject loops that perform observable work
    for block_id in natural_loop.blocks.iter().copied() {
        let block = tree.get(block_id);

        for instruction_id in block.instructions.iter().copied() {
            let instruction = tree.get(instruction_id);
            match instruction {
                mir::Instruction::Error => {
                    return Err(ProviderError::internal(
                        "MIR error instruction reached missing-spin-loop",
                    ));
                }
                mir::Instruction::AtomicLoad { .. }
                | mir::Instruction::AtomicCompareExchange { .. } => {}
                mir::Instruction::Intrinsic {
                    intrinsic: mir::Intrinsic::SpinLoop,
                    ..
                } => hint_blocks.push(block_id),
                _ if instruction.has_side_effects() => return Ok(None),
                _ => {}
            }
        }

        let terminator = tree.get(block.terminator);
        if matches!(terminator, mir::Terminator::Error) {
            return Err(ProviderError::internal(
                "MIR error terminator reached missing-spin-loop",
            ));
        }
    }

    // accept only when every backedge executes a processor hint
    let has_hint_on_every_backedge = natural_loop.latches.iter().copied().all(|latch| {
        hint_blocks
            .iter()
            .copied()
            .any(|hint| dominators.dominates(hint, latch))
    });
    if has_hint_on_every_backedge {
        return Ok(None);
    }

    // find the first atomic value controlling an exit from this loop
    for block_id in natural_loop.exiting_blocks.iter().copied() {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let mut visited = FxIndexSet::default();
        let poll = match terminator {
            mir::Terminator::Branch { condition, .. } => find_atomic_definition(
                *condition,
                natural_loop,
                entry,
                definitions,
                tree,
                &mut visited,
            )?,
            mir::Terminator::Check { constraint, .. } => {
                let mut poll = None;
                for value in constraint.uses() {
                    poll = find_atomic_definition(
                        value,
                        natural_loop,
                        entry,
                        definitions,
                        tree,
                        &mut visited,
                    )?;
                    if poll.is_some() {
                        break;
                    }
                }

                poll
            }
            mir::Terminator::Switch { value, .. }
            | mir::Terminator::VariantSwitch { value, .. } => find_atomic_definition(
                *value,
                natural_loop,
                entry,
                definitions,
                tree,
                &mut visited,
            )?,
            _ => None,
        };
        if poll.is_some() {
            return Ok(poll);
        }
    }

    Ok(None)
}

/// Return the atomic operation that contributes to one SSA value.
fn find_atomic_definition(
    value: mir::Value,
    natural_loop: &mir::Loop,
    entry: mir::LocalNodeId<mir::Block>,
    definitions: &mir::DefinitionTable,
    tree: &mir::Tree,
    visited: &mut FxIndexSet<mir::Value>,
) -> Result<Option<mir::LocalNodeId<mir::Instruction>>, ProviderError> {
    if !visited.insert(value) {
        return Ok(None);
    }

    // follow values merged at block parameters
    let definition = definitions.definition(value).ok_or_else(|| {
        ProviderError::internal(format!(
            "MIR value {value:?} has no definition in missing-spin-loop"
        ))
    })?;
    let (block, instruction_id) = match definition {
        mir::ValueDefinition::FunctionParameter(_) => return Ok(None),
        mir::ValueDefinition::BlockParameter { block, .. } => {
            if block == entry {
                return Ok(None);
            }
            let inputs = definitions.inputs(value);
            if inputs.is_empty() {
                return Err(ProviderError::internal(format!(
                    "MIR block parameter {value:?} has no incoming values"
                )));
            }

            for value in inputs.iter().filter_map(|input| input.argument) {
                let poll =
                    find_atomic_definition(value, natural_loop, entry, definitions, tree, visited)?;
                if poll.is_some() {
                    return Ok(poll);
                }
            }

            return Ok(None);
        }
        mir::ValueDefinition::Instruction { block, instruction } => (block, instruction),
    };

    // recognize atomic definitions inside this loop
    let instruction = tree.get(instruction_id);
    match instruction {
        mir::Instruction::Error => {
            return Err(ProviderError::internal(
                "MIR error instruction reached missing-spin-loop",
            ));
        }
        mir::Instruction::AtomicLoad { .. } | mir::Instruction::AtomicCompareExchange { .. }
            if natural_loop.contains(block) =>
        {
            return Ok(Some(instruction_id));
        }
        _ if instruction.has_side_effects() => return Ok(None),
        _ => {}
    }

    // trace ordinary SSA computations to their operands
    for value in instruction.reads(tree) {
        let poll = find_atomic_definition(value, natural_loop, entry, definitions, tree, visited)?;
        if poll.is_some() {
            return Ok(poll);
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an atomic load loop without a processor hint.
    #[test]
    fn test_reports_atomic_load_polling_loop() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>):
    jump poll

poll:
    v1: boolean = atomic.load v0, acquire, scope(system)
    branch v1 => done | poll

done:
    return
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-spin-loop]: atomic busy-wait loop has no processor hint
 ──▶ main.mir:6:5
  │
4 │
5 │ poll:
6 │     v1: boolean = atomic.load v0, acquire, scope(system)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     branch v1 => done | poll
8 │
  │

 = help: call spinLoop() during bounded spinning or use a blocking wait
"#,
        );
    }

    /// Accept an atomic polling loop with a processor hint.
    #[test]
    fn test_accepts_atomic_load_polling_loop_with_spin_hint() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>):
    jump poll

poll:
    v1: boolean = atomic.load v0, acquire, scope(system)
    intrinsic.hint.spinLoop()
    branch v1 => done | poll

done:
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore loops whose observable work is not limited to polling.
    #[test]
    fn test_ignores_atomic_loop_with_observable_work() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>):
    jump poll

poll:
    v1: boolean = atomic.load v0, acquire, scope(system)
    atomic.store v0, v1, release, scope(system)
    branch v1 => done | poll

done:
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a compare-exchange retry loop without a processor hint.
    #[test]
    fn test_reports_atomic_compare_exchange_loop() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<uint32, borrowed, 'a, mutable>): void {
entry(v0: ref<uint32, borrowed, 'a, mutable>):
    v1: uint32 = 0
    v2: uint32 = 1
    jump poll(v1, v2)

poll(v3: uint32, v4: uint32):
    v5: (uint32, boolean) = atomic.cas.weak v0, v3, v4, acquireRelease, failure(acquire)
    v6: boolean = field.get v5, 1
    branch v6 => done | poll(v3, v4)

done:
    return
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-spin-loop]: atomic busy-wait loop has no processor hint
  ──▶ main.mir:8:5
   │
 6 │
 7 │ poll(v3: uint32, v4: uint32):
 8 │     v5: (uint32, boolean) = atomic.cas.weak v0, v3, v4, acquireRelease, failure(acquire)
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     v6: boolean = field.get v5, 1
10 │     branch v6 => done | poll(v3, v4)
   │

 = help: call spinLoop() during bounded spinning or use a blocking wait
"#,
        );
    }

    /// Treat a nested polling loop as substantive work in its parent loop.
    #[test]
    fn test_accepts_nested_polling_loop_with_spin_hint() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>):
    jump outer

outer:
    v1: boolean = atomic.load v0, acquire, scope(system)
    branch v1 => done | inner

inner:
    v2: boolean = atomic.load v0, acquire, scope(system)
    intrinsic.hint.spinLoop()
    branch v2 => outer | inner

done:
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Follow ordinary SSA operations from an atomic poll to the exit branch.
    #[test]
    fn test_reports_negated_atomic_condition() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>):
    jump poll

poll:
    v1: boolean = atomic.load v0, acquire, scope(system)
    v2: boolean = not v1
    branch v2 => poll | done

done:
    return
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-spin-loop]: atomic busy-wait loop has no processor hint
 ──▶ main.mir:6:5
  │
4 │
5 │ poll:
6 │     v1: boolean = atomic.load v0, acquire, scope(system)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     v2: boolean = not v1
8 │     branch v2 => poll | done
  │

 = help: call spinLoop() during bounded spinning or use a blocking wait
"#,
        );
    }

    /// Follow atomic conditions carried through block parameters.
    #[test]
    fn test_reports_atomic_condition_through_block_parameter() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>):
    jump poll

poll:
    v1: boolean = atomic.load v0, acquire, scope(system)
    jump decide(v1)

decide(v2: boolean):
    branch v2 => done | poll

done:
    return
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-spin-loop]: atomic busy-wait loop has no processor hint
 ──▶ main.mir:6:5
  │
4 │
5 │ poll:
6 │     v1: boolean = atomic.load v0, acquire, scope(system)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     jump decide(v1)
8 │
  │

 = help: call spinLoop() during bounded spinning or use a blocking wait
"#,
        );
    }

    /// Ignore atomic operations that do not contribute to loop control.
    #[test]
    fn test_ignores_unrelated_atomic_load() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>, v1: boolean): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>, v1: boolean):
    jump poll

poll:
    v2: boolean = atomic.load v0, acquire, scope(system)
    branch v1 => done | poll

done:
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Require a processor hint along every backedge path.
    #[test]
    fn test_reports_retry_path_without_spin_hint() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>, v1: boolean): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>, v1: boolean):
    jump poll

poll:
    v2: boolean = atomic.load v0, acquire, scope(system)
    branch v2 => done | retry

retry:
    branch v1 => hinted | unhinted

hinted:
    intrinsic.hint.spinLoop()
    jump poll

unhinted:
    jump poll

done:
    return
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-spin-loop]: atomic busy-wait loop has no processor hint
 ──▶ main.mir:6:5
  │
4 │
5 │ poll:
6 │     v2: boolean = atomic.load v0, acquire, scope(system)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     branch v2 => done | retry
8 │
  │

 = help: call spinLoop() during bounded spinning or use a blocking wait
"#,
        );
    }

    /// Accept a hint that dominates every backedge path.
    #[test]
    fn test_accepts_spin_hint_before_retry_paths_diverge() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: ref<boolean, borrowed, 'a, mutable>, v1: boolean): void {
entry(v0: ref<boolean, borrowed, 'a, mutable>, v1: boolean):
    jump poll

poll:
    v2: boolean = atomic.load v0, acquire, scope(system)
    branch v2 => done | retry

retry:
    intrinsic.hint.spinLoop()
    branch v1 => first | second

first:
    jump poll

second:
    jump poll

done:
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore loops that do not poll atomic storage.
    #[test]
    fn test_ignores_non_atomic_loop() {
        let session = TestSession::mir(
            &MISSING_SPIN_LOOP,
            r#"function wait<'a>(v0: boolean): void {
entry(v0: boolean):
    branch v0 => done | entry(v0)

done:
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
