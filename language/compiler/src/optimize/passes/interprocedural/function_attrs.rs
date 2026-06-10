use std::collections::{HashMap, VecDeque};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::CallGraph;
use crate::optimize::{AnalysisPreservation, ModuleAnalyses, ModulePass, PipelineContext};

declare_mir_pass! {
    /// Infer memory and behavior attributes for functions and callsites.
    ///
    /// This pass aggregates memory effects, allocation behavior, and convergence from instruction semantics and direct callees.
    /// This pass fills missing metadata and rederives summaries after IR changes.
    ///
    /// ```mir
    /// function pure(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = int.add v0, v0
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function pure(v0: int32): int32 { ... } // memory_effects = none
    /// ```
    #[pass(id = "function-attrs")]
    pub FunctionAttrs,
    "Infer function attributes"
}

impl ModulePass for FunctionAttrs {
    /// Run the function attribute inference pass.
    fn run(&self, tree: &mut mir::Tree, _ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_function_attrs(tree);

        // report analysis preservation based on whether changes occurred
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "FunctionAttrs"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "function-attrs"
    }
}

/// Summary of function behavior and memory effects.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSummary {
    /// Memory effects for the function.
    memory_effects: mir::MemoryEffect,
    /// Behavioral effects for the function.
    function_behavior: mir::FunctionBehavior,
}

/// Builder for aggregate memory effects.
struct MemoryEffectBuilder {
    /// Whether any read is observed.
    reads: bool,
    /// Whether any write is observed.
    writes: bool,
    /// The aggregate memory space set.
    spaces: mir::SpaceSet,
    /// Whether at least one access is recorded.
    has_access: bool,
}

impl MemoryEffectBuilder {
    /// Create a new empty memory effect builder.
    fn new() -> Self {
        // seed the builder with no effects
        Self {
            reads: false,
            writes: false,
            spaces: mir::SpaceSet::NONE,
            has_access: false,
        }
    }

    /// Record a memory effect into the builder.
    fn record_effect(&mut self, effect: &mir::MemoryEffect) {
        // ignore effects with no memory access
        if !effect.reads && !effect.writes {
            return;
        }

        // merge access flags and sets
        self.has_access = true;
        self.reads |= effect.reads;
        self.writes |= effect.writes;
        self.spaces.insert(effect.spaces);
    }

    /// Finish the builder into a memory effect summary.
    fn finish(self) -> mir::MemoryEffect {
        if !self.has_access {
            return mir::MemoryEffect::none();
        }

        mir::MemoryEffect {
            reads: self.reads,
            writes: self.writes,
            spaces: self.spaces,
        }
    }
}

/// Builder for function behavior summaries.
struct FunctionBehaviorBuilder {
    /// Determinism for the function.
    determinism: mir::Determinism,
    /// Whether any callee may suspend execution.
    may_suspend: bool,
    /// Whether execution may panic.
    may_panic: bool,
    /// Whether any callee must not be duplicated.
    must_not_duplicate: bool,
    /// Whether any callee allocates.
    allocates: bool,
    /// Whether any callee frees memory.
    frees: bool,
}

impl FunctionBehaviorBuilder {
    /// Create a new behavior builder.
    fn new() -> Self {
        // seed the builder with no behavior flags
        Self {
            determinism: mir::Determinism::Deterministic,
            may_suspend: false,
            may_panic: false,
            must_not_duplicate: false,
            allocates: false,
            frees: false,
        }
    }

    /// Record a function behavior into the builder.
    fn record_behavior(&mut self, behavior: &mir::FunctionBehavior) {
        // merge determinism and behavior flags
        if behavior.determinism == mir::Determinism::NonDeterministic {
            self.determinism = mir::Determinism::NonDeterministic;
        }
        self.may_suspend |= behavior.suspend.may_suspend();
        self.may_panic |= behavior.panic.may_panic();

        // merge duplication restrictions
        self.must_not_duplicate |= behavior.must_not_duplicate;

        // merge allocation information
        self.allocates |= behavior.allocates;

        // merge free information
        self.frees |= behavior.frees;
    }

    /// Finish the builder into a function behavior summary.
    fn finish(self, noreturn: bool) -> mir::FunctionBehavior {
        // assemble the final function behavior summary
        mir::FunctionBehavior {
            determinism: self.determinism,
            suspend: if self.may_suspend {
                mir::SuspendBehavior::MaySuspend
            } else {
                mir::SuspendBehavior::CannotSuspend
            },
            return_behavior: if noreturn {
                mir::ReturnBehavior::NoReturn
            } else {
                mir::ReturnBehavior::MayReturn
            },
            panic: if self.may_panic {
                mir::PanicBehavior::MayPanic
            } else {
                mir::PanicBehavior::CannotPanic
            },
            must_not_duplicate: self.must_not_duplicate,
            allocates: self.allocates,
            frees: self.frees,
        }
    }
}

/// Run function attribute inference over the module.
fn run_function_attrs(tree: &mut mir::Tree) -> bool {
    // build the call graph for direct caller tracking
    let analyses = ModuleAnalyses::new(tree);
    let callgraph = analyses.get::<CallGraph>();

    // collect functions that have bodies
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| function.entry.is_some().then_some(id))
        .collect();

    // initialize the summary cache and worklist
    let mut summaries: HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary> = HashMap::new();
    let mut worklist: VecDeque<_> = function_ids.iter().copied().collect();

    // propagate summaries to a fixpoint
    while let Some(function_id) = worklist.pop_front() {
        // recompute the summary for this function
        let summary = compute_function_summary(tree, function_id, &summaries);

        // detect changes to the cached summary
        let changed = summaries
            .get(&function_id)
            .map(|existing| existing != &summary)
            .unwrap_or(true);

        // skip propagation when nothing changed
        if !changed {
            continue;
        }

        // update the summary cache
        summaries.insert(function_id, summary);

        // propagate changes to direct callers
        for edge in callgraph.incoming(function_id) {
            if edge.dispatch == mir::CallDispatchKind::Direct {
                worklist.push_back(edge.caller);
            }
        }
    }

    // apply summaries to function and callsite metadata
    let mut changed = false;
    for function_id in function_ids {
        // read the computed summary for this function
        let Some(summary) = summaries.get(&function_id) else {
            continue;
        };

        // merge summaries with existing annotations
        let metadata = tree
            .metadata
            .functions
            .function(function_id)
            .cloned()
            .unwrap_or_default();
        let merged_memory = merge_memory_effect(Some(&metadata.memory), &summary.memory_effects);
        let merged_behavior =
            merge_function_behavior(Some(&metadata.behavior), &summary.function_behavior);

        // write back any updated attributes
        let mut function_changed = false;
        {
            let metadata = tree.metadata.functions.function_mut(function_id);
            if metadata.memory != merged_memory {
                metadata.memory = merged_memory.clone();
                function_changed = true;
            }
            if metadata.behavior != merged_behavior {
                metadata.behavior = merged_behavior.clone();
                function_changed = true;
            }
        }

        // update callsite metadata in the function body
        let callsite_changed = update_call_metadata(tree, function_id, &summaries);
        changed |= function_changed || callsite_changed;
    }

    changed
}

/// Compute a summary for a function using current callee summaries.
fn compute_function_summary(
    tree: &mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> FunctionSummary {
    // scan instructions and terminators for effects
    let function = tree.get(function_id);
    let mut memory_builder = MemoryEffectBuilder::new();
    let mut behavior_builder = FunctionBehaviorBuilder::new();
    let mut has_return = false;

    // walk each block to collect memory effects
    for &block_id in &function.blocks {
        // scan instructions in the block
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some((effect, behavior)) =
                call_effects_for_instruction(tree, instruction_id, instruction, summaries)
            {
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                continue;
            }

            // record non call instruction effects
            let (effect, behavior) =
                effects_for_instruction(tree, function, instruction_id, instruction);
            memory_builder.record_effect(&effect);
            behavior_builder.record_behavior(&behavior);
        }

        // record terminator level effects
        let terminator = tree.get(block.terminator);
        match terminator {
            mir::Terminator::Return { .. } => {
                has_return = true;
            }
            mir::Terminator::Call { function, .. } => {
                let Some(function) = function.function() else {
                    continue;
                };

                let (effect, behavior) = call_effects_for_direct_callee(function, summaries);
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                if !behavior.return_behavior.is_no_return() {
                    has_return = true;
                }
            }
            mir::Terminator::CallIndirect { .. }
            | mir::Terminator::CallVirtual { .. }
            | mir::Terminator::CallDynamic { .. } => {
                let (effect, behavior) =
                    call_effects_for_dynamic_terminator(tree, block_id, summaries);
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                if !behavior.return_behavior.is_no_return() {
                    has_return = true;
                }
            }
            mir::Terminator::Panic { .. } | mir::Terminator::ResumeUnwind => {
                behavior_builder.may_panic = true;
            }
            mir::Terminator::Trap { .. } => {}
            mir::Terminator::TailCall { function, .. } => {
                let Some(function) = function.function() else {
                    continue;
                };

                let (effect, behavior) = call_effects_for_direct_callee(function, summaries);
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                if !behavior.return_behavior.is_no_return() {
                    has_return = true;
                }
            }
            mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallDynamic { .. } => {
                let (effect, behavior) =
                    call_effects_for_dynamic_terminator(tree, block_id, summaries);
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                has_return = true;
            }
            mir::Terminator::Yield { .. } => {
                behavior_builder.may_suspend = true;
            }
            _ => {}
        }
    }

    // build the function behavior summary
    let noreturn = function.suspension.is_none() && !has_return;
    let function_behavior = behavior_builder.finish(noreturn);

    FunctionSummary {
        memory_effects: memory_builder.finish(),
        function_behavior,
    }
}

/// Merge memory effects with any existing annotations.
fn merge_memory_effect(
    existing: Option<&mir::MemoryEffect>,
    inferred: &mir::MemoryEffect,
) -> mir::MemoryEffect {
    // prefer inferred effects when nothing was annotated
    let Some(existing) = existing else {
        return inferred.clone();
    };

    // treat the default unknown effect as missing annotation
    if *existing == mir::MemoryEffect::unknown() {
        return inferred.clone();
    }

    // union the memory effect flags and sets
    let mut merged = existing.clone();
    merged.reads |= inferred.reads;
    merged.writes |= inferred.writes;
    merged.spaces.insert(inferred.spaces);

    merged
}

/// Merge function behavior with any existing annotations.
fn merge_function_behavior(
    existing: Option<&mir::FunctionBehavior>,
    inferred: &mir::FunctionBehavior,
) -> mir::FunctionBehavior {
    // prefer inferred behavior when nothing was annotated
    let Some(existing) = existing else {
        return inferred.clone();
    };

    // treat the default unknown behavior as missing annotation
    if *existing == mir::FunctionBehavior::unknown() {
        return inferred.clone();
    }

    // union behavioral flags and sets
    let mut merged = existing.clone();
    if inferred.determinism == mir::Determinism::NonDeterministic {
        merged.determinism = mir::Determinism::NonDeterministic;
    }
    if inferred.suspend.may_suspend() {
        merged.suspend = mir::SuspendBehavior::MaySuspend;
    }
    if inferred.panic.may_panic() {
        merged.panic = mir::PanicBehavior::MayPanic;
    }
    merged.return_behavior = match (merged.return_behavior, inferred.return_behavior) {
        (mir::ReturnBehavior::NoReturn, mir::ReturnBehavior::NoReturn) => {
            mir::ReturnBehavior::NoReturn
        }
        (mir::ReturnBehavior::WillReturn, mir::ReturnBehavior::WillReturn) => {
            mir::ReturnBehavior::WillReturn
        }
        _ => mir::ReturnBehavior::MayReturn,
    };
    merged.must_not_duplicate |= inferred.must_not_duplicate;
    merged.allocates |= inferred.allocates;
    merged.frees |= inferred.frees;

    merged
}

/// Update call metadata entries with inferred callee summaries.
fn update_call_metadata(
    tree: &mut mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> bool {
    // apply callee summaries to call metadata when missing
    let mut changed = false;
    let block_ids = tree.get(function_id).blocks.clone();
    for block_id in block_ids {
        // scan call instructions in the block
        let instruction_ids = tree.get(block_id).instructions.clone();
        for instruction_id in instruction_ids {
            let callee_id = {
                let instruction = tree.get(instruction_id);
                direct_callee_for_instruction(instruction)
            };
            let Some(callee_id) = callee_id else {
                continue;
            };
            let Some(summary) = summaries.get(&callee_id) else {
                continue;
            };

            // populate missing callsite metadata from the callee summary
            let callsite = mir::CallSite::Instruction(instruction_id);
            changed |= update_callsite_metadata(tree, callsite, callee_id, summary);
        }

        // scan call terminators in the block
        let callee_id = {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            terminator
                .call_direct_target()
                .and_then(|target| target.function())
        };
        let Some(callee_id) = callee_id else {
            continue;
        };
        let Some(summary) = summaries.get(&callee_id) else {
            continue;
        };

        // populate missing callsite metadata from the callee summary
        let callsite = mir::CallSite::Terminator(block_id);
        changed |= update_callsite_metadata(tree, callsite, callee_id, summary);
    }

    changed
}

/// Update missing metadata for one resolved callsite.
fn update_callsite_metadata(
    tree: &mut mir::Tree,
    callsite: mir::CallSite,
    callee: mir::LocalNodeId<mir::Function>,
    summary: &FunctionSummary,
) -> bool {
    let metadata = tree.metadata.functions.call_mut(callsite);
    let mut changed = false;

    // fill missing memory effects
    if metadata.memory == mir::MemoryEffect::unknown() {
        metadata.memory = summary.memory_effects.clone();
        changed = true;
    }

    // fill missing function behavior
    if metadata.behavior == mir::FunctionBehavior::unknown() {
        metadata.behavior = summary.function_behavior.clone();
        changed = true;
    }

    // fill missing target resolution
    if metadata.target.is_none() {
        metadata.target = Some(callee);
        changed = true;
    }

    changed
}

/// Resolve call effects for a call instruction.
fn call_effects_for_instruction(
    tree: &mir::Tree,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> Option<(mir::MemoryEffect, mir::FunctionBehavior)> {
    // skip non call instructions
    let is_call = matches!(
        instruction,
        mir::Instruction::Call { .. }
            | mir::Instruction::CallVirtual { .. }
            | mir::Instruction::CallDynamic { .. }
            | mir::Instruction::CallIndirect { .. }
    );
    if !is_call {
        return None;
    }

    // seed effects from call metadata when present
    let callsite = mir::CallSite::Instruction(instruction_id);
    let call_metadata = tree.metadata.functions.call(callsite);
    let mut memory_effect = call_metadata
        .filter(|metadata| metadata.memory != mir::MemoryEffect::unknown())
        .map(|metadata| metadata.memory.clone());
    let mut behavior = call_metadata
        .filter(|metadata| metadata.behavior != mir::FunctionBehavior::unknown())
        .map(|metadata| metadata.behavior.clone());

    // resolve the best known target
    let callee = direct_callee_for_instruction(instruction)
        .or_else(|| call_metadata.and_then(|metadata| metadata.target));

    // fill missing pieces from direct callee summaries
    if let Some(summary) = callee.and_then(|callee_id| summaries.get(&callee_id)) {
        if memory_effect.is_none() {
            memory_effect = Some(summary.memory_effects.clone());
        }
        if behavior.is_none() {
            behavior = Some(summary.function_behavior.clone());
        }
    }

    // default to unknown effects when metadata is missing
    let memory_effect = memory_effect.unwrap_or_else(mir::MemoryEffect::unknown);
    let behavior = behavior.unwrap_or_else(mir::FunctionBehavior::unknown);
    Some((memory_effect, behavior))
}

/// Resolve call effects for a direct callee.
fn call_effects_for_direct_callee(
    callee: mir::LocalNodeId<mir::Function>,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> (mir::MemoryEffect, mir::FunctionBehavior) {
    // read the cached summary for the callee
    let Some(summary) = summaries.get(&callee) else {
        return (
            mir::MemoryEffect::unknown(),
            mir::FunctionBehavior::unknown(),
        );
    };

    (
        summary.memory_effects.clone(),
        summary.function_behavior.clone(),
    )
}

/// Resolve call effects for a dynamic call terminator.
fn call_effects_for_dynamic_terminator(
    tree: &mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> (mir::MemoryEffect, mir::FunctionBehavior) {
    // seed effects from call metadata when present
    let callsite = mir::CallSite::Terminator(block_id);
    let call_metadata = tree.metadata.functions.call(callsite);
    let mut memory_effect = call_metadata
        .filter(|metadata| metadata.memory != mir::MemoryEffect::unknown())
        .map(|metadata| metadata.memory.clone());
    let mut behavior = call_metadata
        .filter(|metadata| metadata.behavior != mir::FunctionBehavior::unknown())
        .map(|metadata| metadata.behavior.clone());

    // resolve the best known target
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);
    let callee = terminator
        .call_direct_target()
        .and_then(|target| target.function())
        .or_else(|| call_metadata.and_then(|metadata| metadata.target));

    // fill missing pieces from direct callee summaries
    if let Some(summary) = callee.and_then(|callee| summaries.get(&callee)) {
        if memory_effect.is_none() {
            memory_effect = Some(summary.memory_effects.clone());
        }
        if behavior.is_none() {
            behavior = Some(summary.function_behavior.clone());
        }
    }

    // default to unknown effects when metadata is missing
    let memory_effect = memory_effect.unwrap_or_else(mir::MemoryEffect::unknown);
    let behavior = behavior.unwrap_or_else(mir::FunctionBehavior::unknown);
    (memory_effect, behavior)
}

/// Resolve a direct callee id for a call instruction.
fn direct_callee_for_instruction(
    instruction: &mir::Instruction,
) -> Option<mir::LocalNodeId<mir::Function>> {
    match instruction {
        mir::Instruction::Call { function, .. } => function.function(),
        _ => None,
    }
}

/// Compute effects for non call instructions.
fn effects_for_instruction(
    tree: &mir::Tree,
    function: &mir::Function,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
) -> (mir::MemoryEffect, mir::FunctionBehavior) {
    // prefer explicit memory access metadata when present
    if let Some(accesses) = tree.metadata.memory.memory_accesses(instruction_id) {
        let effect = memory_effect_from_accesses(accesses);
        return (effect, mir::FunctionBehavior::none());
    }

    // map instruction semantics to effect summaries
    match instruction {
        mir::Instruction::Load { .. } => {
            let effect = mir::MemoryEffect::read_only(mir::SpaceSet::ANY);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::AtomicLoad { .. } => {
            let effect = mir::MemoryEffect::read_only(mir::SpaceSet::ANY);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::Store { .. } => {
            let effect = mir::MemoryEffect::write_only(mir::SpaceSet::ANY);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::AtomicStore { .. } => {
            let effect = mir::MemoryEffect::write_only(mir::SpaceSet::ANY);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::AtomicCompareExchange { .. } | mir::Instruction::AtomicRmw { .. } => {
            let effect = mir::MemoryEffect::read_write(mir::SpaceSet::ANY);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::AtomicFence { .. } => {
            let effect = mir::MemoryEffect::read_write(mir::SpaceSet::ANY);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::BarrierWrite { .. } => {
            let effect = mir::MemoryEffect::none();
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::LocalGet { .. } => {
            let effect = frame_effect(mir::MemoryEffect::read_only(mir::SpaceSet::FRAME));
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::LocalSet { .. } => {
            let effect = frame_effect(mir::MemoryEffect::write_only(mir::SpaceSet::FRAME));
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::NewZeroed { result_type, .. }
        | mir::Instruction::NewUninit { result_type, .. }
        | mir::Instruction::NewSliceZeroed { result_type, .. }
        | mir::Instruction::NewSliceUninit { result_type, .. } => {
            let spaces = space_set_for_type(tree, result_type);
            let effect = mir::MemoryEffect::write_only(spaces);
            let behavior = alloc_behavior();
            (effect, behavior)
        }
        mir::Instruction::Free { value } => {
            let spaces = space_set_for_value(tree, function, *value);
            let effect = mir::MemoryEffect::write_only(spaces);
            let behavior = free_behavior();
            (effect, behavior)
        }
        mir::Instruction::FrameAllocZeroed { .. } | mir::Instruction::FrameAllocUninit { .. } => {
            let effect = frame_effect(mir::MemoryEffect::write_only(mir::SpaceSet::FRAME));
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::NewComplete { result_type, .. } => {
            let spaces = space_set_for_type(tree, result_type);
            let effect = mir::MemoryEffect::read_write(spaces);
            (effect, mir::FunctionBehavior::none())
        }
        mir::Instruction::Pin { .. }
        | mir::Instruction::Unpin { .. }
        | mir::Instruction::Drop { .. } => (
            mir::MemoryEffect::unknown(),
            mir::FunctionBehavior::unknown(),
        ),
        mir::Instruction::Intrinsic { intrinsic, .. } => {
            let effect = memory_effect_for_intrinsic(*intrinsic);
            (effect, mir::FunctionBehavior::none())
        }
        _ => (mir::MemoryEffect::none(), mir::FunctionBehavior::none()),
    }
}

/// Build a memory effect from memory access metadata entries.
fn memory_effect_from_accesses(accesses: &[mir::MemoryAccessMetadata]) -> mir::MemoryEffect {
    let mut builder = MemoryEffectBuilder::new();
    for access in accesses {
        let effect = memory_effect_for_access(access);
        builder.record_effect(&effect);
    }

    builder.finish()
}

/// Build a memory effect for a single access entry.
fn memory_effect_for_access(access: &mir::MemoryAccessMetadata) -> mir::MemoryEffect {
    // map access kind to a base memory effect
    let mut effect = match access.kind {
        mir::MemoryAccessKind::Read | mir::MemoryAccessKind::PrefetchRead => {
            mir::MemoryEffect::read_only(mir::SpaceSet::ANY)
        }
        mir::MemoryAccessKind::Write | mir::MemoryAccessKind::PrefetchWrite => {
            mir::MemoryEffect::write_only(mir::SpaceSet::ANY)
        }
        mir::MemoryAccessKind::ReadWrite
        | mir::MemoryAccessKind::ReadModifyWrite
        | mir::MemoryAccessKind::Fence => mir::MemoryEffect::read_write(mir::SpaceSet::ANY),
    };

    // apply space annotations
    if let Some(space) = access.space.clone() {
        effect.spaces = space_set_for_space(space);
    }
    // refine space sets for locals and globals
    match access.target {
        mir::MemoryAccessTarget::Local(_) => {
            effect.spaces = mir::SpaceSet::FRAME;
        }
        mir::MemoryAccessTarget::Global(_) => {
            effect.spaces = mir::SpaceSet::STATIC;
        }
        _ => {}
    }

    effect
}

/// Build a memory effect for an intrinsic.
fn memory_effect_for_intrinsic(intrinsic: mir::Intrinsic) -> mir::MemoryEffect {
    // classify intrinsic memory effects
    match intrinsic {
        mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => {
            mir::MemoryEffect::read_write(mir::SpaceSet::ANY)
        }
        mir::Intrinsic::Memset => mir::MemoryEffect::write_only(mir::SpaceSet::ANY),
        mir::Intrinsic::Memcmp => mir::MemoryEffect::read_only(mir::SpaceSet::ANY),
        mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
            mir::MemoryEffect::read_only(mir::SpaceSet::ANY)
        }
        _ => mir::MemoryEffect::none(),
    }
}

/// Apply frame space metadata to a memory effect.
fn frame_effect(mut effect: mir::MemoryEffect) -> mir::MemoryEffect {
    effect.spaces = mir::SpaceSet::FRAME;
    effect
}

/// Build a function behavior for an allocation.
fn alloc_behavior() -> mir::FunctionBehavior {
    mir::FunctionBehavior::none().with_allocates()
}

/// Build a function behavior for a free.
fn free_behavior() -> mir::FunctionBehavior {
    mir::FunctionBehavior::none().with_frees()
}

/// Resolve the backing space for one typed value.
fn space_set_for_value(
    tree: &mir::Tree,
    function: &mir::Function,
    value: mir::ValueReference,
) -> mir::SpaceSet {
    let Some(value) = value.value() else {
        return mir::SpaceSet::ANY;
    };
    let Some(ty) = function.value_type(value) else {
        return mir::SpaceSet::ANY;
    };

    let ty = mir::TypeReference::from(ty);

    space_set_for_type(tree, &ty)
}

/// Resolve the backing space for one reference-like type.
fn space_set_for_type(tree: &mir::Tree, ty: &mir::TypeReference) -> mir::SpaceSet {
    let Some(ty) = ty.ty() else {
        return mir::SpaceSet::ANY;
    };

    match tree.get(ty) {
        mir::Type::Uninit { value } => space_set_for_type(tree, value),
        mir::Type::Reference { space, .. } | mir::Type::TensorView { space, .. } => {
            space_set_for_space(space.clone())
        }
        _ => mir::SpaceSet::ANY,
    }
}

/// Resolve the backing space for one space.
fn space_set_for_space(space: mir::Space) -> mir::SpaceSet {
    match space {
        mir::Space::Local => mir::SpaceSet::LOCAL,
        mir::Space::Shared => mir::SpaceSet::SHARED,
        mir::Space::Static => mir::SpaceSet::STATIC,
        mir::Space::Frame => mir::SpaceSet::FRAME,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Functions without memory effects are marked as no memory.
    #[test]
    fn test_function_attrs_pure() {
        let input = r#"
function pure(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("pure");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(function_id)
            .expect("missing function metadata");

        assert_eq!(metadata.memory, mir::MemoryEffect::none());
    }

    /// Allocation and free instructions are surfaced in function behavior.
    #[test]
    fn test_function_attrs_alloc_behavior() {
        let input = r#"
function alloc(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("alloc");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(function_id)
            .expect("missing function metadata");
        let behavior = metadata.behavior.clone();

        assert!(behavior.allocates);
        assert!(behavior.frees);
    }

    /// Panic terminators are surfaced in function behavior.
    #[test]
    fn test_function_attrs_panic_behavior() {
        let input = r#"
function fail(): void {
b0:
    panic
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("fail");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(function_id)
            .expect("missing function metadata");
        let behavior = metadata.behavior.clone();

        assert_eq!(behavior.panic, mir::PanicBehavior::MayPanic);
        assert_eq!(behavior.return_behavior, mir::ReturnBehavior::NoReturn);
    }

    /// Call metadata is populated from callee summaries.
    #[test]
    fn test_function_attrs_updates_call_metadata() {
        let input = r#"
function callee(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}
function caller(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let caller_id = test.function_id_by_name("caller");
        let (call_inst, _callee_id) = test.first_call_in_entry(caller_id);
        let callsite = mir::CallSite::Instruction(call_inst);
        test.tree.metadata.functions.call_mut(callsite).memory = mir::MemoryEffect::none();

        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let metadata = test
            .tree
            .metadata
            .functions
            .call(callsite)
            .expect("missing call metadata");
        assert_eq!(metadata.memory, mir::MemoryEffect::none());
    }

    /// Tail calls to returning functions do not imply noreturn.
    #[test]
    fn test_function_attrs_tailcall_returns() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): int32 {
b0(v0: int32):
    tailCall callee(v0): (int32) -> int32
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let caller_id = test.function_id_by_name("caller");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(caller_id)
            .expect("missing function metadata");
        let behavior = metadata.behavior.clone();

        assert_ne!(behavior.return_behavior, mir::ReturnBehavior::NoReturn);
    }

    /// Tail calls to noreturn callees propagate noreturn.
    #[test]
    fn test_function_attrs_tailcall_noreturn() {
        let input = r#"
function sink(): void {
b0:
    jump b0
}
function caller(): void {
b0:
    tailCall sink(): () -> void
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let caller_id = test.function_id_by_name("caller");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(caller_id)
            .expect("missing function metadata");
        let behavior = metadata.behavior.clone();

        assert_eq!(behavior.return_behavior, mir::ReturnBehavior::NoReturn);
    }

    /// Unknown indirect calls produce unknown memory effects.
    #[test]
    fn test_function_attrs_unknown_indirect_effects() {
        let input = r#"
function callee(v0: (int32) -> int32, v1: int32): int32  {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("callee");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(function_id)
            .expect("missing function metadata");
        let effects = &metadata.memory;

        assert!(effects.reads);
        assert!(effects.writes);
    }

    /// Resolved dynamic calls contribute callee summaries.
    #[test]
    fn test_function_attrs_resolved_dynamic_call_effects() {
        let input = r#"
function callee(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = 1int32
    store v0, v1
    return v1
}
function caller(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    v1: int32 = call.virtual v0, int32, 1(v0): (ref<int32, raw>) -> int32
    return
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("callee");
        let caller_id = test.function_id_by_name("caller");
        let call_id = test
            .entry_instructions(caller_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    test.tree.get(*instruction_id),
                    mir::Instruction::CallVirtual { .. }
                )
            })
            .expect("missing virtual call instruction");
        let callsite = mir::CallSite::Instruction(call_id);
        test.tree.metadata.functions.call_mut(callsite).target = Some(callee_id);

        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let metadata = test
            .tree
            .metadata
            .functions
            .function(caller_id)
            .expect("missing function metadata");

        assert!(metadata.memory.writes);
    }

    /// Direct call terminators contribute callee summaries.
    #[test]
    fn test_function_attrs_call_terminator_effects() {
        let input = r#"
function callee(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    v1: int32 = 1int32
    store v0, v1
    return
}
function caller(v0: ref<int32, raw>, v1: ref<void, managed, readonly>): void {
b0(v0: ref<int32, raw>, v1: ref<void, managed, readonly>):
    call callee(v0): (ref<int32, raw>) -> void -> b1
b1:
    return
b2(v2: ref<void, managed, readonly>):
    panic v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let caller_id = test.function_id_by_name("caller");
        let metadata = test
            .tree
            .metadata
            .functions
            .function(caller_id)
            .expect("missing function metadata");

        assert!(metadata.memory.writes);

        let entry = test.entry_block_id(caller_id);
        let callsite = mir::CallSite::Terminator(entry);
        let metadata = test
            .tree
            .metadata
            .functions
            .call(callsite)
            .expect("missing call terminator metadata");

        assert!(metadata.memory.writes);
        assert_eq!(
            metadata.behavior.return_behavior,
            mir::ReturnBehavior::MayReturn
        );
    }
}
