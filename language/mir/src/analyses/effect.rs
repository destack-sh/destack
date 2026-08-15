use std::collections::{HashMap, VecDeque};

use crate as mir;

use super::{Analysis, AnalysisCache, Mutation};

impl mir::EffectTable {
    /// Build complete effects for all functions with bodies.
    fn build(
        tree: &mir::Tree,
        analyses: &mut AnalysisCache,
        accesses: &mir::AccessTable,
        effect_table: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
    ) -> Self {
        let resolution = analyses.resolution(tree, dispatch);
        let calls = analyses.call(tree, dispatch);
        let function_ids = Self::function_body_ids(tree);
        let mut effects = effect_table
            .functions()
            .map(|(function, effect)| (function, effect.clone()))
            .collect::<HashMap<_, _>>();
        let mut worklist: VecDeque<_> = function_ids.iter().copied().collect();

        // propagate direct-call effects to a fixpoint
        while let Some(function_id) = worklist.pop_front() {
            let effect = FunctionEffectBuilder::compute(
                tree,
                function_id,
                accesses,
                effect_table,
                &resolution,
                &effects,
            );
            let changed = effects
                .get(&function_id)
                .map(|existing| existing != &effect)
                .unwrap_or(true);

            if changed {
                effects.insert(function_id, effect);

                for edge in calls.incoming(function_id) {
                    worklist.push_back(edge.caller);
                }
            }
        }

        let mut table = effect_table.clone();
        table.replace_functions(effects);

        table
    }

    /// Return ids for all functions with bodies.
    fn function_body_ids(tree: &mir::Tree) -> Vec<mir::FunctionId> {
        tree.iter_nodes::<mir::Function>()
            .filter_map(|(id, function)| function.entry().is_some().then_some(id))
            .collect()
    }
}

impl Analysis for mir::EffectTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::MEMORY)
        .union(Mutation::EFFECT);
}

impl mir::EffectTable {
    /// Compute complete module effects.
    pub(crate) fn compute(
        tree: &mir::Tree,
        analyses: &mut AnalysisCache,
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
    ) -> Self {
        Self::build(tree, analyses, accesses, effects, dispatch)
    }
}

/// Builder state for one function effect.
struct FunctionEffectBuilder<'a> {
    /// The MIR tree being analyzed.
    tree: &'a mir::Tree,
    /// The function being analyzed.
    function: &'a mir::Function,
    /// Explicit memory access table.
    accesses: &'a mir::AccessTable,
    /// Explicit effect table.
    effect_table: &'a mir::EffectTable,
    /// Static callsite resolutions.
    resolution: &'a mir::ResolutionTable,
    /// Effects available from previous fixpoint iterations.
    effects: &'a HashMap<mir::FunctionId, mir::FunctionEffect>,
    /// Accumulated memory effect.
    memory: MemoryAccumulator,
    /// Accumulated behavior effect.
    behavior: BehaviorAccumulator,
    /// Whether the function may return normally.
    has_return: bool,
}

impl<'a> FunctionEffectBuilder<'a> {
    /// Compute the current effect for one function.
    fn compute(
        tree: &'a mir::Tree,
        function_id: mir::FunctionId,
        accesses: &'a mir::AccessTable,
        effect_table: &'a mir::EffectTable,
        resolution: &'a mir::ResolutionTable,
        effects: &'a HashMap<mir::FunctionId, mir::FunctionEffect>,
    ) -> mir::FunctionEffect {
        let function = tree.get(function_id);
        let mut builder = Self {
            tree,
            function,
            accesses,
            effect_table,
            resolution,
            effects,
            memory: MemoryAccumulator::new(),
            behavior: BehaviorAccumulator::new(),
            has_return: false,
        };

        // scan every block for instruction and terminator effects
        for &block_id in function.blocks() {
            builder.record_block(block_id);
        }

        // finish function behavior from reachable return observations
        let noreturn = !builder.has_return;
        mir::FunctionEffect {
            memory: builder.memory.finish(),
            behavior: builder.behavior.finish(noreturn),
        }
    }

    /// Record all effects in one block.
    fn record_block(&mut self, block_id: mir::BlockId) {
        let block = self.tree.get(block_id);

        // record instruction effects
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            let effect = self.instruction_effect(instruction_id, instruction);
            self.record_effect(&effect);
        }

        // record terminator effects
        let terminator = self.tree.get(block.terminator);
        let effect = self.terminator_effect(block_id, terminator);
        self.record_effect(&effect);
    }

    /// Record a child effect into this function effect.
    fn record_effect(&mut self, effect: &mir::FunctionEffect) {
        self.memory.record(&effect.memory);
        self.behavior.record(&effect.behavior);
    }

    /// Build an effect for one instruction.
    fn instruction_effect(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> mir::FunctionEffect {
        if let Some(effect) = self.call_instruction_effect(instruction_id, instruction) {
            return effect;
        }

        self.non_call_instruction_effect(instruction_id, instruction)
    }

    /// Build an effect for one call instruction.
    fn call_instruction_effect(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> Option<mir::FunctionEffect> {
        instruction.call_dispatch()?;

        let callsite = mir::CallSite::Instruction(instruction_id);
        let callee = instruction.call_direct_target();

        Some(self.callsite_effect(callsite, callee))
    }

    /// Build an effect for one non-call instruction.
    fn non_call_instruction_effect(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> mir::FunctionEffect {
        // prefer precise memory access entries when present
        if let Some(accesses) = self.accesses.get(instruction_id) {
            let memory = MemoryAccumulator::from_accesses(accesses).finish();
            return mir::FunctionEffect {
                memory,
                behavior: mir::FunctionBehavior::none(),
            };
        }

        // map MIR operations to local effects
        match instruction {
            mir::Instruction::Load { .. } | mir::Instruction::AtomicLoad { .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::read_only(mir::StorageSet::ANY))
            }
            mir::Instruction::Store { .. } | mir::Instruction::AtomicStore { .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::write_only(mir::StorageSet::ANY))
            }
            mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::read_write(mir::StorageSet::ANY))
            }
            mir::Instruction::BarrierWrite { .. } => mir::FunctionEffect::none(),
            mir::Instruction::LocalGet { .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::read_only(mir::StorageSet::FRAME))
            }
            mir::Instruction::LocalSet { .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::write_only(mir::StorageSet::FRAME))
            }
            mir::Instruction::NewZeroed { result_type, .. }
            | mir::Instruction::NewUninit { result_type, .. }
            | mir::Instruction::NewSliceZeroed { result_type, .. }
            | mir::Instruction::NewSliceUninit { result_type, .. } => mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(self.storage_set_for_type(result_type)),
                behavior: mir::FunctionBehavior::none().with_allocates(),
            },
            mir::Instruction::Free { value } => mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(self.storage_set_for_value(*value)),
                behavior: mir::FunctionBehavior::none().with_frees(),
            },
            mir::Instruction::Drop { .. } => mir::FunctionEffect {
                memory: mir::MemoryEffect::unknown(),
                behavior: mir::FunctionBehavior::none().with_frees(),
            },
            mir::Instruction::Pin { .. } | mir::Instruction::Unpin { .. } => {
                mir::FunctionEffect::unknown()
            }
            mir::Instruction::Intrinsic { intrinsic, .. } => self.intrinsic_effect(*intrinsic),
            mir::Instruction::Breakpoint => mir::FunctionEffect {
                memory: mir::MemoryEffect::none(),
                behavior: mir::FunctionBehavior::none().with_preserved_execution(),
            },
            _ => mir::FunctionEffect::none(),
        }
    }

    /// Build an effect for one terminator.
    fn terminator_effect(
        &mut self,
        block_id: mir::BlockId,
        terminator: &mir::Terminator,
    ) -> mir::FunctionEffect {
        match terminator {
            mir::Terminator::Return { .. } => {
                self.has_return = true;
                mir::FunctionEffect::none()
            }
            mir::Terminator::Invoke { .. } | mir::Terminator::TailCall { .. } => {
                let callsite = mir::CallSite::Terminator(block_id);
                let effect = self.callsite_effect(callsite, terminator.call_direct_target());

                if !effect.behavior.return_behavior.is_no_return() {
                    self.has_return = true;
                }

                effect
            }
            mir::Terminator::Panic { .. } | mir::Terminator::UnwindResume => mir::FunctionEffect {
                memory: mir::MemoryEffect::none(),
                behavior: mir::FunctionBehavior::none().with_panic().with_noreturn(),
            },
            _ => mir::FunctionEffect::none(),
        }
    }

    /// Build an effect for one callsite.
    fn callsite_effect(
        &self,
        callsite: mir::CallSite,
        callee: Option<mir::FunctionId>,
    ) -> mir::FunctionEffect {
        // seed effects from explicit call tables
        let tables = self.effect_table.call(callsite);
        let memory = tables
            .filter(|tables| tables.memory != mir::MemoryEffect::unknown())
            .map(|tables| tables.memory.clone());
        let behavior = tables
            .filter(|tables| tables.behavior != mir::FunctionBehavior::unknown())
            .map(|tables| tables.behavior.clone());

        // fill missing effects from the best known direct target
        let callee = callee.or_else(|| self.resolution.target(callsite));

        self.call_effect(callee, memory, behavior)
    }

    /// Build a call effect from explicit tables and callee effects.
    fn call_effect(
        &self,
        callee: Option<mir::FunctionId>,
        memory: Option<mir::MemoryEffect>,
        behavior: Option<mir::FunctionBehavior>,
    ) -> mir::FunctionEffect {
        let callee_effect = callee.and_then(|callee| self.effects.get(&callee));
        let memory = memory
            .or_else(|| callee_effect.map(|effect| effect.memory.clone()))
            .unwrap_or_else(mir::MemoryEffect::unknown);
        let behavior = behavior
            .or_else(|| callee_effect.map(|effect| effect.behavior.clone()))
            .unwrap_or_else(mir::FunctionBehavior::unknown);

        mir::FunctionEffect { memory, behavior }
    }

    /// Resolve the backing storage for one typed value.
    fn storage_set_for_value(&self, value: mir::Value) -> mir::StorageSet {
        let Some(ty) = self.function.value_type(value) else {
            return mir::StorageSet::ANY;
        };

        self.storage_set_for_type(&mir::TypeId::from(ty))
    }

    /// Resolve the backing storage for one reference-like type.
    fn storage_set_for_type(&self, ty: &mir::TypeId) -> mir::StorageSet {
        match self.tree.get(*ty) {
            mir::Type::Uninit { value } => self.storage_set_for_type(value),
            ty => ty
                .reference_storage()
                .map_or(mir::StorageSet::ANY, mir::Storage::storage_set),
        }
    }

    /// Build a memory effect for an intrinsic.
    fn intrinsic_memory(&self, intrinsic: mir::Intrinsic) -> mir::MemoryEffect {
        match intrinsic {
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => {
                mir::MemoryEffect::read_write(mir::StorageSet::ANY)
            }
            mir::Intrinsic::Memset => mir::MemoryEffect::write_only(mir::StorageSet::ANY),
            mir::Intrinsic::Memcmp => mir::MemoryEffect::read_only(mir::StorageSet::ANY),
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
                mir::MemoryEffect::read_only(mir::StorageSet::ANY)
            }
            _ => mir::MemoryEffect::none(),
        }
    }

    /// Build the memory and behavioral effects for one intrinsic.
    fn intrinsic_effect(&self, intrinsic: mir::Intrinsic) -> mir::FunctionEffect {
        let memory = self.intrinsic_memory(intrinsic);
        let behavior = if matches!(
            intrinsic,
            mir::Intrinsic::SpinLoop | mir::Intrinsic::BlackBox
        ) {
            mir::FunctionBehavior::none().with_preserved_execution()
        } else {
            mir::FunctionBehavior::none()
        };

        mir::FunctionEffect { memory, behavior }
    }
}

/// Accumulated memory effects.
struct MemoryAccumulator {
    /// Memory spaces read by this function.
    read: mir::StorageSet,
    /// Memory spaces written by this function.
    write: mir::StorageSet,
}

impl MemoryAccumulator {
    /// Create a new empty memory accumulator.
    fn new() -> Self {
        Self {
            read: mir::StorageSet::NONE,
            write: mir::StorageSet::NONE,
        }
    }

    /// Build a memory accumulator from memory accesses.
    fn from_accesses(accesses: &[mir::MemoryAccess]) -> Self {
        let mut builder = Self::new();
        for access in accesses {
            builder.record_access(access);
        }

        builder
    }

    /// Record a memory effect into the builder.
    fn record(&mut self, effect: &mir::MemoryEffect) {
        self.read.insert(effect.read);
        self.write.insert(effect.write);
    }

    /// Record one memory access.
    fn record_access(&mut self, access: &mir::MemoryAccess) {
        let effect = self.access_effect(access);
        self.record(&effect);
    }

    /// Build a memory effect for one access.
    fn access_effect(&self, access: &mir::MemoryAccess) -> mir::MemoryEffect {
        let mut effect = match access.operation {
            mir::MemoryOperation::Read => mir::MemoryEffect::read_only(mir::StorageSet::ANY),
            mir::MemoryOperation::Write => mir::MemoryEffect::write_only(mir::StorageSet::ANY),
            mir::MemoryOperation::ReadWrite => mir::MemoryEffect::read_write(mir::StorageSet::ANY),
        };

        // refine storage-specific targets
        match access.target {
            mir::MemoryTarget::Local(_) => {
                effect = effect.with_storage(mir::StorageSet::FRAME);
            }
            mir::MemoryTarget::Global(_) => {
                effect = effect.with_storage(mir::StorageSet::GLOBAL);
            }
            _ => {}
        }

        effect
    }

    /// Finish the builder into a memory effect.
    fn finish(self) -> mir::MemoryEffect {
        if !self.read.is_empty() || !self.write.is_empty() {
            mir::MemoryEffect {
                read: self.read,
                write: self.write,
            }
        } else {
            mir::MemoryEffect::none()
        }
    }
}

/// Accumulated function behavior.
struct BehaviorAccumulator {
    /// Determinism for the function.
    determinism: mir::Determinism,
    /// Whether execution may panic.
    may_panic: bool,
    /// Whether execution may park the current fiber.
    may_park: bool,
    /// Whether each execution of any operation must be preserved.
    must_preserve_execution: bool,
    /// Whether any callee allocates.
    allocates: bool,
    /// Whether any callee frees memory.
    frees: bool,
}

impl BehaviorAccumulator {
    /// Create a new behavior accumulator.
    fn new() -> Self {
        Self {
            determinism: mir::Determinism::Deterministic,
            may_panic: false,
            may_park: false,
            must_preserve_execution: false,
            allocates: false,
            frees: false,
        }
    }

    /// Record a function behavior into the builder.
    fn record(&mut self, behavior: &mir::FunctionBehavior) {
        if behavior.determinism == mir::Determinism::NonDeterministic {
            self.determinism = mir::Determinism::NonDeterministic;
        }

        self.may_panic |= behavior.panic.may_panic();
        self.may_park |= behavior.park.may_park();
        self.must_preserve_execution |= behavior.must_preserve_execution;
        self.allocates |= behavior.allocates;
        self.frees |= behavior.frees;
    }

    /// Finish the builder into a function behavior.
    fn finish(self, noreturn: bool) -> mir::FunctionBehavior {
        mir::FunctionBehavior {
            determinism: self.determinism,
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
            park: if self.may_park {
                mir::ParkBehavior::MayPark
            } else {
                mir::ParkBehavior::CannotPark
            },
            must_preserve_execution: self.must_preserve_execution,
            allocates: self.allocates,
            frees: self.frees,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as mir;
    use crate::analyses::tests::TestProgram;

    /// Pure arithmetic functions have no memory or behavioral effects.
    #[test]
    fn test_function_effects_mark_pure_function() {
        let program = TestProgram::new(
            r#"
function pure(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    return v1
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("pure");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(effect.memory, mir::MemoryEffect::none());
        assert_eq!(effect.behavior, mir::FunctionBehavior::none());
    }

    /// Allocation and free instructions are surfaced in function behavior.
    #[test]
    fn test_function_effects_mark_allocation_effects() {
        let program = TestProgram::new(
            r#"
function allocate(): void {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    free v0
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("allocate");
        let effect = effects.function(function).expect("missing function effect");

        assert!(effect.behavior.allocates);
        assert!(effect.behavior.frees);
    }

    /// Spin-loop hints require their containing function call to execute.
    #[test]
    fn test_function_effects_preserve_spin_loop_execution() {
        let program = TestProgram::new(
            r#"
function backoff(): void {
entry:
    intrinsic.hint.spinLoop()
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("backoff");
        let effect = effects.function(function).expect("missing function effect");

        assert!(effect.behavior.must_preserve_execution);
    }

    /// Black-box hints require their containing function call to execute.
    #[test]
    fn test_function_effects_preserve_black_box_execution() {
        let program = TestProgram::new(
            r#"
function conceal(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = intrinsic.hint.blackBox(v0)
    return v1
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("conceal");
        let effect = effects.function(function).expect("missing function effect");

        assert!(effect.behavior.must_preserve_execution);
    }

    /// Breakpoints require their containing function call to execute.
    #[test]
    fn test_function_effects_preserve_breakpoint_execution() {
        let program = TestProgram::new(
            r#"
function inspect(): void {
entry:
    breakpoint
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("inspect");
        let effect = effects.function(function).expect("missing function effect");

        assert!(effect.behavior.must_preserve_execution);
    }

    /// Direct calls propagate callee effects to callers.
    #[test]
    fn test_function_effects_propagate_direct_calls() {
        let program = TestProgram::new(
            r#"
function allocate(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    return v0
}

function root(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = call allocate(): () => ref<int32, unique, mutable>
    return v0
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("root");
        let effect = effects.function(function).expect("missing function effect");

        assert!(effect.behavior.allocates);
    }

    /// Resolved virtual calls propagate callee effects to callers.
    #[test]
    fn test_function_effects_propagate_virtual_calls() {
        let mut program = TestProgram::new(
            r#"
function allocate(v0: int32): ref<int32, unique, mutable> {
entry(v0: int32):
    v1: ref<int32, unique, mutable> = new.zeroed int32
    return v1
}

function root(v0: int32): ref<int32, unique, mutable> {
entry(v0: int32):
    v1: ref<int32, unique, mutable> = call.virtual v0, int32, 0(v0): (int32) => ref<int32, unique, mutable>
    return v1
}
"#,
        );
        let allocate = program.function_id_by_name("allocate");
        let root = program.function_id_by_name("root");
        let concrete = program.tree.get(root).parameters[0].ty;
        program.dispatch.insert_virtual_table(mir::VirtualTable {
            concrete,
            methods: vec![allocate],
        });

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let effect = effects.function(root).expect("missing function effect");

        assert!(effect.behavior.allocates);
    }

    /// Direct calls propagate declared effects from bodyless functions.
    #[test]
    fn test_function_effects_propagate_declared_calls() {
        let mut program = TestProgram::new(
            r#"
external function allocate(): void

function root(): void {
entry:
    call allocate(): () => void
    return
}
"#,
        );
        let allocate = program.function_id_by_name("allocate");
        *program.effects.upsert_function(allocate) = mir::FunctionEffect {
            memory: mir::MemoryEffect::unknown(),
            behavior: mir::FunctionBehavior::none().with_allocates(),
        };

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let root = program.function_id_by_name("root");
        let effect = effects.function(root).expect("missing function effect");

        assert!(effect.behavior.allocates);
    }

    /// Direct calls propagate parking behavior from bodyless functions.
    #[test]
    fn test_function_effects_propagate_parking() {
        let mut program = TestProgram::new(
            r#"
external function park(): void

function root(): void {
entry:
    call park(): () => void
    return
}
"#,
        );
        let park = program.function_id_by_name("park");
        *program.effects.upsert_function(park) = mir::FunctionEffect {
            memory: mir::MemoryEffect::none(),
            behavior: mir::FunctionBehavior::none().with_park(),
        };

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let root = program.function_id_by_name("root");
        let effect = effects.function(root).expect("missing function effect");

        assert!(effect.behavior.park.may_park());
    }

    /// Open calls stay unknown until tables or dispatch proves a target.
    #[test]
    fn test_function_effects_mark_open_calls_unknown() {
        let program = TestProgram::new(
            r#"
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("test");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(effect.memory, mir::MemoryEffect::unknown());
        assert_eq!(effect.behavior, mir::FunctionBehavior::unknown());
    }

    /// Panic terminators are may-panic and no-return.
    #[test]
    fn test_function_effects_mark_panic_noreturn() {
        let program = TestProgram::new(
            r#"
function fail(v0: ref<int32, managed, readonly>): void {
entry(v0: ref<int32, managed, readonly>):
    panic v0
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(
            &program.tree,
            &program.accesses,
            &program.effects,
            &program.dispatch,
        );
        let function = program.function_id_by_name("fail");
        let effect = effects.function(function).expect("missing function effect");

        assert!(effect.behavior.panic.may_panic());
        assert!(effect.behavior.return_behavior.is_no_return());
    }
}
