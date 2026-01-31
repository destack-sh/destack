use std::collections::{HashMap, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::CallGraph;
use crate::optimize::{AnalysisPreservation, ModuleAnalyses, ModulePass, PipelineContext};

declare_pass! {
    /// Infer memory and behavior attributes for functions and callsites.
    ///
    /// This pass aggregates memory effects, allocation behavior, and convergence from instruction semantics and direct callees.
    /// This pass fills missing metadata and rederives summaries after IR changes.
    ///
    /// ```mir
    /// function @pure(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iadd v0, v0
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @pure(v0: i32) -> i32 { ... } // memory_effects = none
    /// ```
    #[pass(id = "function-attrs")]
    pub FunctionAttrs,
    "Infer function attributes"
}

impl ModulePass for FunctionAttrs {
    /// Run the function attribute inference pass.
    fn run(&self, tree: &mut mir::NodeTree, _ctx: &PipelineContext<'_>) -> AnalysisPreservation {
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
    /// The aggregate memory location set.
    locations: mir::MemoryLocationSet,
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
            locations: mir::MemoryLocationSet::NONE,
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
        self.locations.insert(effect.locations);
        self.argmemonly &= effect.argmemonly;
        self.inaccessible_mem_only &= effect.inaccessible_mem_only;
        self.nosync &= effect.nosync;

        // merge address space sets when both are available
        match (&mut self.address_spaces, &effect.address_spaces) {
            (Some(existing), Some(other)) => {
                // append missing address spaces
                for space in &other.spaces {
                    if !existing.contains(*space) {
                        existing.spaces.push(*space);
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
            locations: self.locations,
            address_spaces: self.address_spaces,
            argmemonly: self.argmemonly,
            inaccessible_mem_only: self.inaccessible_mem_only,
            nosync: self.nosync,
        }
    }
}

/// Builder for call behavior summaries.
struct CallBehaviorBuilder {
    /// Repeatability classification for the function.
    repeatability: mir::Repeatability,
    /// Whether any callee may suspend execution.
    may_suspend: bool,
    /// Whether any callee forbids reordering or duplication.
    no_reorder: bool,
    /// Whether any callee forbids deoptimization across the call.
    no_deopt_across: bool,
    /// Whether any callee forbids replay mode.
    no_replay: bool,
    /// Whether any callee is convergent.
    convergent: bool,
    /// Whether any callee allocates.
    allocates: bool,
    /// The aggregate allocation location set when available.
    alloc_locations: Option<mir::MemoryLocationSet>,
    /// The aggregate allocation address space set when available.
    alloc_address_spaces: Option<mir::AddressSpaceSet>,
    /// Whether any callee frees memory.
    frees: bool,
    /// The aggregate free location set when available.
    free_locations: Option<mir::MemoryLocationSet>,
    /// The aggregate free address space set when available.
    free_address_spaces: Option<mir::AddressSpaceSet>,
}

impl CallBehaviorBuilder {
    /// Create a new behavior builder.
    fn new() -> Self {
        // seed the builder with no behavior flags
        Self {
            repeatability: mir::Repeatability::Repeatable,
            may_suspend: false,
            no_reorder: false,
            no_deopt_across: false,
            no_replay: false,
            convergent: false,
            allocates: false,
            alloc_locations: Some(mir::MemoryLocationSet::NONE),
            alloc_address_spaces: Some(mir::AddressSpaceSet::new(Vec::new())),
            frees: false,
            free_locations: Some(mir::MemoryLocationSet::NONE),
            free_address_spaces: Some(mir::AddressSpaceSet::new(Vec::new())),
        }
    }

    /// Record a call behavior into the builder.
    fn record_behavior(&mut self, behavior: &mir::CallBehavior) {
        // merge repeatability and barrier flags
        if behavior.repeatability == mir::Repeatability::NonRepeatable {
            self.repeatability = mir::Repeatability::NonRepeatable;
        }
        self.may_suspend |= behavior.may_suspend;
        self.no_reorder |= behavior.no_reorder;
        self.no_deopt_across |= behavior.no_deopt_across;
        self.no_replay |= behavior.no_replay;

        // merge convergence information
        self.convergent |= behavior.convergent;

        // merge allocation information
        if behavior.allocates {
            self.allocates = true;
            self.alloc_locations =
                merge_location_set(self.alloc_locations, behavior.alloc_locations);
            let alloc_address_spaces = self.alloc_address_spaces.take();
            self.alloc_address_spaces = merge_address_space_set(
                alloc_address_spaces,
                behavior.alloc_address_spaces.clone(),
            );
        }

        // merge free information
        if behavior.frees {
            self.frees = true;
            self.free_locations = merge_location_set(self.free_locations, behavior.free_locations);
            let free_address_spaces = self.free_address_spaces.take();
            self.free_address_spaces =
                merge_address_space_set(free_address_spaces, behavior.free_address_spaces.clone());
        }
    }

    /// Finish the builder into a call behavior summary.
    fn finish(self, noreturn: bool) -> mir::CallBehavior {
        // assemble the final call behavior summary
        mir::CallBehavior {
            repeatability: self.repeatability,
            may_suspend: self.may_suspend,
            no_reorder: self.no_reorder,
            no_deopt_across: self.no_deopt_across,
            no_replay: self.no_replay,
            noreturn,
            will_return: false,
            convergent: self.convergent,
            allocates: self.allocates,
            alloc_locations: self.allocates.then_some(self.alloc_locations).flatten(),
            alloc_address_spaces: self
                .allocates
                .then_some(self.alloc_address_spaces)
                .flatten(),
            frees: self.frees,
            free_locations: self.frees.then_some(self.free_locations).flatten(),
            free_address_spaces: self.frees.then_some(self.free_address_spaces).flatten(),
        }
    }
}

/// Run function attribute inference over the module.
fn run_function_attrs(tree: &mut mir::NodeTree) -> bool {
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
                merge_memory_effect(function.memory_effects.as_ref(), &summary.memory_effects),
                merge_call_behavior(function.call_behavior.as_ref(), &summary.call_behavior),
            )
        };

        // write back any updated attributes
        let mut function_changed = false;
        {
            let function = tree.get_mut(function_id);
            if function.memory_effects.as_ref() != Some(&merged_memory) {
                function.memory_effects = Some(merged_memory.clone());
                function_changed = true;
            }
            if function.call_behavior.as_ref() != Some(&merged_behavior) {
                function.call_behavior = Some(merged_behavior.clone());
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
    tree: &mir::NodeTree,
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
        match &block.terminator {
            mir::Terminator::Return { .. } => {
                has_return = true;
            }
            mir::Terminator::TailCall { function, .. } => {
                let (effect, behavior) = call_effects_for_tailcall(tree, *function, summaries);
                memory_builder.record_effect(&effect);
                behavior_builder.record_behavior(&behavior);
                if !behavior.noreturn {
                    has_return = true;
                }
            }
            mir::Terminator::TailCallIndirect { .. } => {
                let effect = mir::MemoryEffect::unknown();
                let behavior = mir::CallBehavior::unknown();
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
    let noreturn = function.coroutine.is_none() && !has_return;
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

    // union the memory effect flags and sets
    let mut merged = existing.clone();
    merged.reads |= inferred.reads;
    merged.writes |= inferred.writes;
    merged.locations.insert(inferred.locations);
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

    // union behavioral flags and location sets
    let mut merged = existing.clone();
    merged.repeatability = match (merged.repeatability, inferred.repeatability) {
        (mir::Repeatability::NonRepeatable, _) | (_, mir::Repeatability::NonRepeatable) => {
            mir::Repeatability::NonRepeatable
        }
        (mir::Repeatability::Pure, mir::Repeatability::Pure) => mir::Repeatability::Pure,
        _ => mir::Repeatability::Repeatable,
    };
    merged.may_suspend |= inferred.may_suspend;
    merged.no_reorder |= inferred.no_reorder;
    merged.no_deopt_across |= inferred.no_deopt_across;
    merged.no_replay |= inferred.no_replay;
    merged.noreturn |= inferred.noreturn;
    merged.convergent |= inferred.convergent;
    merged.allocates |= inferred.allocates;
    merged.frees |= inferred.frees;
    merged.alloc_locations = merge_location_set(merged.alloc_locations, inferred.alloc_locations);
    merged.free_locations = merge_location_set(merged.free_locations, inferred.free_locations);
    let alloc_address_spaces = merged.alloc_address_spaces.take();
    merged.alloc_address_spaces =
        merge_address_space_set(alloc_address_spaces, inferred.alloc_address_spaces.clone());
    let free_address_spaces = merged.free_address_spaces.take();
    merged.free_address_spaces =
        merge_address_space_set(free_address_spaces, inferred.free_address_spaces.clone());

    merged
}

/// Update call metadata entries with inferred callee summaries.
fn update_call_metadata(
    tree: &mut mir::NodeTree,
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
            let Some(metadata) = instruction.call_effects_mut() else {
                continue;
            };
            if metadata.memory_effects.is_none() {
                metadata.memory_effects = Some(summary.memory_effects.clone());
                changed = true;
            }
            if metadata.behavior.is_none() {
                metadata.behavior = Some(summary.call_behavior.clone());
                changed = true;
            }
        }
    }

    changed
}

/// Resolve call effects for a call instruction.
fn call_effects_for_instruction(
    _tree: &mir::NodeTree,
    instruction: &mir::Instruction,
    summaries: &HashMap<mir::LocalNodeId<mir::Function>, FunctionSummary>,
) -> Option<(mir::MemoryEffect, mir::CallBehavior)> {
    // skip non call instructions
    let is_call = matches!(
        instruction,
        mir::Instruction::Call { .. }
            | mir::Instruction::CallVirtual { .. }
            | mir::Instruction::CallInterface { .. }
            | mir::Instruction::CallIndirect { .. }
    );
    if !is_call {
        return None;
    }

    // seed effects from call metadata when present
    let metadata = instruction.call_effects();
    let mut memory_effect = metadata.and_then(|meta| meta.memory_effects.clone());
    let mut behavior = metadata.and_then(|meta| meta.behavior.clone());

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

/// Resolve call effects for a tail call terminator.
fn call_effects_for_tailcall(
    _tree: &mir::NodeTree,
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

/// Resolve a direct callee id for a call instruction.
fn direct_callee_for_instruction(
    instruction: &mir::Instruction,
) -> Option<mir::LocalNodeId<mir::Function>> {
    match instruction {
        mir::Instruction::Call { function, .. } => Some(*function),
        _ => None,
    }
}

/// Compute effects for non call instructions.
fn effects_for_instruction(
    tree: &mir::NodeTree,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
) -> (mir::MemoryEffect, mir::CallBehavior) {
    // prefer explicit memory access metadata when present
    if let Some(accesses) = tree.memory_table.memory_accesses(instruction_id) {
        let effect = memory_effect_from_accesses(accesses);
        return (effect, mir::CallBehavior::none());
    }

    // map instruction semantics to effect summaries
    match instruction {
        mir::Instruction::Load { .. } => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemoryLocationSet::ANY);
            effect.nosync = true;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::Store { .. } => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemoryLocationSet::ANY);
            effect.nosync = true;
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::LocalGet { .. } => {
            let effect = stack_effect(mir::MemoryEffect::read_only(mir::MemoryLocationSet::STACK));
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::LocalSet { .. } => {
            let effect = stack_effect(mir::MemoryEffect::write_only(mir::MemoryLocationSet::STACK));
            (effect, mir::CallBehavior::none())
        }
        mir::Instruction::ManagedAlloc { .. } | mir::Instruction::ManagedAllocArray { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemoryLocationSet::HEAP));
            let behavior =
                alloc_behavior(mir::MemoryLocationSet::HEAP, Some(mir::AddressSpace::Heap));
            (effect, behavior)
        }
        mir::Instruction::RawAlloc { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemoryLocationSet::HEAP));
            let behavior =
                alloc_behavior(mir::MemoryLocationSet::HEAP, Some(mir::AddressSpace::Heap));
            (effect, behavior)
        }
        mir::Instruction::RawFree { .. } | mir::Instruction::RawDrop { .. } => {
            let effect = heap_effect(mir::MemoryEffect::write_only(mir::MemoryLocationSet::HEAP));
            let behavior =
                free_behavior(mir::MemoryLocationSet::HEAP, Some(mir::AddressSpace::Heap));
            (effect, behavior)
        }
        mir::Instruction::StackAlloc { .. } | mir::Instruction::StackDrop { .. } => {
            let effect = stack_effect(mir::MemoryEffect::write_only(mir::MemoryLocationSet::STACK));
            (effect, mir::CallBehavior::none())
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
            mir::MemoryEffect::read_only(mir::MemoryLocationSet::ANY)
        }
        mir::MemoryAccessKind::Write | mir::MemoryAccessKind::PrefetchWrite => {
            mir::MemoryEffect::write_only(mir::MemoryLocationSet::ANY)
        }
        mir::MemoryAccessKind::ReadWrite
        | mir::MemoryAccessKind::ReadModifyWrite
        | mir::MemoryAccessKind::Fence => {
            mir::MemoryEffect::read_write(mir::MemoryLocationSet::ANY)
        }
    };

    // apply ordering and address space annotations
    effect.nosync = true;
    if let Some(space) = access.address_space {
        effect.address_spaces = Some(mir::AddressSpaceSet::new(vec![space]));
    }
    if access.is_volatile
        || access.ordering.is_some()
        || access.semantics.is_some()
        || access.scope.is_some()
        || access.memory_scope.is_some()
        || access.kind == mir::MemoryAccessKind::Fence
    {
        effect.nosync = false;
    }

    // refine location sets for locals and globals
    match access.target {
        mir::MemoryAccessTarget::Local(_) => {
            effect.locations = mir::MemoryLocationSet::STACK;
        }
        mir::MemoryAccessTarget::Global(_) => {
            effect.locations = mir::MemoryLocationSet::GLOBAL;
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
            let mut effect = mir::MemoryEffect::read_write(mir::MemoryLocationSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::Memset => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemoryLocationSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::Memcmp => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemoryLocationSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::VolatileLoad => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemoryLocationSet::ANY);
            effect.nosync = false;
            effect
        }
        Intrinsic::VolatileStore => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemoryLocationSet::ANY);
            effect.nosync = false;
            effect
        }
        Intrinsic::PrefetchRead | Intrinsic::PrefetchWrite => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemoryLocationSet::ANY);
            effect.nosync = true;
            effect
        }
        Intrinsic::GcWriteBarrier => {
            let mut effect = mir::MemoryEffect::write_only(mir::MemoryLocationSet::INACCESSIBLE);
            effect.nosync = true;
            effect
        }
        Intrinsic::AtomicLoad => {
            let mut effect = mir::MemoryEffect::read_only(mir::MemoryLocationSet::ANY);
            effect.nosync = false;
            effect
        }
        Intrinsic::AtomicStore
        | Intrinsic::AtomicCas
        | Intrinsic::AtomicFetchAdd
        | Intrinsic::AtomicFetchSub
        | Intrinsic::AtomicFetchAnd
        | Intrinsic::AtomicFetchOr
        | Intrinsic::AtomicFetchXor
        | Intrinsic::AtomicFetchMin
        | Intrinsic::AtomicFetchMax => {
            let mut effect = mir::MemoryEffect::read_write(mir::MemoryLocationSet::ANY);
            effect.nosync = false;
            effect
        }
        Intrinsic::AtomicFence => {
            let mut effect = mir::MemoryEffect::read_write(mir::MemoryLocationSet::ANY);
            effect.nosync = false;
            effect
        }
        Intrinsic::Barrier => {
            let mut effect = mir::MemoryEffect::read_write(mir::MemoryLocationSet::ANY);
            effect.nosync = false;
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

/// Apply heap address space metadata to a memory effect.
fn heap_effect(mut effect: mir::MemoryEffect) -> mir::MemoryEffect {
    effect.address_spaces = Some(mir::AddressSpaceSet::new(vec![mir::AddressSpace::Heap]));
    effect.nosync = true;
    effect
}

/// Build a call behavior for an allocation effect.
fn alloc_behavior(
    locations: mir::MemoryLocationSet,
    address_space: Option<mir::AddressSpace>,
) -> mir::CallBehavior {
    let mut behavior = mir::CallBehavior::none();
    behavior.allocates = true;
    behavior.alloc_locations = Some(locations);
    behavior.alloc_address_spaces =
        address_space.map(|space| mir::AddressSpaceSet::new(vec![space]));
    behavior
}

/// Build a call behavior for a free effect.
fn free_behavior(
    locations: mir::MemoryLocationSet,
    address_space: Option<mir::AddressSpace>,
) -> mir::CallBehavior {
    let mut behavior = mir::CallBehavior::none();
    behavior.frees = true;
    behavior.free_locations = Some(locations);
    behavior.free_address_spaces =
        address_space.map(|space| mir::AddressSpaceSet::new(vec![space]));
    behavior
}

/// Merge two optional memory location sets.
fn merge_location_set(
    left: Option<mir::MemoryLocationSet>,
    right: Option<mir::MemoryLocationSet>,
) -> Option<mir::MemoryLocationSet> {
    // merge location sets conservatively
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
                if !left.contains(space) {
                    left.spaces.push(space);
                }
            }
            Some(left)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Functions without memory effects are marked as readnone.
    #[test]
    fn test_function_attrs_pure() {
        let input = r#"function @pure(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("pure");
        let function = test.tree.get(function_id);

        assert_eq!(function.memory_effects, Some(mir::MemoryEffect::none()));
        assert!(function.call_behavior.is_some());
    }

    /// Allocation and free instructions are surfaced in call behavior.
    #[test]
    fn test_function_attrs_alloc_behavior() {
        let input = r#"function @alloc() -> void {
block0:
    v0: ref<raw i32> = raw.alloc i32
    raw.free v0
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("alloc");
        let function = test.tree.get(function_id);
        let behavior = function.call_behavior.clone().expect("missing behavior");

        assert!(behavior.allocates);
        assert!(behavior.frees);
    }

    /// Call metadata is populated from callee summaries.
    #[test]
    fn test_function_attrs_updates_call_metadata() {
        let input = r#"function @callee() -> i32 {
block0:
    v0: i32 = iconst 1i32
    return v0
}
function @caller() -> i32 {
block0:
    v0: i32 = call @callee() -> fn() -> i32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let caller_id = test.function_id_by_name("caller");
        let (call_inst, _callee_id) = test.first_call_in_entry(caller_id);
        let instruction = test.tree.get_mut(call_inst);
        let mir::Instruction::Call { effects, .. } = instruction else {
            panic!("expected call instruction");
        };
        *effects = Some(mir::CallEffects::default());

        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let instruction = test.tree.get(call_inst);
        let effects = instruction.call_effects().expect("missing call effects");
        assert_eq!(effects.memory_effects, Some(mir::MemoryEffect::none()));
    }

    /// Tail calls to returning functions do not imply noreturn.
    #[test]
    fn test_function_attrs_tailcall_returns() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    tailcall @callee(v0)
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let caller_id = test.function_id_by_name("caller");
        let caller = test.tree.get(caller_id);
        let behavior = caller.call_behavior.clone().expect("missing behavior");

        assert!(!behavior.noreturn);
    }

    /// Tail calls to noreturn callees propagate noreturn.
    #[test]
    fn test_function_attrs_tailcall_noreturn() {
        let input = r#"function @sink() -> void {
block0:
    jump block0
}
function @caller() -> void {
block0:
    tailcall @sink()
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let caller_id = test.function_id_by_name("caller");
        let caller = test.tree.get(caller_id);
        let behavior = caller.call_behavior.clone().expect("missing behavior");

        assert!(behavior.noreturn);
    }

    /// Unknown indirect calls produce unknown memory effects.
    #[test]
    fn test_function_attrs_unknown_indirect_effects() {
        let input = r#"function @callee(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    v2: i32 = call.indirect v0(v1) -> fn(i32) -> i32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&FunctionAttrs);
        test.assert_output(input);
        let function_id = test.function_id_by_name("callee");
        let function = test.tree.get(function_id);
        let effects = function.memory_effects.as_ref().expect("missing effects");

        assert!(effects.reads);
        assert!(effects.writes);
    }
}
