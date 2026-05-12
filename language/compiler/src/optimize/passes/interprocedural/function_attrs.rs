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
    call_behavior: mir::CallBehavior,
}

/// Builder for aggregate memory effects.
struct MemoryEffectBuilder {
    /// Whether any read is observed.
    reads: bool,
    /// Whether any write is observed.
    writes: bool,
    /// The aggregate memory space set.
    spaces: mir::MemorySpaceSet,
    /// The aggregate address space set when available.
    address_spaces: Option<mir::AddressSpaceSet>,
    /// Whether effects are limited to argument memory.
    argmemonly: bool,
    /// Whether effects are limited to inaccessible memory.
    inaccessible_mem_only: bool,
    /// Whether effects are non synchronizing.
    nosync: bool,
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
            spaces: mir::MemorySpaceSet::NONE,
            address_spaces: Some(mir::AddressSpaceSet::new(Vec::new())),
            argmemonly: true,
            inaccessible_mem_only: true,
            nosync: true,
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
        self.argmemonly &= effect.argmemonly;
        self.inaccessible_mem_only &= effect.inaccessible_mem_only;
        self.nosync &= effect.nosync;

        // merge address space sets when both are available
        match (&mut self.address_spaces, &effect.address_spaces) {
            (Some(existing), Some(other)) => {
                // append missing address spaces
                for space in &other.spaces {
                    if !existing.contains(space.clone()) {
                        existing.spaces.push(space.clone());
                    }
                }
            }
            _ => {
                self.address_spaces = None;
            }
        }
    }

    /// Finish the builder into a memory effect summary.
    fn finish(mut self) -> mir::MemoryEffect {
        if !self.has_access {
            return mir::MemoryEffect::none();
        }

        // drop empty address space sets when nothing was recorded
        if let Some(set) = &self.address_spaces
            && set.is_empty()
        {
            self.address_spaces = None;
        }

        mir::MemoryEffect {
            reads: self.reads,
            writes: self.writes,
            spaces: self.spaces,
            address_spaces: self.address_spaces,
            argmemonly: self.argmemonly,
            inaccessible_mem_only: self.inaccessible_mem_only,
            nosync: self.nosync,
        }
    }
}

/// Builder for call behavior summaries.
struct CallBehaviorBuilder {
    /// Effect class for the function.
    effect_class: mir::EffectClass,
    /// Whether any callee may suspend execution.
    may_suspend: bool,
    /// Whether any callee must not be duplicated.
    must_not_duplicate: bool,
    /// Whether any callee allocates.
    allocates: bool,
    /// The aggregate allocation space set when available.
    alloc_spaces: Option<mir::MemorySpaceSet>,
    /// The aggregate allocation address space set when available.
    alloc_address_spaces: Option<mir::AddressSpaceSet>,
    /// Whether any callee frees memory.
    frees: bool,
    /// The aggregate free space set when available.
    free_spaces: Option<mir::MemorySpaceSet>,
    /// The aggregate free address space set when available.
    free_address_spaces: Option<mir::AddressSpaceSet>,
}

impl CallBehaviorBuilder {
    /// Create a new behavior builder.
    fn new() -> Self {
        // seed the builder with no behavior flags
        Self {
            effect_class: mir::EffectClass::Deterministic,
            may_suspend: false,
            must_not_duplicate: false,
            allocates: false,
            alloc_spaces: Some(mir::MemorySpaceSet::NONE),
            alloc_address_spaces: Some(mir::AddressSpaceSet::new(Vec::new())),
            frees: false,
            free_spaces: Some(mir::MemorySpaceSet::NONE),
            free_address_spaces: Some(mir::AddressSpaceSet::new(Vec::new())),
        }
    }

    /// Record a call behavior into the builder.
    fn record_behavior(&mut self, behavior: &mir::CallBehavior) {
        // merge effect class and behavior flags
        if behavior.effect_class == mir::EffectClass::NonDeterministic {
            self.effect_class = mir::EffectClass::NonDeterministic;
        }
        self.may_suspend |= behavior.suspend.may_suspend();

        // merge duplication restrictions
        self.must_not_duplicate |= behavior.must_not_duplicate;

        // merge allocation information
        if let Some(allocate) = &behavior.allocation.allocate {
            self.allocates = true;
            self.alloc_spaces = merge_space_set(self.alloc_spaces, Some(allocate.spaces));
            let alloc_address_spaces = self.alloc_address_spaces.take();
            self.alloc_address_spaces =
                merge_address_space_set(alloc_address_spaces, allocate.address_spaces.clone());
        }

        // merge free information
        if let Some(free) = &behavior.allocation.free {
            self.frees = true;
            self.free_spaces = merge_space_set(self.free_spaces, Some(free.spaces));
            let free_address_spaces = self.free_address_spaces.take();
            self.free_address_spaces =
                merge_address_space_set(free_address_spaces, free.address_spaces.clone());
        }
    }

    /// Finish the builder into a call behavior summary.
    fn finish(self, noreturn: bool) -> mir::CallBehavior {
        // assemble the final call behavior summary
        mir::CallBehavior {
            effect_class: self.effect_class,
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
            must_not_duplicate: self.must_not_duplicate,
            allocation: mir::AllocationEffect {
                allocate: self.allocates.then_some(mir::AllocationAccess {
                    spaces: self.alloc_spaces.unwrap_or(mir::MemorySpaceSet::ANY),
                    address_spaces: self.alloc_address_spaces,
                }),
                free: self.frees.then_some(mir::AllocationAccess {
                    spaces: self.free_spaces.unwrap_or(mir::MemorySpaceSet::ANY),
                    address_spaces: self.free_address_spaces,
                }),
            },
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
        let (merged_memory, merged_behavior) = {
            let function = tree.get(function_id);
            (
                merge_memory_effect(Some(&function.memory_effect), &summary.memory_effects),
                merge_call_behavior(Some(&function.call_behavior), &summary.call_behavior),
            )
        };

        // write back any updated attributes
        let mut function_changed = false;
        {
            let function = tree.get_mut(function_id);
            if function.memory_effect != merged_memory {
                function.memory_effect = merged_memory.clone();
                function_changed = true;
            }
            if function.call_behavior != merged_behavior {
                function.call_behavior = merged_behavior.clone();
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
    let mut behavior_builder = CallBehaviorBuilder::new();
    let mut has_return = false;

    // walk each block to collect memory effects
    for &block_id in &function.blocks {
        // scan instructions in the block
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some((effect, behavior)) =
                call_effects_for_instruction(tree, instruction, summaries)
            {
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                continue;
            }

            // record non call instruction effects
            let (effect, behavior) = effects_for_instruction(tree, instruction_id, instruction);
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
            | mir::Terminator::CallClass { .. }
            | mir::Terminator::CallInterface { .. } => {
                let (effect, behavior) =
                    call_effects_for_dynamic_terminator(tree, block_id, summaries);
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                if !behavior.return_behavior.is_no_return() {
                    has_return = true;
                }
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
            | mir::Terminator::TailCallClass { .. }
            | mir::Terminator::TailCallInterface { .. } => {
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

    // build the call behavior summary
    let noreturn = function.suspension.is_none() && !has_return;
    let call_behavior = behavior_builder.finish(noreturn);

    FunctionSummary {
        memory_effects: memory_builder.finish(),
        call_behavior,
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
    merged.argmemonly &= inferred.argmemonly;
    merged.inaccessible_mem_only &= inferred.inaccessible_mem_only;
    merged.nosync &= inferred.nosync;

    // merge address space constraints
    let address_spaces = merged.address_spaces.take();
    merged.address_spaces =
        merge_address_space_set(address_spaces, inferred.address_spaces.clone());

    merged
}

/// Merge call behavior with any existing annotations.
fn merge_call_behavior(
    existing: Option<&mir::CallBehavior>,
    inferred: &mir::CallBehavior,
) -> mir::CallBehavior {
    // prefer inferred behavior when nothing was annotated
    let Some(existing) = existing else {
        return inferred.clone();
    };

    // treat the default unknown behavior as missing annotation
    if *existing == mir::CallBehavior::unknown() {
        return inferred.clone();
    }

    // union behavioral flags and space sets
    let mut merged = existing.clone();
    merged.effect_class = match (merged.effect_class, inferred.effect_class) {
        (mir::EffectClass::NonDeterministic, _) | (_, mir::EffectClass::NonDeterministic) => {
            mir::EffectClass::NonDeterministic
        }
        (mir::EffectClass::Pure, mir::EffectClass::Pure) => mir::EffectClass::Pure,
        _ => mir::EffectClass::Deterministic,
    };
    if inferred.suspend.may_suspend() {
        merged.suspend = mir::SuspendBehavior::MaySuspend;
    }
    merged.return_behavior = match (merged.return_behavior, inferred.return_behavior) {
        (mir::ReturnBehavior::NoReturn, _) | (_, mir::ReturnBehavior::NoReturn) => {
            mir::ReturnBehavior::NoReturn
        }
        (mir::ReturnBehavior::WillReturn, mir::ReturnBehavior::WillReturn) => {
            mir::ReturnBehavior::WillReturn
        }
        _ => mir::ReturnBehavior::MayReturn,
    };
    merged.must_not_duplicate |= inferred.must_not_duplicate;
    merged.allocation.allocate = merge_allocation_access(
        merged.allocation.allocate.take(),
        inferred.allocation.allocate.clone(),
    );
    merged.allocation.free = merge_allocation_access(
        merged.allocation.free.take(),
        inferred.allocation.free.clone(),
    );

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

            // update metadata entries when missing
            let instruction = tree.get_mut(instruction_id);
            let Some(memory_effect) = instruction.call_memory_effect_mut() else {
                continue;
            };

            if memory_effect.is_none() {
                *memory_effect = Some(summary.memory_effects.clone());
                changed = true;
            }

            let Some(call_behavior) = instruction.call_behavior_mut() else {
                continue;
            };
            if call_behavior.is_none() {
                *call_behavior = Some(summary.call_behavior.clone());
                changed = true;
            }
        }
    }

    changed
}

/// Resolve call effects for a call instruction.
fn call_effects_for_instruction(
    _tree: &mir::Tree,
    instruction: &mir::Instruction,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> Option<(mir::MemoryEffect, mir::CallBehavior)> {
    // skip non call instructions
    let is_call = matches!(
        instruction,
        mir::Instruction::Call { .. }
            | mir::Instruction::CallClass { .. }
            | mir::Instruction::CallInterface { .. }
            | mir::Instruction::CallIndirect { .. }
    );
    if !is_call {
        return None;
    }

    // seed effects from call metadata when present
    let mut memory_effect = instruction.call_memory_effect().cloned();
    let mut behavior = instruction.call_behavior().cloned();

    // fall back to direct callee summaries when available
    if let Some(summary) =
        direct_callee_for_instruction(instruction).and_then(|callee_id| summaries.get(&callee_id))
    {
        if memory_effect.is_none() {
            memory_effect = Some(summary.memory_effects.clone());
        }
        if behavior.is_none() {
            behavior = Some(summary.call_behavior.clone());
        }
    }

    // default to unknown effects when metadata is missing
    let memory_effect = memory_effect.unwrap_or_else(mir::MemoryEffect::unknown);
    let behavior = behavior.unwrap_or_else(mir::CallBehavior::unknown);
    Some((memory_effect, behavior))
}

/// Resolve call effects for a direct callee.
fn call_effects_for_direct_callee(
    callee: mir::LocalNodeId<mir::Function>,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> (mir::MemoryEffect, mir::CallBehavior) {
    // read the cached summary for the callee
    let Some(summary) = summaries.get(&callee) else {
        return (mir::MemoryEffect::unknown(), mir::CallBehavior::unknown());
    };

    (
        summary.memory_effects.clone(),
        summary.call_behavior.clone(),
    )
}

/// Resolve call effects for a dynamic call terminator.
fn call_effects_for_dynamic_terminator(
    tree: &mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> (mir::MemoryEffect, mir::CallBehavior) {
    // use the declared target when present
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);
    let declared_target = terminator.call_declared_target();

    let Some(callee) = declared_target else {
        return (mir::MemoryEffect::unknown(), mir::CallBehavior::unknown());
    };

    let Some(callee) = callee.function() else {
        return (mir::MemoryEffect::unknown(), mir::CallBehavior::unknown());
    };

    call_effects_for_direct_callee(callee, summaries)
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
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
) -> (mir::MemoryEffect, mir::CallBehavior) {
    // prefer explicit memory access metadata when present
    if let Some(accesses) = tree.metadata.memory.memory_accesses(instruction_id) {
        let effect = memory_effect_from_accesses(accesses);
        return (effect, mir::CallBehavior::none());
    }

    // map instruction semantics to effect summaries
    match instruction {
        mir::Instruction::Load { .. } => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemorySpaceSet::ANY);
            effect.nosync = true;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::AtomicLoad { .. } => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemorySpaceSet::ANY);
            effect.nosync = false;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::Store { .. } => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemorySpaceSet::ANY);
            effect.nosync = true;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::AtomicStore { .. } => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemorySpaceSet::ANY);
            effect.nosync = false;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::AtomicCompareExchange { .. } | mir::Instruction::AtomicRmw { .. } => {
            let mut effect = mir::MemoryEffect::read_write(mir::MemorySpaceSet::ANY);
            effect.nosync = false;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::AtomicFence { .. } => {
            let mut effect = mir::MemoryEffect::read_write(mir::MemorySpaceSet::ANY);
            effect.nosync = false;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::BarrierWrite { .. } => {
            let mut effect = inaccessible_write_effect();
            effect.nosync = true;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::LocalGet { .. } => {
            let effect = stack_effect(mir::MemoryEffect::read_only(mir::MemorySpaceSet::STACK));
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::LocalSet { .. } => {
            let effect = stack_effect(mir::MemoryEffect::write_only(mir::MemorySpaceSet::STACK));
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::New { .. } | mir::Instruction::NewSlice { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemorySpaceSet::HEAP));
            let behavior = alloc_behavior(mir::MemorySpaceSet::HEAP, None);
            (effect, behavior)
        }
        mir::Instruction::RawAlloc { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemorySpaceSet::RAW_HEAP));
            let behavior = alloc_behavior(mir::MemorySpaceSet::RAW_HEAP, None);
            (effect, behavior)
        }
        mir::Instruction::RawFree { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemorySpaceSet::RAW_HEAP));
            let behavior = free_behavior(mir::MemorySpaceSet::RAW_HEAP, None);
            (effect, behavior)
        }
        mir::Instruction::Free { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemorySpaceSet::HEAP));
            let behavior = free_behavior(mir::MemorySpaceSet::HEAP, None);
            (effect, behavior)
        }
        mir::Instruction::StackAlloc { .. } => {
            let effect = stack_effect(mir::MemoryEffect::write_only(mir::MemorySpaceSet::STACK));
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::Pin { .. }
        | mir::Instruction::Unpin { .. }
        | mir::Instruction::Drop { .. } => {
            (mir::MemoryEffect::unknown(), mir::CallBehavior::unknown())
        }
        mir::Instruction::Intrinsic { intrinsic, .. } => {
            let effect = memory_effect_for_intrinsic(*intrinsic);
            (effect, mir::CallBehavior::none())
        }
        _ => (mir::MemoryEffect::none(), mir::CallBehavior::none()),
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
            mir::MemoryEffect::read_only(mir::MemorySpaceSet::ANY)
        }
        mir::MemoryAccessKind::Write | mir::MemoryAccessKind::PrefetchWrite => {
            mir::MemoryEffect::write_only(mir::MemorySpaceSet::ANY)
        }
        mir::MemoryAccessKind::ReadWrite
        | mir::MemoryAccessKind::ReadModifyWrite
        | mir::MemoryAccessKind::Fence => mir::MemoryEffect::read_write(mir::MemorySpaceSet::ANY),
    };

    // apply ordering and address space annotations
    effect.nosync = true;
    if let Some(space) = access.address_space.clone() {
        effect.address_spaces = Some(mir::AddressSpaceSet::new(vec![space]));
    }
    if access.is_volatile
        || access.ordering.is_some()
        || access.flags.is_some()
        || access.scope.is_some()
        || access.memory_scope.is_some()
        || access.kind == mir::MemoryAccessKind::Fence
    {
        effect.nosync = false;
    }

    // refine space sets for locals and globals
    match access.target {
        mir::MemoryAccessTarget::Local(_) => {
            effect.spaces = mir::MemorySpaceSet::STACK;
        }
        mir::MemoryAccessTarget::Global(_) => {
            effect.spaces = mir::MemorySpaceSet::STATIC;
        }
        _ => {}
    }

    effect
}

/// Build a memory effect for an intrinsic.
fn memory_effect_for_intrinsic(intrinsic: mir::Intrinsic) -> mir::MemoryEffect {
    use mir::Intrinsic;

    // classify intrinsic memory effects
    match intrinsic {
        Intrinsic::Memcpy | Intrinsic::Memmove => {
            let mut effect = mir::MemoryEffect::read_write(mir::MemorySpaceSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::Memset => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemorySpaceSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::Memcmp => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemorySpaceSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::PrefetchRead | Intrinsic::PrefetchWrite => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemorySpaceSet::ANY);
            effect.nosync = true;
            effect
        }
        _ => mir::MemoryEffect::none(),
    }
}

/// Apply stack address space metadata to a memory effect.
fn stack_effect(mut effect: mir::MemoryEffect) -> mir::MemoryEffect {
    effect.address_spaces = Some(mir::AddressSpaceSet::new(vec![mir::AddressSpace::Stack]));
    effect.nosync = true;
    effect
}

/// Apply heap region metadata to a memory effect.
fn heap_effect(mut effect: mir::MemoryEffect) -> mir::MemoryEffect {
    effect.nosync = true;
    effect
}

/// Build a call behavior for an allocation effect.
fn alloc_behavior(
    spaces: mir::MemorySpaceSet,
    address_space: Option<mir::AddressSpace>,
) -> mir::CallBehavior {
    let mut behavior = mir::CallBehavior::none();
    behavior.allocation.allocate = Some(mir::AllocationAccess {
        spaces,
        address_spaces: address_space.map(|space| mir::AddressSpaceSet::new(vec![space])),
    });
    behavior
}

/// Build a call behavior for a free effect.
fn free_behavior(
    spaces: mir::MemorySpaceSet,
    address_space: Option<mir::AddressSpace>,
) -> mir::CallBehavior {
    let mut behavior = mir::CallBehavior::none();
    behavior.allocation.free = Some(mir::AllocationAccess {
        spaces,
        address_spaces: address_space.map(|space| mir::AddressSpaceSet::new(vec![space])),
    });
    behavior
}

/// Build an inaccessible write effect.
fn inaccessible_write_effect() -> mir::MemoryEffect {
    mir::MemoryEffect::write_only(mir::MemorySpaceSet::NONE).with_inaccessible_mem_only()
}

/// Merge two optional memory space sets.
fn merge_space_set(
    left: Option<mir::MemorySpaceSet>,
    right: Option<mir::MemorySpaceSet>,
) -> Option<mir::MemorySpaceSet> {
    // merge space sets conservatively
    match (left, right) {
        (Some(mut left), Some(right)) => {
            left.insert(right);
            Some(left)
        }
        _ => None,
    }
}

/// Merge two optional address space sets.
fn merge_address_space_set(
    left: Option<mir::AddressSpaceSet>,
    right: Option<mir::AddressSpaceSet>,
) -> Option<mir::AddressSpaceSet> {
    // merge address space sets conservatively
    match (left, right) {
        (Some(mut left), Some(right)) => {
            for space in right.spaces {
                if !left.contains(space.clone()) {
                    left.spaces.push(space);
                }
            }
            Some(left)
        }
        _ => None,
    }
}

/// Merge two optional allocation access summaries.
fn merge_allocation_access(
    left: Option<mir::AllocationAccess>,
    right: Option<mir::AllocationAccess>,
) -> Option<mir::AllocationAccess> {
    match (left, right) {
        (Some(left), Some(right)) => Some(mir::AllocationAccess {
            spaces: merge_space_set(Some(left.spaces), Some(right.spaces))
                .unwrap_or(mir::MemorySpaceSet::ANY),
            address_spaces: merge_address_space_set(left.address_spaces, right.address_spaces),
        }),
        (Some(access), None) | (None, Some(access)) => Some(access),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Functions without memory effects are marked as readnone.
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
        let function = test.tree.get(function_id);

        assert_eq!(function.memory_effect, mir::MemoryEffect::none());
    }

    /// Allocation and free instructions are surfaced in call behavior.
    #[test]
    fn test_function_attrs_alloc_behavior() {
        let input = r#"
function alloc(): void {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    raw.free v0
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("alloc");
        let function = test.tree.get(function_id);
        let behavior = function.call_behavior.clone();

        assert!(behavior.allocation.allocate.is_some());
        assert!(behavior.allocation.free.is_some());
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
        let instruction = test.tree.get_mut(call_inst);
        let Some(memory_effect) = instruction.call_memory_effect_mut() else {
            panic!("expected call instruction");
        };
        *memory_effect = Some(mir::MemoryEffect::none());

        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let instruction = test.tree.get(call_inst);
        assert_eq!(
            instruction.call_memory_effect(),
            Some(&mir::MemoryEffect::none())
        );
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
        let caller = test.tree.get(caller_id);
        let behavior = caller.call_behavior.clone();

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
        let caller = test.tree.get(caller_id);
        let behavior = caller.call_behavior.clone();

        assert_eq!(behavior.return_behavior, mir::ReturnBehavior::NoReturn);
    }

    /// Unknown indirect calls produce unknown memory effects.
    #[test]
    fn test_function_attrs_unknown_indirect_effects() {
        let input = r#"
function callee(v0: fn(int32) -> int32, v1: int32): int32  {
b0(v0: fn(int32) -> int32, v1: int32) -> v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("callee");
        let function = test.tree.get(function_id);
        let effects = &function.memory_effect;

        assert!(effects.reads);
        assert!(effects.writes);
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
    trap.panic v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let caller_id = test.function_id_by_name("caller");
        let caller = test.tree.get(caller_id);

        assert!(caller.memory_effect.writes);
    }
}
