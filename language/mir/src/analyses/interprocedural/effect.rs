use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap;
use tspp_serde::Reflect;

use crate as mir;
use crate::{Analysis, Mutation, is_copy};

/// Local effects and outgoing calls extracted from one function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EffectBody {
    /// Effects produced by the body's own operations.
    pub local: mir::FunctionEffect,
    /// Whether local execution returns if every call returns.
    pub will_return: bool,
    /// Calls in body order.
    pub calls: Vec<EffectCall>,
}

/// Callee effects consumed by one call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EffectCall {
    /// Possible callees in symbol order.
    pub targets: Vec<mir::Symbol>,
    /// Whether the call can select another function.
    pub is_open: bool,
    /// Whether a normal callee return returns from this function.
    pub is_tail: bool,
    /// Whether the call's signature may park the calling fiber.
    pub park: mir::ParkBehavior,
}

impl EffectBody {
    /// Extract local operation effects and calls from one function or declaration.
    pub fn analyse(
        function: mir::FunctionId,
        graph: &mir::ControlTable,
        resolution: &mir::ResolutionTable,
        tree: &mir::Tree,
    ) -> Self {
        // declare the effects of a function without a body by its binding and park
        let declaration = tree.get(function);
        if !declaration.is_defined() {
            let mut local = declaration
                .binding
                .as_ref()
                .map(|binding| mir::FunctionEffect::binding(binding))
                .unwrap_or_else(mir::FunctionEffect::external);
            local.behavior.park = declaration.park;

            return Self {
                will_return: local.behavior.return_behavior.is_will_return(),
                local,
                calls: Vec::new(),
            };
        }

        // extract each reachable operation once before interprocedural propagation
        let mut builder = FunctionEffectBuilder {
            tree,
            function,
            resolution,
            definitions: mir::DefinitionTable::analyse(declaration, tree),
            memory: mir::MemoryEffect::none(),
            behavior: mir::FunctionBehavior {
                park: declaration.park,
                ..mir::FunctionBehavior::none()
            },
            has_return: false,
            calls: Vec::new(),
        };
        for block in graph.reachable_blocks() {
            builder.record_block(block);
        }

        // reject cycles and abrupt exits before deriving guaranteed return from callees
        let order = mir::NodeTable::from_entries(
            graph
                .postorder()
                .enumerate()
                .map(|(index, block)| (block, index))
                .collect(),
        );
        let will_return = graph.reachable_blocks().all(|block| {
            let terminator = builder.tree.get(builder.tree.get(block).terminator);
            let is_abrupt = matches!(
                terminator,
                mir::Terminator::Abort { .. }
                    | mir::Terminator::Panic { .. }
                    | mir::Terminator::UnwindResume
                    | mir::Terminator::Unreachable
            );

            !is_abrupt
                && graph
                    .successors(block)
                    .all(|successor| order.get(successor) < order.get(block))
        }) && !builder.behavior.panic.may_panic()
            && !builder.behavior.park.may_park()
            && !builder.behavior.must_preserve_execution
            && !builder.behavior.allocates
            && !builder.behavior.frees;

        Self {
            local: mir::FunctionEffect {
                memory: builder.memory,
                behavior: mir::FunctionBehavior {
                    return_behavior: if builder.has_return {
                        mir::ReturnBehavior::MayReturn
                    } else {
                        mir::ReturnBehavior::NoReturn
                    },
                    ..builder.behavior
                },
            },
            will_return,
            calls: builder.calls,
        }
    }

    /// Combine local effects with the current effects of every possible callee.
    pub fn analyse_calls(
        &self,
        effects: &FxIndexMap<mir::Symbol, mir::FunctionEffect>,
    ) -> mir::FunctionEffect {
        // start with the effects of this function
        let mut memory = self.local.memory.clone();
        let mut behavior = self.local.behavior.clone();
        let mut has_return = !self.local.behavior.return_behavior.is_no_return();
        let mut will_return = self.will_return;

        // combine each call's alternatives before applying explicit call effects
        for call in &self.calls {
            let effect = call.analyse(effects);
            will_return &= effect.behavior.return_behavior.is_will_return();
            memory = memory.union(&effect.memory);
            behavior = behavior.union(&effect.behavior);
            if call.is_tail && !effect.behavior.return_behavior.is_no_return() {
                has_return = true;
            }
        }

        // publish guaranteed return only after every callee establishes it
        behavior.return_behavior = if has_return {
            mir::ReturnBehavior::MayReturn
        } else {
            mir::ReturnBehavior::NoReturn
        };
        if has_return && will_return {
            behavior.return_behavior = mir::ReturnBehavior::WillReturn;
        }

        mir::FunctionEffect { memory, behavior }
    }
}

impl EffectCall {
    /// Combine every possible callee, preserving uncertainty for unresolved dispatch.
    pub fn analyse(
        &self,
        effects: &FxIndexMap<mir::Symbol, mir::FunctionEffect>,
    ) -> mir::FunctionEffect {
        // start with an empty set of callee effects
        let mut memory = mir::MemoryEffect::none();
        let mut behavior = mir::FunctionBehavior::none();
        let mut has_return = false;
        let mut will_return = !self.is_open && !self.targets.is_empty();

        // include unknown internal callees
        if self.is_open {
            memory = memory.union(&mir::MemoryEffect::unknown());
            behavior = behavior.union(&mir::FunctionBehavior::external());
            has_return = true;
        }

        // require every named callee to have either a body or a declaration
        for target in &self.targets {
            let effect = effects
                .get(target)
                .unwrap_or_else(|| unreachable!("callee outside effect graph: {target:?}"));
            memory = memory.union(&effect.memory);
            behavior = behavior.union(&effect.behavior);
            has_return |= !effect.behavior.return_behavior.is_no_return();
            will_return &= effect.behavior.return_behavior.is_will_return();
        }

        // park as the signature declares
        behavior.park = behavior.park.union(self.park);

        // publish guaranteed return only after every callee establishes it
        behavior.return_behavior = if has_return {
            mir::ReturnBehavior::MayReturn
        } else {
            mir::ReturnBehavior::NoReturn
        };
        if will_return && !behavior.panic.may_panic() && !behavior.park.may_park() {
            behavior.return_behavior = mir::ReturnBehavior::WillReturn;
        }

        mir::FunctionEffect { memory, behavior }
    }
}

impl mir::EffectTable {
    /// Analyse function effects in callee first order, solving each recursive component.
    pub fn analyse(
        resolution: &mir::ResolutionTable,
        calls: &mir::CallTable,
        tree: &mir::Tree,
    ) -> Self {
        let mut bodies = FxIndexMap::default();
        let mut effects = FxIndexMap::default();

        // seed defined functions with their local effects and declarations with their bindings
        let functions = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for function in functions {
            let declaration = tree.get(function);
            let symbol = declaration.symbol;
            let graph = mir::ControlTable::analyse(declaration, tree);
            let body = EffectBody::analyse(function, &graph, resolution, tree);
            effects.insert(symbol, body.local.clone());
            bodies.insert(function, body);
        }

        // revisit only calls inside the component while its effects grow
        for (component_index, component) in calls.components().enumerate() {
            let functions = component.collect::<Vec<_>>();
            loop {
                let mut changed = false;
                for &function in &functions {
                    let symbol = tree.get(function).symbol;
                    let effect = bodies[&function].analyse_calls(&effects);
                    if effects[&symbol] != effect {
                        effects.insert(symbol, effect);
                        changed = true;
                    }
                }
                if !changed || !calls.is_recursive_component(component_index) {
                    break;
                }
            }
        }

        // publish inferred effects for every function and callsite
        let mut result = Self::default();
        result.replace_functions(
            tree.iter_nodes::<mir::Function>()
                .map(|(function, declaration)| (function, effects[&declaration.symbol].clone())),
        );
        for (_, function) in tree
            .iter_nodes::<mir::Function>()
            .filter(|(_, function)| function.is_defined())
        {
            for &block_id in function.blocks() {
                let block = tree.get(block_id);
                for &instruction in &block.instructions {
                    if let Some(call) = tree.get(instruction).call() {
                        let point = mir::Point::Instruction(instruction);
                        let call = EffectCall::new(point, call, false, resolution, tree);
                        result.insert_call(point, call.analyse(&effects));
                    }
                }
                if let Some(call) = tree.get(block.terminator).call() {
                    let point = mir::Point::Terminator(block_id);
                    let call = EffectCall::new(point, call, false, resolution, tree);
                    result.insert_call(point, call.analyse(&effects));
                }
            }
        }

        result
    }
}

impl EffectCall {
    /// Extract the callee symbols and signature park of one callsite.
    fn new(
        point: mir::Point,
        call: &mir::Call,
        is_tail: bool,
        resolution: &mir::ResolutionTable,
        tree: &mir::Tree,
    ) -> Self {
        // translate the known targets into persistent symbols
        let resolution = resolution.resolution(point);
        let mut targets = resolution
            .functions
            .iter()
            .map(|&function| tree.get(function).symbol)
            .collect::<Vec<_>>();
        targets.sort_unstable();
        targets.dedup();

        Self {
            targets,
            is_open: resolution.is_open,
            is_tail,
            park: call.park(tree),
        }
    }
}

impl Analysis for mir::EffectTable {
    const INVALIDATED_BY: Mutation = mir::CallTable::INVALIDATED_BY.union(Mutation::MEMORY);
}

/// Extract operation effects and call dependencies from one function body.
struct FunctionEffectBuilder<'a> {
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// The function being analysed.
    function: mir::FunctionId,

    /// Possible callees at each callsite.
    resolution: &'a mir::ResolutionTable,
    /// The definition of each SSA value.
    definitions: mir::DefinitionTable,

    /// Accumulated local memory accesses.
    memory: mir::MemoryEffect,
    /// Accumulated local behavior.
    behavior: mir::FunctionBehavior,
    /// Whether a reachable block returns normally.
    has_return: bool,
    /// Calls whose effects must be propagated.
    calls: Vec<EffectCall>,
}

impl FunctionEffectBuilder<'_> {
    /// Record the local effects and calls of one reachable block.
    fn record_block(&mut self, block_id: mir::BlockId) {
        let block = self.tree.get(block_id).clone();
        for &instruction in &block.instructions {
            let operation = &self.tree.get(instruction).clone();
            if let Some(call) = operation.call() {
                self.calls.push(EffectCall::new(
                    mir::Point::Instruction(instruction),
                    call,
                    false,
                    self.resolution,
                    self.tree,
                ));
            } else {
                let effect = self.instruction_effect(operation);
                self.memory = self.memory.union(&effect.memory);
                self.behavior = self.behavior.union(&effect.behavior);
            }
        }

        // preserve call returns separately from returns executed by this body
        let terminator = self.tree.get(block.terminator);
        match terminator {
            mir::Terminator::Return { .. } => self.has_return = true,
            mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
                let is_tail = matches!(terminator, mir::Terminator::TailCall { .. });
                self.calls.push(EffectCall::new(
                    mir::Point::Terminator(block_id),
                    call,
                    is_tail,
                    self.resolution,
                    self.tree,
                ));
            }
            mir::Terminator::Panic { .. } | mir::Terminator::UnwindResume => {
                self.behavior.panic = mir::PanicBehavior::MayPanic
            }
            mir::Terminator::Abort { .. } => self.behavior.must_preserve_execution = true,
            mir::Terminator::NewZeroedTry { space, .. }
            | mir::Terminator::NewUninitTry { space, .. }
            | mir::Terminator::NewSliceZeroedTry { space, .. }
            | mir::Terminator::NewSliceUninitTry { space, .. } => {
                self.memory = self
                    .memory
                    .union(&mir::MemoryEffect::write_only(space.space_set()));
                self.behavior.allocates = true;
            }
            mir::Terminator::Error => unreachable!("recovered terminator reached effect analysis"),
            mir::Terminator::Jump { .. }
            | mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::VariantSwitch { .. }
            | mir::Terminator::Unreachable => {}
        }
    }

    /// Build an effect for one non-call instruction.
    fn instruction_effect(&mut self, instruction: &mir::Instruction) -> mir::FunctionEffect {
        // map MIR operations to local effects
        let mut effect = match instruction {
            mir::Instruction::Load {
                place, result_type, ..
            } => {
                let storage = self.place_storage(place);
                let generics = &self.tree.get(self.function).generics;
                let memory = match is_copy(self.tree, *result_type, generics) {
                    true => mir::MemoryEffect::read_only(storage),
                    false => mir::MemoryEffect::read_write(storage),
                };

                mir::FunctionEffect::memory(memory)
            }
            mir::Instruction::AtomicLoad { place, .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::read_only(self.place_storage(place)))
            }
            mir::Instruction::VariantTagLoad { place, .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::read_only(self.place_storage(place)))
            }
            mir::Instruction::DynamicRead { dynamic, .. } => mir::FunctionEffect::memory(
                mir::MemoryEffect::read_only(self.value_storage(*dynamic)),
            ),
            mir::Instruction::Store { place, .. } | mir::Instruction::AtomicStore { place, .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::write_only(
                    self.place_storage(place),
                ))
            }
            mir::Instruction::AtomicCompareExchange { place, .. }
            | mir::Instruction::AtomicRmw { place, .. } => mir::FunctionEffect::memory(
                mir::MemoryEffect::read_write(self.place_storage(place)),
            ),
            mir::Instruction::AtomicFence { access } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::barrier(access.storage))
            }
            mir::Instruction::BarrierWrite { .. } => mir::FunctionEffect::none(),
            mir::Instruction::NewZeroed { space, .. }
            | mir::Instruction::NewUninit { space, .. }
            | mir::Instruction::NewSliceZeroed { space, .. }
            | mir::Instruction::NewSliceUninit { space, .. } => mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(space.space_set()),
                behavior: mir::FunctionBehavior::none().with_allocates(),
            },
            mir::Instruction::Release { value } => mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(self.value_storage(*value)),
                behavior: mir::FunctionBehavior::none().with_frees(),
            },
            mir::Instruction::Drop { .. } => mir::FunctionEffect::external(),
            mir::Instruction::ContextCurrent { .. } | mir::Instruction::ContextGet { .. } => {
                mir::FunctionEffect::memory(mir::MemoryEffect::read_only(mir::StorageSet::ANY))
            }
            mir::Instruction::ContextReplace { .. } => mir::FunctionEffect {
                memory: mir::MemoryEffect::unknown(),
                behavior: mir::FunctionBehavior::none().with_preserved_execution(),
            },
            mir::Instruction::ContextBind { .. } => mir::FunctionEffect {
                memory: mir::MemoryEffect::unknown(),
                behavior: mir::FunctionBehavior::none().with_allocates(),
            },
            mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::Poll => mir::FunctionEffect {
                memory: mir::MemoryEffect::none(),
                behavior: mir::FunctionBehavior::none().with_preserved_execution(),
            },
            mir::Instruction::Intrinsic { intrinsic, .. } => self.intrinsic_effect(*intrinsic),
            mir::Instruction::Breakpoint => mir::FunctionEffect {
                memory: mir::MemoryEffect::none(),
                behavior: mir::FunctionBehavior::none().with_preserved_execution(),
            },
            mir::Instruction::Copy { .. }
            | mir::Instruction::FakeBorrow { .. }
            | mir::Instruction::FakeRead { .. }
            | mir::Instruction::Const { .. }
            | mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Select { .. }
            | mir::Instruction::Address { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::FunctionBind { .. }
            | mir::Instruction::FunctionEnvironment { .. }
            | mir::Instruction::FunctionEnvironmentCurrent { .. }
            | mir::Instruction::Aggregate { .. }
            | mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementGet { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::VariantNew { .. }
            | mir::Instruction::VariantTag { .. }
            | mir::Instruction::VariantPayload { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicBind { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::TypeOf { .. }
            | mir::Instruction::DynamicFind { .. }
            | mir::Instruction::VectorSplat { .. }
            | mir::Instruction::VectorExtract { .. }
            | mir::Instruction::VectorInsert { .. }
            | mir::Instruction::VectorShuffle { .. }
            | mir::Instruction::VectorSelect { .. }
            | mir::Instruction::VectorReduce { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::VectorConvert { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::Assume { .. } => mir::FunctionEffect::none(),
            mir::Instruction::Error => {
                unreachable!("recovered instruction reached effect analysis")
            }
            mir::Instruction::Call { .. } => unreachable!("call bypassed effect extraction"),
        };

        // preserve atomic execution and its dependence on concurrent operations
        if matches!(
            instruction,
            mir::Instruction::AtomicLoad { .. }
                | mir::Instruction::AtomicStore { .. }
                | mir::Instruction::AtomicCompareExchange { .. }
                | mir::Instruction::AtomicRmw { .. }
                | mir::Instruction::AtomicFence { .. }
        ) {
            effect.behavior.must_preserve_execution = true;
            effect.behavior.determinism = mir::Determinism::NonDeterministic;
        }

        // retain reads of stored references traversed by every memory operand
        if let Some(place) = instruction.place() {
            let mut prefix = mir::Place::new(place.origin);
            for projection in &place.path.projections {
                if *projection == mir::Projection::Deref {
                    let read = mir::MemoryEffect::read_only(self.place_storage(&prefix));
                    effect.memory = effect.memory.union(&read);
                }
                prefix.push(projection.clone());
            }
        }

        effect
    }

    /// Resolve externally observable storage selected by a place.
    fn place_storage(&mut self, place: &mir::Place) -> mir::StorageSet {
        // direct function storage does not escape through the operation itself
        if matches!(
            place.origin,
            mir::PlaceOrigin::Local(_) | mir::PlaceOrigin::Value(_)
        ) && !place.path.projections.contains(&mir::Projection::Deref)
        {
            return mir::StorageSet::NONE;
        }

        place.storage_set(self.function, self.tree)
    }

    /// Resolve the backing storage for one typed value, an allocation's from its site.
    fn value_storage(&mut self, value: mir::Value) -> mir::StorageSet {
        if let Some(instruction) = self.definitions.instruction(value)
            && let Some(space) = self.tree.get(instruction).allocation_space()
        {
            return space.space_set();
        }
        let ty = self.tree.get(self.function).expect_value_type(value);

        self.type_storage(ty)
    }

    /// Resolve the backing storage for one reference-like type.
    fn type_storage(&mut self, ty: mir::TypeId) -> mir::StorageSet {
        let ty = ty.storage(self.tree);

        self.tree
            .get(ty)
            .reference_storage_set()
            .unwrap_or(mir::StorageSet::ANY)
    }

    /// Build a memory effect for an intrinsic.
    fn intrinsic_memory(&self, intrinsic: mir::Intrinsic) -> mir::MemoryEffect {
        match intrinsic {
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => {
                mir::MemoryEffect::read_write(mir::StorageSet::ANY)
            }
            mir::Intrinsic::VolatileLoad => mir::MemoryEffect::read_only(mir::StorageSet::ANY),
            mir::Intrinsic::VolatileStore => mir::MemoryEffect::write_only(mir::StorageSet::ANY),
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
            mir::Intrinsic::SpinLoop
                | mir::Intrinsic::BlackBox
                | mir::Intrinsic::VolatileLoad
                | mir::Intrinsic::VolatileStore
        ) {
            mir::FunctionBehavior::none().with_preserved_execution()
        } else {
            mir::FunctionBehavior::none()
        };

        mir::FunctionEffect { memory, behavior }
    }
}

#[cfg(test)]
mod tests {
    use crate as mir;
    use crate::analyses::tests::TestModule;

    /// Pure arithmetic functions have no memory or behavioral effects.
    #[test]
    fn test_classify_pure_function() {
        let program = TestModule::new(
            r#"
function pure(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    return v1
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let function = program.function_id_by_name("pure");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(effect.memory, mir::MemoryEffect::none());
        assert_eq!(
            effect.behavior,
            mir::FunctionBehavior::none().with_will_return()
        );
    }

    /// A fence orders the storage it names without reading or writing it.
    #[test]
    fn test_summarize_fence_as_barrier() {
        let program = TestModule::new(
            r#"
function fence(): void {
entry:
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let function = program.function_id_by_name("fence");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(
            effect.memory,
            mir::MemoryEffect::barrier(mir::StorageSet::SHARED)
        );
        assert!(!effect.memory.reads() && !effect.memory.writes());
    }

    /// Allocation and free instructions are surfaced in function behavior.
    #[test]
    fn test_record_allocation_effects() {
        let program = TestModule::new(
            r#"
function allocate(): void {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    release v0
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let function = program.function_id_by_name("allocate");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(
            *effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(mir::StorageSet::LOCAL),
                behavior: mir::FunctionBehavior::none().with_allocates().with_frees(),
            }
        );
    }

    /// Preserve execution of spin hints, black boxes, and breakpoints.
    #[test]
    fn test_preserve_execution_effects() {
        let program = TestModule::new(
            r#"
function backoff(): void {
entry:
    intrinsic.hint.spinLoop()
    return
}

function conceal(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = intrinsic.hint.blackBox(v0)
    return v1
}

function inspect(): void {
entry:
    breakpoint
    return
}
"#,
        );
        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let expected = mir::FunctionEffect {
            memory: mir::MemoryEffect::none(),
            behavior: mir::FunctionBehavior::none().with_preserved_execution(),
        };
        for name in ["backoff", "conceal", "inspect"] {
            assert_eq!(
                effects.function(program.function_id_by_name(name)),
                Some(&expected),
                "{name}"
            );
        }
    }

    /// Direct calls propagate callee effects to callers.
    #[test]
    fn test_propagate_direct_calls() {
        let program = TestModule::new(
            r#"
function allocate(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
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
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let function = program.function_id_by_name("root");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(
            *effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(mir::StorageSet::LOCAL),
                behavior: mir::FunctionBehavior::none().with_allocates(),
            }
        );
    }

    /// Resolved virtual calls propagate callee effects to callers.
    #[test]
    fn test_propagate_virtual_calls() {
        let mut program = TestModule::new(
            r#"
function allocate(v0: int32): ref<int32, unique, mutable> {
entry(v0: int32):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    return v1
}

function root(v0: int32): ref<int32, unique, mutable> {
entry(v0: int32):
    v1: ref<int32, unique, mutable> = call.virtual int32, 0(v0): (int32) => ref<int32, unique, mutable>
    return v1
}
"#,
        );
        let allocate = program.function_id_by_name("allocate");
        let root = program.function_id_by_name("root");
        let concrete = program.tree.get(root).parameters[0].ty;
        program.dispatch.insert_virtual_table(mir::VirtualTable {
            concrete,
            value: concrete,
            slots: vec![mir::VirtualSlot::Method {
                function: allocate,
                arguments: Vec::new(),
            }],
        });

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let effect = effects.function(root).expect("missing function effect");

        assert_eq!(
            *effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::write_only(mir::StorageSet::LOCAL),
                behavior: mir::FunctionBehavior::none().with_allocates(),
            }
        );
    }

    /// Direct calls propagate the effects a bodyless function's binding declares.
    #[test]
    fn test_propagate_declared_calls() {
        let program = TestModule::new(
            r#"
@binding("test.allocate", { provider: "runtime", effect: "deterministic" })
external function allocate(): void

function root(): void {
entry:
    call allocate(): () => void
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let root = program.function_id_by_name("root");
        let effect = effects.function(root).expect("missing function effect");

        // inherit the deterministic binding's allocation and possible panic
        assert_eq!(
            *effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::unknown(),
                behavior: mir::FunctionBehavior {
                    determinism: mir::Determinism::Deterministic,
                    panic: mir::PanicBehavior::MayPanic,
                    return_behavior: mir::ReturnBehavior::MayReturn,
                    park: mir::ParkBehavior::CannotPark,
                    must_preserve_execution: false,
                    allocates: true,
                    frees: false,
                },
            }
        );
    }

    /// Direct calls propagate parking behavior from bodyless functions.
    #[test]
    fn test_propagate_parking() {
        let program = TestModule::new(
            r#"
external park function park(): void

function root(): void {
entry:
    call park(): park () => void
    return
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let root = program.function_id_by_name("root");
        let effect = effects.function(root).expect("missing function effect");

        // inherit the external callee's effects and its declared park
        assert_eq!(
            *effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::unknown(),
                behavior: mir::FunctionBehavior::external().with_park(),
            }
        );
    }

    /// Park an unknown callee exactly as its signature declares.
    #[test]
    fn test_park_unknown_callee_by_signature() {
        let program = TestModule::new(
            r#"
function run(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}

park function wait(v0: fn park (int32) => int32, v1: int32): int32 {
entry(v0: fn park (int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): park (int32) => int32
    return v2
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let run = program.function_id_by_name("run");
        let run = effects.function(run).expect("missing function effect");
        let wait = program.function_id_by_name("wait");
        let wait = effects.function(wait).expect("missing function effect");

        // keep the external effects of an unknown callee, parking only through a parking signature
        assert_eq!(run.memory, mir::MemoryEffect::unknown());
        assert_eq!(run.behavior, mir::FunctionBehavior::external());
        assert_eq!(wait.memory, mir::MemoryEffect::unknown());
        assert_eq!(wait.behavior, mir::FunctionBehavior::external().with_park());
    }

    /// Panic terminators are may-panic and no-return.
    #[test]
    fn test_record_panics_without_returns() {
        let program = TestModule::new(
            r#"
function fail(v0: ref<int32, managed, readonly, local>): void {
entry(v0: ref<int32, managed, readonly, local>):
    panic v0
}
"#,
        );

        let mut analyses = program.module_analyses();
        let effects = analyses.effect(&program.tree, &program.dispatch);
        let function = program.function_id_by_name("fail");
        let effect = effects.function(function).expect("missing function effect");

        assert_eq!(
            *effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::none(),
                behavior: mir::FunctionBehavior::none().with_panic().with_noreturn(),
            }
        );
    }

    /// Infer guaranteed return through acyclic callees while preserving possible divergence.
    #[test]
    fn test_propagate_return_behavior() {
        let program = TestModule::new(
            r#"
function leaf(): void {
entry:
    return
}

function caller(): void {
entry:
    call leaf(): () => void
    return
}

function tail(): void {
entry:
    tail.call caller(): () => void
}

function recursive(): void {
entry:
    call recursive(): () => void
    return
}

function cycle(v0: boolean): void {
entry(v0: boolean):
    jump loop

loop:
    branch v0 => loop | exit

exit:
    return
}

function endless(): void {
entry:
    jump entry
}
"#,
        );
        let mut analyses = program.module_analyses();
        let table = analyses.effect(&program.tree, &program.dispatch);
        let returns = ["leaf", "caller", "tail", "recursive", "cycle", "endless"].map(|name| {
            table
                .function(program.function_id_by_name(name))
                .unwrap()
                .behavior
                .return_behavior
        });

        assert_eq!(
            returns,
            [
                mir::ReturnBehavior::WillReturn,
                mir::ReturnBehavior::WillReturn,
                mir::ReturnBehavior::WillReturn,
                mir::ReturnBehavior::MayReturn,
                mir::ReturnBehavior::MayReturn,
                mir::ReturnBehavior::NoReturn,
            ]
        );
    }

    /// Distinguish ordinary memory accesses from observable atomic accesses.
    #[test]
    fn test_classify_memory_accesses() {
        let program = TestModule::new(
            r#"
function read<'a>(v0: ref<int32, borrowed, 'a, readonly>): int32 {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    v1: int32 = load (*v0)
    return v1

dead:
    poll
    v2: int32 = 0
    return v2
}

function write<'a>(v0: ref<int32, borrowed, 'a, mutable>, v1: int32): void {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: int32):
    store (*v0), v1
    return
}

function synchronize<'a>(v0: ref<int32, borrowed, 'a, readonly>): int32 {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    v1: int32 = atomic.load (*v0), acquire, scope(device)
    return v1
}
"#,
        );
        let mut analyses = program.module_analyses();
        let table = analyses.effect(&program.tree, &program.dispatch);
        let actual = ["read", "write", "synchronize"].map(|name| {
            table
                .function(program.function_id_by_name(name))
                .unwrap()
                .clone()
        });
        let atomic = mir::FunctionBehavior {
            determinism: mir::Determinism::NonDeterministic,
            ..mir::FunctionBehavior::none().with_preserved_execution()
        };

        assert_eq!(
            actual,
            [
                mir::FunctionEffect {
                    memory: mir::MemoryEffect::read_only(mir::StorageSet::ANY),
                    behavior: mir::FunctionBehavior::none().with_will_return()
                },
                mir::FunctionEffect {
                    memory: mir::MemoryEffect::write_only(mir::StorageSet::ANY),
                    behavior: mir::FunctionBehavior::none().with_will_return()
                },
                mir::FunctionEffect {
                    memory: mir::MemoryEffect::read_only(mir::StorageSet::ANY),
                    behavior: atomic
                },
            ]
        );
    }
}
