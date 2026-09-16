use std::collections::VecDeque;

use destack_core::BitSet;

use crate as mir;
use crate::{Analysis, Mutation};

/// Possible callees at each callsite in one MIR module.
#[derive(Debug, Default)]
pub struct ResolutionTable {
    /// Call targets sorted by callsite.
    targets: Vec<(mir::Point, Resolution)>,
}

/// The known callees and completeness of one call resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    /// Known functions in id order.
    pub functions: Vec<mir::FunctionId>,
    /// Whether the target can include an unknown function.
    pub is_open: bool,
}

impl Resolution {
    /// Create a closed set containing one function.
    fn function(function: mir::FunctionId) -> Self {
        Self {
            functions: vec![function],
            is_open: false,
        }
    }

    /// Create an unresolved target set.
    fn open() -> Self {
        Self {
            functions: Vec::new(),
            is_open: true,
        }
    }

    /// Add possible callees and return whether the set changed.
    fn extend(&mut self, other: &Self) -> bool {
        // preserve unresolved alternatives from either path
        let mut changed = other.is_open && !self.is_open;
        self.is_open |= other.is_open;

        // insert each newly discovered function in id order
        for &function in &other.functions {
            if let Err(index) = self.functions.binary_search(&function) {
                self.functions.insert(index, function);
                changed = true;
            }
        }

        changed
    }
}

impl ResolutionTable {
    /// Return the sole callee when every possible target agrees.
    pub fn target(&self, callsite: mir::Point) -> Option<mir::FunctionId> {
        let targets = self.resolution(callsite);

        match targets.functions.as_slice() {
            [function] if !targets.is_open => Some(*function),
            _ => None,
        }
    }

    /// Return all known callees and whether the call remains open.
    pub fn resolution(&self, callsite: mir::Point) -> &Resolution {
        let index = self
            .targets
            .binary_search_by_key(&callsite, |(point, _)| *point)
            .unwrap_or_else(|_| unreachable!("callsite outside resolution table: {callsite:?}"));

        &self.targets[index].1
    }

    /// Analyse call targets from function values and concrete dispatch tables.
    pub fn analyse(
        dispatch: &mir::DispatchTable,
        witnesses: Option<&mir::WitnessTable>,
        tree: &mir::Tree,
    ) -> Self {
        let mut targets = Vec::new();

        // resolve function values once before visiting each callsite
        let functions = tree
            .iter_nodes::<mir::Function>()
            .filter(|(_, function)| function.is_defined())
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for function in functions {
            let mut resolver = Resolver {
                tree,
                function,
                dispatch,
                witnesses,
                values: Vec::new(),
            };
            resolver.analyse();

            // resolve each instruction call from its propagated operands
            for block_index in 0..resolver.tree.get(function).blocks().len() {
                let block_id = resolver.tree.get(function).blocks()[block_index];
                let block = resolver.tree.get(block_id).clone();
                for &instruction_id in &block.instructions {
                    if let mir::Instruction::Call { call, .. } =
                        &resolver.tree.get(instruction_id).clone()
                    {
                        targets.push((
                            mir::Point::Instruction(instruction_id),
                            resolver.resolve(call),
                        ));
                    }
                }

                // include calls that end the block
                if let mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } =
                    &resolver.tree.get(block.terminator).clone()
                {
                    targets.push((mir::Point::Terminator(block_id), resolver.resolve(call)));
                }
            }
        }

        // index the callsites independently of function layout order
        targets.sort_unstable_by_key(|(point, _)| *point);

        Self { targets }
    }
}

impl Analysis for ResolutionTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::LAYOUT)
        .union(Mutation::DISPATCH)
        .union(Mutation::SYMBOL);
}

/// Callees and concrete receivers established by one value.
#[derive(Clone, PartialEq, Eq)]
enum CallValue {
    /// No incoming assignment has established a value.
    Unknown,
    /// The value has an unknown callee or concrete receiver type.
    Open,
    /// The value contains these function addresses.
    Functions(Resolution),
    /// The value addresses storage of this concrete type.
    Receiver(mir::TypeId),
    /// The aggregate has known values at these field or element indices.
    Fields(Vec<(u32, CallValue)>),
}

impl CallValue {
    /// Read a field while preserving an unevaluated aggregate.
    fn field(&self, index: u32) -> &Self {
        match self {
            Self::Unknown => self,
            Self::Fields(fields) => fields
                .binary_search_by_key(&index, |(field, _)| *field)
                .map(|index| &fields[index].1)
                .unwrap_or(&Self::Open),
            _ => &Self::Open,
        }
    }

    /// Merge possible values and report whether the result changed.
    fn merge(&mut self, incoming: &Self) -> bool {
        // preserve stable values and assignments that have not been evaluated
        if incoming == &Self::Unknown || self == incoming {
            return false;
        }

        match (&mut *self, incoming) {
            // accept the first evaluated assignment
            (Self::Unknown, _) => {
                *self = incoming.clone();

                true
            }
            // combine named function alternatives
            (Self::Functions(targets), Self::Functions(incoming)) => targets.extend(incoming),
            (Self::Functions(targets), _) => targets.extend(&Resolution::open()),
            // merge aggregate fields without mixing their signatures
            (Self::Fields(fields), _) => {
                let mut changed = false;
                for (index, value) in fields.iter_mut() {
                    changed |= value.merge(incoming.field(*index));
                }

                // include fields first established by this incoming assignment
                if let Self::Fields(incoming) = incoming {
                    for (index, value) in incoming {
                        if let Err(position) =
                            fields.binary_search_by_key(index, |(index, _)| *index)
                        {
                            let mut merged = Self::Open;
                            merged.merge(value);
                            fields.insert(position, (*index, merged));
                            changed = true;
                        }
                    }
                }

                changed
            }
            // preserve named alternatives discovered after an open assignment
            (_, Self::Functions(_) | Self::Fields(_)) => {
                let mut merged = incoming.clone();
                merged.merge(&Self::Open);
                *self = merged;

                true
            }
            // discard an exact receiver when incoming paths disagree
            _ => {
                let changed = self != &Self::Open;
                *self = Self::Open;

                changed
            }
        }
    }
}

/// Resolve one function's calls by propagating values through assignments and projections.
struct Resolver<'a> {
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// The function whose values and locals are indexed.
    function: mir::FunctionId,
    /// The available dispatch tables.
    dispatch: &'a mir::DispatchTable,
    /// Available witnesses before instantiation resolves their calls.
    witnesses: Option<&'a mir::WitnessTable>,

    /// Propagated values followed by local storage.
    values: Vec<CallValue>,
}

/// One assignment whose sources can change during propagation.
enum Transfer {
    /// Copy an SSA value or local into another location.
    Copy {
        /// The source value or local index.
        source: usize,
        /// The destination value or local index.
        destination: usize,
    },
    /// Evaluate the result of an instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
}

impl Resolver<'_> {
    /// Propagate call targets and receiver types through this function's assignments.
    fn analyse(&mut self) {
        // allocate one entry per value and local
        let count = self.tree.get(self.function).value_capacity();
        self.values = vec![CallValue::Unknown; count + self.tree.get(self.function).locals().len()];
        let locals = mir::NodeTable::from_entries(
            self.tree
                .get(self.function)
                .locals()
                .iter()
                .enumerate()
                .map(|(index, &local)| (local, count + index))
                .collect(),
        );
        let mut transfers = Vec::new();
        let mut edges = Vec::new();

        // admit caller supplied values at function entry
        for parameter in &self.tree.get(self.function).parameters {
            self.values[parameter.value.id() as usize] = CallValue::Open;
        }

        // connect instruction operands to the assignments that consume them
        for &block_id in self.tree.get(self.function).blocks() {
            let block = self.tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                let transfer = match instruction {
                    mir::Instruction::Copy { destination, value } => Transfer::Copy {
                        source: value.id() as usize,
                        destination: destination.id() as usize,
                    },
                    mir::Instruction::Store {
                        place:
                            mir::Place {
                                origin: mir::PlaceOrigin::Local(local),
                                path,
                            },
                        value,
                    } if path.is_root() => Transfer::Copy {
                        source: value.id() as usize,
                        destination: *locals.get(*local),
                    },
                    mir::Instruction::Load {
                        destination,
                        place:
                            mir::Place {
                                origin: mir::PlaceOrigin::Local(local),
                                path,
                            },
                        ..
                    } if path.is_root() => Transfer::Copy {
                        source: *locals.get(*local),
                        destination: destination.id() as usize,
                    },
                    mir::Instruction::Address {
                        place:
                            mir::Place {
                                origin: mir::PlaceOrigin::Local(local),
                                path,
                            },
                        ..
                    } if !path.projections.contains(&mir::Projection::Deref) => {
                        self.values[*locals.get(*local)] = CallValue::Open;
                        Transfer::Instruction(instruction_id)
                    }
                    _ if instruction.destination().is_some() => {
                        Transfer::Instruction(instruction_id)
                    }
                    _ => continue,
                };

                // index every source used by this assignment
                match &transfer {
                    Transfer::Copy { source, .. } => edges.push((*source, transfers.len())),
                    Transfer::Instruction(_) => {
                        edges.extend(
                            instruction
                                .reads(self.tree)
                                .into_iter()
                                .map(|value| (value.id() as usize, transfers.len())),
                        );
                    }
                }
                transfers.push(transfer);
            }

            // connect explicit edge arguments and seed generated allocation results
            let terminator = self.tree.get(block.terminator);
            for (edge, target) in terminator.targets(self.tree, block_id) {
                let parameters = &self.tree.get(target.block).parameters;
                let count = terminator.target_result_count(self.tree, edge.successor);
                for parameter in parameters.iter().take(count) {
                    let value = match terminator {
                        mir::Terminator::NewZeroedTry { storage_type, .. }
                        | mir::Terminator::NewUninitTry { storage_type, .. }
                            if edge.successor == mir::Successor::NewSuccess =>
                        {
                            CallValue::Receiver(*storage_type)
                        }
                        _ => CallValue::Open,
                    };
                    self.values[parameter.value.id() as usize].merge(&value);
                }

                // copy explicit arguments into their destination parameters
                for (parameter, argument) in
                    parameters[count..].iter().zip(target.arguments(self.tree))
                {
                    let source = argument.id() as usize;
                    let destination = parameter.value.id() as usize;
                    edges.push((source, transfers.len()));
                    transfers.push(Transfer::Copy {
                        source,
                        destination,
                    });
                }
            }
        }

        // group consumers by their source location
        edges.sort_unstable();
        edges.dedup();
        let mut offsets = vec![0; self.values.len() + 1];
        for &(source, _) in &edges {
            offsets[source + 1] += 1;
        }
        for index in 0..self.values.len() {
            offsets[index + 1] += offsets[index];
        }

        // evaluate each assignment once before following changes to its operands
        let mut pending = (0..transfers.len()).collect::<VecDeque<_>>();
        let mut queued = BitSet::new(transfers.len());
        for index in 0..transfers.len() {
            queued.insert(index);
        }

        // evaluate pending assignments from their current source values
        while let Some(index) = pending.pop_front() {
            queued.remove(index);
            let (destination, incoming) = match transfers[index] {
                Transfer::Copy {
                    source,
                    destination,
                } => (destination, self.values[source].clone()),
                Transfer::Instruction(instruction) => {
                    let instruction = self.tree.get(instruction);
                    let destination = instruction
                        .destination()
                        .unwrap_or_else(|| unreachable!("call transfer has no result"));

                    (destination.id() as usize, self.instruction(instruction))
                }
            };

            // revisit consumers only when their source gains a possible value
            if self.values[destination].merge(&incoming) {
                for &(_, consumer) in &edges[offsets[destination]..offsets[destination + 1]] {
                    if queued.insert(consumer) {
                        pending.push_back(consumer);
                    }
                }
            }
        }
    }

    /// Evaluate one instruction using its current operands.
    fn instruction(&self, instruction: &mir::Instruction) -> CallValue {
        match instruction {
            // identify named functions and concrete allocations
            mir::Instruction::FunctionAddr { function, .. }
            | mir::Instruction::FunctionBind { function, .. } => {
                CallValue::Functions(Resolution::function(*function))
            }
            mir::Instruction::NewZeroed { storage_type, .. }
            | mir::Instruction::NewUninit { storage_type, .. } => {
                CallValue::Receiver(*storage_type)
            }
            mir::Instruction::DynamicBind { concrete, .. } => CallValue::Receiver(*concrete),
            mir::Instruction::NewComplete { value, .. }
            | mir::Instruction::DynamicPayload { dynamic: value, .. } => self.value(*value).clone(),
            mir::Instruction::Cast {
                operator: mir::CastOperator::Bitcast,
                argument,
                destination,
                ..
            } => self.cast(*argument, *destination),
            // combine the alternatives selected by this value
            mir::Instruction::Select {
                then_value,
                else_value,
                ..
            } => {
                let mut value = self.value(*then_value).clone();
                value.merge(self.value(*else_value));

                value
            }
            // preserve each constructed field separately
            mir::Instruction::Aggregate { values, .. } => CallValue::Fields(
                self.tree
                    .get_values(*values)
                    .iter()
                    .enumerate()
                    .map(|(index, &value)| (index as u32, self.value(value).clone()))
                    .collect(),
            ),
            // select the requested field or array element
            mir::Instruction::FieldGet {
                aggregate, field, ..
            }
            | mir::Instruction::ElementGet {
                aggregate,
                index: field,
                ..
            } => self.value(*aggregate).field(*field).clone(),
            // replace one field while retaining the other aggregate values
            mir::Instruction::FieldSet {
                aggregate,
                field,
                value,
                ..
            }
            | mir::Instruction::ElementSet {
                aggregate,
                index: field,
                value,
                ..
            } => match self.value(*aggregate) {
                CallValue::Unknown => CallValue::Unknown,
                aggregate => {
                    let mut fields = match aggregate {
                        CallValue::Fields(fields) => fields.clone(),
                        _ => Vec::new(),
                    };
                    let value = self.value(*value).clone();
                    match fields.binary_search_by_key(field, |(field, _)| *field) {
                        Ok(index) => fields[index].1 = value,
                        Err(index) => fields.insert(index, (*field, value)),
                    }

                    CallValue::Fields(fields)
                }
            },
            // forward identities through compiler intrinsics
            mir::Instruction::Intrinsic {
                intrinsic: mir::Intrinsic::Transmute,
                arguments,
                destination: Some(destination),
            } => self.cast(self.tree.get_values(*arguments)[0], *destination),
            mir::Instruction::Intrinsic {
                intrinsic: mir::Intrinsic::SpaceCast | mir::Intrinsic::Expect,
                arguments,
                ..
            } => self.value(self.tree.get_values(*arguments)[0]).clone(),
            _ => CallValue::Open,
        }
    }

    /// Preserve scalar identities and aggregate fields through compatible bitcasts.
    fn cast(&self, source: mir::Value, destination: mir::Value) -> CallValue {
        match self.value(source) {
            CallValue::Fields(_)
                if self.tree.get(self.function).value_type(source)
                    != self.tree.get(self.function).value_type(destination) =>
            {
                CallValue::Open
            }
            value => value.clone(),
        }
    }

    /// Read a propagated SSA value.
    fn value(&self, value: mir::Value) -> &CallValue {
        &self.values[value.id() as usize]
    }

    /// Resolve the possible targets of one call.
    fn resolve(&mut self, call: &mir::Call) -> Resolution {
        match &call.callee {
            mir::Callee::Direct { function, .. } => Resolution::function(*function),
            mir::Callee::Indirect { value } => match self.value(*value) {
                CallValue::Functions(targets) => targets.clone(),
                _ => Resolution::open(),
            },
            mir::Callee::Virtual {
                receiver,
                class,
                slot,
            } => {
                let target = self
                    .receiver_type(*receiver)
                    .filter(|receiver| receiver == class)
                    .and_then(|receiver| self.dispatch.virtual_table(receiver))
                    .map(|table| {
                        *table.methods.get(slot.index()).unwrap_or_else(|| {
                            unreachable!("virtual call selects a missing method")
                        })
                    });

                target
                    .map(Resolution::function)
                    .unwrap_or_else(Resolution::open)
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot,
            } => {
                let entry =
                    self.receiver_type(*receiver)
                        .and_then(|receiver| self.dispatch.dynamic_table(receiver, *constraint))
                        .map(|table| {
                            table.entries.get(slot.index()).unwrap_or_else(|| {
                                unreachable!("dynamic call selects a missing entry")
                            })
                        });

                match entry {
                    Some(mir::DynamicEntry::Function { function }) => {
                        Resolution::function(*function)
                    }
                    Some(mir::DynamicEntry::Field { .. }) => {
                        unreachable!("dynamic call selects a field entry")
                    }
                    Some(mir::DynamicEntry::Absent) | None => Resolution::open(),
                }
            }
            mir::Callee::Witness {
                receiver,
                interface,
                requirement,
                ..
            } => {
                let function = self
                    .witnesses
                    .and_then(|witnesses| witnesses.get(*receiver, *interface))
                    .and_then(|witness| {
                        witness
                            .functions
                            .iter()
                            .find(|member| member.requirement == *requirement)
                    })
                    .map(|member| member.function);

                function
                    .map(Resolution::function)
                    .unwrap_or_else(Resolution::open)
            }
        }
    }

    /// Return a statically exact receiver type when its representation establishes it.
    fn receiver_type(&mut self, receiver: mir::Value) -> Option<mir::TypeId> {
        // use an allocation's concrete type for reference receivers
        if let CallValue::Receiver(ty) = self.value(receiver) {
            return Some(*ty);
        }

        // use the declared type for values whose representation is concrete
        let ty = self
            .tree
            .get(self.function)
            .value_type(receiver)
            .unwrap_or_else(|| unreachable!("receiver has no type"));
        let definition = mir::Substitution::resolve(ty, self.tree);

        (!self.tree.get(definition).is_reference_representation()).then_some(ty)
    }
}

#[cfg(test)]
mod tests {
    use crate as mir;

    use crate::analyses::tests::{TestModule, TestProgram};

    /// Resolve a function field independently of neighboring fields with different signatures.
    #[test]
    fn test_resolve_function_field() {
        let mut program = TestProgram::new(&[r#"
function first(): void {
entry:
    return
}

function second(v0: int32): void {
entry(v0: int32):
    return
}

function test(v0: int32): void {
entry(v0: int32):
    v1: fn() => void = function.address first
    v2: fn(int32) => void = function.address second
    v3: (fn() => void, fn(int32) => void) = aggregate (v1, v2)
    v4: fn(int32) => void = field.get v3, 1
    call.indirect v4(v0): (int32) => void
    return
}
"#]);
        let effects = program.analyse_effects(None);
        let module = &mut program.modules[0];
        let function = module.entry_function_id();
        let block = module.tree.get(function).block(0);
        let resolution = mir::ResolutionTable::analyse(&module.dispatch, None, &module.tree);
        let point = mir::Point::Instruction(module.tree.get(block).instructions[4]);

        assert_eq!(
            resolution.resolution(point),
            &mir::Resolution {
                functions: vec![module.function_id_by_name("second")],
                is_open: false,
            }
        );
        assert_eq!(
            effects
                .function(module.tree.get(function).symbol)
                .unwrap()
                .effect,
            mir::FunctionEffect {
                memory: mir::MemoryEffect::none(),
                behavior: mir::FunctionBehavior::none().with_will_return(),
            }
        );
    }

    /// Resolve a replaced field while retaining the caller supplied value in its neighbor.
    #[test]
    fn test_resolve_replaced_function_field() {
        let module = TestModule::new(
            r#"
function callee(): void {
entry:
    return
}

function test(v0: (fn() => void, fn() => void)): void {
entry(v0: (fn() => void, fn() => void)):
    v1: fn() => void = function.address callee
    v2: (fn() => void, fn() => void) = field.set v0, 1, v1
    v3: fn() => void = field.get v2, 1
    call.indirect v3(): () => void
    v4: fn() => void = field.get v2, 0
    call.indirect v4(): () => void
    return
}
"#,
        );
        let function = module.entry_function_id();
        let block = module.tree.get(function).block(0);
        let resolution = mir::ResolutionTable::analyse(&module.dispatch, None, &module.tree);
        let callee = module.function_id_by_name("callee");
        let targets = [
            module.tree.get(block).instructions[3],
            module.tree.get(block).instructions[5],
        ]
        .map(|instruction| resolution.resolution(mir::Point::Instruction(instruction)));

        assert_eq!(
            targets,
            [
                &mir::Resolution {
                    functions: vec![callee],
                    is_open: false
                },
                &mir::Resolution {
                    functions: vec![],
                    is_open: true
                },
            ]
        );
    }

    /// Resolve nested function fields selected by loop arguments and copied through a local.
    #[test]
    fn test_resolve_nested_function_fields_in_loop() {
        let module = TestModule::new(
            r#"
function first(): void {
entry:
    return
}

function second(): void {
entry:
    return
}

function test(v0: boolean): void {
    local l0: ((fn() => void, int32), int32)

entry(v0: boolean):
    v1: fn() => void = function.address first
    v2: fn() => void = function.address second
    v3: int32 = 0
    v4: (fn() => void, int32) = aggregate (v1, v3)
    v5: (fn() => void, int32) = aggregate (v2, v3)
    v6: ((fn() => void, int32), int32) = aggregate (v4, v3)
    v7: ((fn() => void, int32), int32) = aggregate (v5, v3)
    jump loop(v6)

loop(v8: ((fn() => void, int32), int32)):
    store l0, v8
    branch v0 => loop(v7) | exit

exit:
    v9: ((fn() => void, int32), int32) = load l0
    v10: (fn() => void, int32) = field.get v9, 0
    v11: fn() => void = field.get v10, 0
    call.indirect v11(): () => void
    return
}
"#,
        );
        let function = module.entry_function_id();
        let block = module.tree.get(function).blocks()[2];
        let resolution = mir::ResolutionTable::analyse(&module.dispatch, None, &module.tree);
        let first = module.function_id_by_name("first");
        let second = module.function_id_by_name("second");

        assert_eq!(
            resolution.resolution(mir::Point::Instruction(
                module.tree.get(block).instructions[3]
            )),
            &mir::Resolution {
                functions: vec![first, second],
                is_open: false
            }
        );
    }

    /// Resolve each callback after replacing one fixed array element.
    #[test]
    fn test_resolve_function_array_elements() {
        let module = TestModule::new(
            r#"
function first(): void {
entry:
    return
}

function second(): void {
entry:
    return
}

function test(): void {
entry:
    v0: fn() => void = function.address first
    v1: fn() => void = function.address second
    v2: [fn() => void; 2] = aggregate (v0, v0)
    v3: [fn() => void; 2] = element.set v2, 1, v1
    v4: fn() => void = element.get v3, 0
    call.indirect v4(): () => void
    v5: fn() => void = element.get v3, 1
    call.indirect v5(): () => void
    return
}
"#,
        );
        let function = module.entry_function_id();
        let block = module.tree.get(function).block(0);
        let resolution = mir::ResolutionTable::analyse(&module.dispatch, None, &module.tree);
        let first = module.function_id_by_name("first");
        let second = module.function_id_by_name("second");
        let targets = [
            module.tree.get(block).instructions[5],
            module.tree.get(block).instructions[7],
        ]
        .map(|instruction| resolution.resolution(mir::Point::Instruction(instruction)));

        assert_eq!(
            targets,
            [
                &mir::Resolution {
                    functions: vec![first],
                    is_open: false
                },
                &mir::Resolution {
                    functions: vec![second],
                    is_open: false
                },
            ]
        );
    }

    /// Preserve indirect dispatch when blackBox conceals a callback from optimization.
    #[test]
    fn test_preserve_indirect_call_through_black_box() {
        let module = TestModule::new(
            r#"
function callee(): void {
entry:
    return
}

function test(): void {
entry:
    v0: fn() => void = function.address callee
    call.indirect v0(): () => void
    v1: fn() => void = intrinsic.hint.blackBox(v0)
    call.indirect v1(): () => void
    return
}
"#,
        );
        let function = module.entry_function_id();
        let block = module.tree.get(function).block(0);
        let resolution = mir::ResolutionTable::analyse(&module.dispatch, None, &module.tree);
        let callee = module.function_id_by_name("callee");
        let targets = [
            module.tree.get(block).instructions[1],
            module.tree.get(block).instructions[3],
        ]
        .map(|instruction| resolution.resolution(mir::Point::Instruction(instruction)));

        assert_eq!(
            targets,
            [
                &mir::Resolution {
                    functions: vec![callee],
                    is_open: false
                },
                &mir::Resolution {
                    functions: vec![],
                    is_open: true
                },
            ]
        );
    }

    /// Preserve function targets through bitcasts and leave numeric conversions unresolved.
    #[test]
    fn test_distinguish_bitcasts_from_numeric_conversions() {
        let program = TestModule::new(
            r#"
function callee(): void {
entry:
    return
}

function test(): void {
entry:
    v0: fn() => void = function.address callee
    v1: uint64 = cast.bit v0 -> uint64
    v2: fn() => void = cast.bit v1 -> fn() => void
    call.indirect v2(): () => void
    v3: uint32 = cast.intToInt v1 -> uint32
    v4: uint64 = cast.intToInt v3 -> uint64
    v5: fn() => void = cast.bit v4 -> fn() => void
    call.indirect v5(): () => void
    return
}
"#,
        );
        let function = program.entry_function_id();
        let block = program.tree.get(function).block(0);
        let resolution = mir::ResolutionTable::analyse(&program.dispatch, None, &program.tree);
        let targets = [
            program.tree.get(block).instructions[3],
            program.tree.get(block).instructions[7],
        ]
        .map(|instruction| resolution.resolution(mir::Point::Instruction(instruction)));
        let callee = program.function_id_by_name("callee");

        assert_eq!(
            targets,
            [
                &mir::Resolution {
                    functions: vec![callee],
                    is_open: false
                },
                &mir::Resolution {
                    functions: vec![],
                    is_open: true
                },
            ]
        );
    }

    /// Virtual calls resolve when receiver type and virtual table are closed.
    #[test]
    fn test_resolve_virtual_call() {
        let mut program = TestModule::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let callee = program.function_id_by_name("callee");
        let test = program.function_id_by_name("test");
        let function = test;
        let class = program.tree.get(function).parameters[0].ty;
        let callsite = mir::Point::Instruction(
            program
                .tree
                .get(program.tree.get(function).block(0))
                .instructions[0],
        );

        // attach the exact virtual table needed by the callsite
        program.dispatch.insert_virtual_table(mir::VirtualTable {
            concrete: class,
            methods: vec![callee],
        });

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert_eq!(resolution.target(callsite), Some(callee));
    }

    /// Dynamic calls resolve when receiver and constraint tables are closed.
    #[test]
    fn test_resolve_dynamic_call() {
        let mut program = TestModule::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.dynamic v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let callee = program.function_id_by_name("callee");
        let test = program.function_id_by_name("test");
        let function = test;
        let concrete = program.tree.get(function).parameters[0].ty;
        let constraint = concrete;
        let callsite = mir::Point::Instruction(
            program
                .tree
                .get(program.tree.get(function).block(0))
                .instructions[0],
        );

        // attach the concrete dynamic table selected by receiver and constraint
        program.dispatch.insert_dynamic_table(mir::DynamicTable {
            concrete,
            constraint,
            entries: vec![mir::DynamicEntry::Function { function: callee }],
            names: Vec::new(),
        });

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert_eq!(resolution.target(callsite), Some(callee));
    }

    /// Recompute dispatch targets and call edges after receiver types change.
    #[test]
    fn test_invalidate_receiver_types() {
        let mut program = TestModule::new(
            r#"
function first(): int32 {
entry:
    v0: int32 = 1
    return v0
}

function second(): int32 {
entry:
    v0: int32 = 2
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.dynamic v0, int32, 0(): () => int32
    return v1
}
"#,
        );
        let first = program.function_id_by_name("first");
        let second = program.function_id_by_name("second");
        let function = program.entry_function_id();
        let declaration = program.tree.get(function);
        let concrete = declaration.parameters[0].ty;
        let constraint = concrete;
        let callsite =
            mir::Point::Instruction(program.tree.get(declaration.block(0)).instructions[0]);
        let receiver_type = program.tree.intern_type(mir::Type::Boolean, mir::Copy::Yes);

        // select different callees for the two receiver types
        for (concrete, callee) in [(concrete, first), (receiver_type, second)] {
            program.dispatch.insert_dynamic_table(mir::DynamicTable {
                concrete,
                constraint,
                entries: vec![mir::DynamicEntry::Function { function: callee }],
                names: Vec::new(),
            });
        }
        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);
        let calls = analyses.call(&program.tree, &program.dispatch);
        assert_eq!(resolution.target(callsite), Some(first));
        assert_eq!(
            calls
                .outgoing(function)
                .iter()
                .map(|edge| edge.callee)
                .collect::<Vec<_>>(),
            vec![first]
        );

        // change the receiver's declared and recorded types
        let body = program.tree.get_mut(function);
        let receiver = body.parameters[0].value;
        body.parameters[0].ty = receiver_type;
        let mut value_types = body.value_types().to_vec();
        value_types[receiver.id() as usize] = Some(receiver_type);
        body.replace_value_types(value_types);
        let entry = body.entry().expect("function entry");
        program.tree.get_mut(entry).parameters[0].ty = receiver_type;
        analyses.invalidate(mir::Mutation::LAYOUT);

        // require both analyses to select the new receiver's implementation
        let resolution = analyses.resolution(&program.tree, &program.dispatch);
        let calls = analyses.call(&program.tree, &program.dispatch);
        assert_eq!(resolution.target(callsite), Some(second));
        assert_eq!(
            calls
                .outgoing(function)
                .iter()
                .map(|edge| edge.callee)
                .collect::<Vec<_>>(),
            vec![second]
        );
    }

    /// Virtual calls do not resolve without a dispatch table.
    #[test]
    fn test_leave_missing_virtual_table_open() {
        let program = TestModule::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        let block = program
            .tree
            .get(program.entry_block_id(program.entry_function_id()));
        let point = mir::Point::Instruction(block.instructions[0]);
        assert_eq!(
            resolution.resolution(point),
            &mir::Resolution {
                functions: vec![],
                is_open: true
            }
        );
    }

    /// Preserve all known function targets selected by branch arguments.
    #[test]
    fn test_resolve_merged_function_addresses() {
        let program = TestModule::new(
            r#"
function alpha(): void {
entry:
    return
}

function beta(): void {
entry:
    return
}

function test(v0: boolean): void {
entry(v0: boolean):
    v1: fn() => void = function.address alpha
    v2: fn() => void = function.address beta
    branch v0 => join(v1) | join(v2)

join(v3: fn() => void):
    call.indirect v3(): () => void
    return
}
"#,
        );
        let function = program.entry_function_id();
        let join = program.tree.get(program.tree.get(function).blocks()[1]);
        let point = mir::Point::Instruction(join.instructions[0]);
        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);
        let calls = analyses.call(&program.tree, &program.dispatch);
        let mut functions = vec![
            program.function_id_by_name("alpha"),
            program.function_id_by_name("beta"),
        ];
        functions.sort_unstable();

        assert_eq!(
            resolution.resolution(point),
            &mir::Resolution {
                functions: functions.clone(),
                is_open: false
            }
        );
        assert_eq!(resolution.target(point), None);
        assert_eq!(
            calls
                .outgoing(function)
                .iter()
                .map(|edge| edge.callee)
                .collect::<Vec<_>>(),
            functions
        );
        assert_eq!(calls.open_callsites(function), &[]);
    }

    /// Preserve known targets when an indirect call can also select an incoming function.
    #[test]
    fn test_preserve_known_targets_of_open_calls() {
        let program = TestModule::new(
            r#"
function callee(): void {
entry:
    return
}

function test(v0: boolean, v1: fn() => void): void {
entry(v0: boolean, v1: fn() => void):
    v2: fn() => void = function.address callee
    v3: fn() => void = select v0, v1, v2
    call.indirect v3(): () => void
    return
}
"#,
        );
        let function = program.entry_function_id();
        let entry = program.tree.get(program.entry_block_id(function));
        let point = mir::Point::Instruction(entry.instructions[2]);
        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);
        let calls = analyses.call(&program.tree, &program.dispatch);
        let callee = program.function_id_by_name("callee");

        assert_eq!(
            resolution.resolution(point),
            &mir::Resolution {
                functions: vec![callee],
                is_open: true
            }
        );
        assert_eq!(
            calls
                .outgoing(function)
                .iter()
                .map(|edge| edge.callee)
                .collect::<Vec<_>>(),
            vec![callee]
        );
        assert_eq!(
            calls.open_callsites(function),
            &[mir::OpenCallSite {
                caller: function,
                callsite: point,
                dispatch: mir::CallDispatch::Indirect
            }]
        );
    }

    /// Resolve virtual calls from the concrete type created by an allocation.
    #[test]
    fn test_resolve_allocated_receiver() {
        let mut program = TestModule::new(
            r#"
type Object { }

function method(v0: ref<Object, managed, mutable, local>): void {
entry(v0: ref<Object, managed, mutable, local>):
    return
}

function test(): void {
entry:
    v0: ref<Object, managed, mutable, local> = new.zeroed Object
    call.virtual v0, Object, 0(v0): (ref<Object, managed, mutable, local>) => void
    return
}
"#,
        );
        let function = program.entry_function_id();
        let block = program.entry_block_id(function);
        let point = mir::Point::Instruction(program.tree.get(block).instructions[1]);
        let mir::Instruction::NewZeroed {
            storage_type: concrete,
            ..
        } = program.tree.get(program.tree.get(block).instructions[0])
        else {
            panic!("expected Object allocation");
        };
        let concrete = *concrete;
        let callee = program.function_id_by_name("method");
        program.dispatch.insert_virtual_table(mir::VirtualTable {
            concrete,
            methods: vec![callee],
        });
        let resolution = mir::ResolutionTable::analyse(&program.dispatch, None, &program.tree);

        assert_eq!(
            resolution.resolution(point),
            &mir::Resolution {
                functions: vec![callee],
                is_open: false
            }
        );
    }

    /// Resolve closed witness calls through the available implementation table.
    #[test]
    fn test_resolve_available_witness() {
        let program = TestModule::new(
            r#"
type Clone = void;

external function Clone.clone(int32): int32

function implementation(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.witness int32, Clone, Clone.clone(v0): (int32) => int32
    return v1
}
"#,
        );
        let function = program.entry_function_id();
        let entry = program.tree.get(program.entry_block_id(function));
        let instruction = entry.instructions[0];
        let mir::Instruction::Call { call, .. } = program.tree.get(instruction) else {
            panic!("expected witness call")
        };
        let mir::Callee::Witness {
            receiver,
            interface,
            requirement,
            ..
        } = call.callee
        else {
            panic!("expected witness dispatch")
        };
        let callee = program.function_id_by_name("implementation");
        let mut witnesses = mir::WitnessTable::default();
        witnesses.insert(mir::Witness {
            concrete: receiver,
            constraint: interface,
            functions: vec![mir::WitnessFunction {
                member: destack_core::StringId::for_text("clone"),
                requirement,
                function: callee,
                arguments: Vec::new(),
            }],
            types: Vec::new(),
            constants: Vec::new(),
        });
        let resolution =
            mir::ResolutionTable::analyse(&program.dispatch, Some(&witnesses), &program.tree);

        assert_eq!(
            resolution.resolution(mir::Point::Instruction(instruction)),
            &mir::Resolution {
                functions: vec![callee],
                is_open: false
            }
        );
    }
}
