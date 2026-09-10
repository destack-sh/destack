use std::sync::Arc;

use destack_core::BitSet;
use smallvec::SmallVec;

use crate::{
    AddressKind, Analysis, Block, BlockTarget, ControlTable, DataflowTable, Edge, ForwardTransfer,
    Function, Instruction, Lattice, LocalNodeId, LocalNodeIdAny, MovePathId, MoveTable, Mutation,
    Place, PlaceTable, Projection, Terminator, Tree, Type, Value,
};

/// Move-path initialization across one MIR function.
#[derive(Debug)]
pub struct InitializationTable {
    /// Move paths.
    paths: Arc<MoveTable>,
    /// Places derived by address values.
    places: Arc<PlaceTable>,
    /// Initialization at reachable block entries and exits.
    flow: DataflowTable<InitializationState>,
}

/// One unavailable move path used by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailability {
    /// The path initialization at the use.
    pub initialization: Initialization,
    /// The operation that moved the path.
    pub moved_at: Option<LocalNodeIdAny>,
}

impl InitializationTable {
    /// Build initialization for one function.
    pub fn analyse(
        function: &Function,
        control: &ControlTable,
        paths: Arc<MoveTable>,
        places: Arc<PlaceTable>,
        tree: &Tree,
    ) -> Self {
        let table = Self {
            paths,
            places,
            flow: DataflowTable::new(),
        };
        let entry = table.parameter_state(function);
        let flow = DataflowTable::forward(
            function,
            tree,
            control,
            entry,
            |transfer, mut state, tree| {
                match transfer {
                    // transfer each instruction and terminator in the block
                    ForwardTransfer::Block(block) => {
                        let block = tree.get(block);
                        for instruction in &block.instructions {
                            table.transfer_instruction(*instruction, &mut state, tree);
                        }
                        table.transfer_terminator(block.terminator, &mut state, tree);
                    }
                    // bind move paths carried through the edge
                    ForwardTransfer::Edge { edge, target } => {
                        table.bind(edge, target, &mut state, tree);
                    }
                }

                state
            },
        );

        Self { flow, ..table }
    }

    /// Return initialization at one reachable block entry.
    pub fn entry(&self, block: LocalNodeId<Block>) -> Option<&InitializationState> {
        self.flow.entry(block)
    }

    /// Return initialization at one reachable block exit.
    pub fn exit(&self, block: LocalNodeId<Block>) -> Option<&InitializationState> {
        self.flow.exit(block)
    }

    /// Return unavailable paths read by one instruction.
    pub fn instruction_unavailability(
        &self,
        instruction: &Instruction,
        state: &InitializationState,
        tree: &Tree,
    ) -> SmallVec<[Unavailability; 4]> {
        let mut unavailable = SmallVec::new();

        // handle projected and replacement reads separately
        match instruction {
            Instruction::LocalGet { local, .. } => {
                self.collect_place(&Place::local(*local), state, &mut unavailable);
            }
            Instruction::FieldGet {
                aggregate, field, ..
            } => {
                let projection = Projection::Field { index: *field };
                self.collect_projection(*aggregate, projection, state, &mut unavailable);
            }
            Instruction::FieldAddr {
                aggregate,
                field,
                kind: AddressKind::Borrow,
                ..
            } => {
                let projection = Projection::Field { index: *field };
                let place = self.places.project(*aggregate, projection);
                self.collect_moved_place(&place, state, &mut unavailable);
            }
            Instruction::FieldAddr { aggregate, .. } => {
                self.collect_value(*aggregate, state, &mut unavailable);
            }
            Instruction::LocalAddr {
                local,
                kind: AddressKind::Borrow,
                ..
            } => {
                self.collect_moved_place(&Place::local(*local), state, &mut unavailable);
            }
            // require the owned storage a load reads
            Instruction::Load { pointer, .. } => {
                self.collect_value(*pointer, state, &mut unavailable);
                if let Some(path) = self.paths.pointee(*pointer)
                    && let Some(path) = state.unavailable(path, &self.paths)
                {
                    unavailable.push(state.unavailability(path));
                }
            }
            // require the reference a release returns, down to its own storage
            Instruction::Release { value } => {
                if let Some(path) = self.paths.value(*value)
                    && let Some(path) = state.unavailable_shallow(path, &self.paths)
                {
                    unavailable.push(state.unavailability(path));
                }
            }
            Instruction::ElementGet {
                aggregate, index, ..
            } => {
                let projection = Projection::Element { index: *index };
                self.collect_projection(*aggregate, projection, state, &mut unavailable);
            }
            Instruction::ElementAddr { base, index, .. } => {
                let projection = Projection::Index { index: *index };
                self.collect_projection(*base, projection, state, &mut unavailable);
                self.collect_value(*index, state, &mut unavailable);
            }
            Instruction::FieldSet {
                aggregate,
                field,
                value,
                ..
            } => {
                let projection = Projection::Field { index: *field };
                self.collect_replacement(*aggregate, projection, state, &mut unavailable);
                self.collect_value(*value, state, &mut unavailable);
            }
            Instruction::ElementSet {
                aggregate,
                index,
                value,
                ..
            } => {
                let projection = Projection::Element { index: *index };
                self.collect_replacement(*aggregate, projection, state, &mut unavailable);
                self.collect_value(*value, state, &mut unavailable);
            }
            Instruction::VariantPayloadAddr { variant, case, .. } => {
                let projection = Projection::Variant { case: *case };
                self.collect_projection(*variant, projection, state, &mut unavailable);
            }
            Instruction::SliceView {
                destination,
                source,
                start,
                length,
                ..
            } => {
                let projection = Projection::Slice {
                    start: *start,
                    length: *length,
                };
                let place = self.places.project(*source, projection);

                // require the source value when the slice defines a view over an untracked base
                if self.paths.place(&place) == self.paths.value(*destination) {
                    self.collect_value(*source, state, &mut unavailable);
                }
                // otherwise require the projected place
                else {
                    self.collect_place(&place, state, &mut unavailable);
                }
                self.collect_value(*start, state, &mut unavailable);
                self.collect_value(*length, state, &mut unavailable);
            }
            _ => {
                for value in instruction.reads(tree) {
                    self.collect_value(value, state, &mut unavailable);
                }
            }
        }

        unavailable
    }

    /// Return unavailable paths read by one terminator.
    pub fn terminator_unavailability(
        &self,
        terminator: &Terminator,
        state: &InitializationState,
        tree: &Tree,
    ) -> SmallVec<[Unavailability; 4]> {
        let mut unavailable = SmallVec::new();

        // require every value read by the terminator
        for value in terminator.reads(tree) {
            self.collect_value(value, state, &mut unavailable);
        }

        unavailable
    }

    /// Collect one unavailable projected place.
    fn collect_projection(
        &self,
        base: Value,
        projection: Projection,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        let place = self.places.project(base, projection);
        self.collect_place(&place, state, unavailable);
    }

    /// Collect one unavailable place.
    fn collect_place(
        &self,
        place: &Place,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        let Some(path) = self.paths.containing(place) else {
            return;
        };
        let Some(path) = state.unavailable(path, &self.paths) else {
            return;
        };

        unavailable.push(state.unavailability(path));
    }

    /// Collect one place a move made unavailable.
    fn collect_moved_place(
        &self,
        place: &Place,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        let Some(path) = self.paths.containing(place) else {
            return;
        };
        let Some(path) = state.unavailable(path, &self.paths) else {
            return;
        };
        if state.moved_at(path).is_none() {
            return;
        }

        unavailable.push(state.unavailability(path));
    }

    /// Collect an unavailable aggregate outside one replacement.
    fn collect_replacement(
        &self,
        aggregate: Value,
        projection: Projection,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        let Some(parent) = self.paths.value(aggregate) else {
            return;
        };
        let place = self.places.project(aggregate, projection);
        let Some(replacement) = self.paths.place(&place) else {
            unreachable!("aggregate replacement has no move path");
        };
        let Some(path) = state.unavailable_replacement(parent, replacement, &self.paths) else {
            return;
        };

        unavailable.push(state.unavailability(path));
    }

    /// Collect one unavailable value.
    fn collect_value(
        &self,
        value: Value,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        let Some(path) = self.paths.value(value) else {
            return;
        };
        let Some(path) = state.unavailable(path, &self.paths) else {
            return;
        };

        unavailable.push(state.unavailability(path));
    }

    /// Transfer one instruction.
    pub fn transfer_instruction(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        state: &mut InitializationState,
        tree: &Tree,
    ) {
        let instruction = tree.get(instruction_id);
        let anchor = instruction_id.into_any();

        // move storage read into a move-only destination
        match instruction {
            Instruction::LocalGet { destination, local }
                if self.paths.value(*destination).is_some() =>
            {
                if let Some(path) = self.paths.local(*local) {
                    state.move_path(path, anchor, &self.paths);
                }
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } if self.paths.value(*destination).is_some() => {
                let projection = Projection::Field { index: *field };
                self.uninitialize_projection(*aggregate, projection, anchor, state, tree);
            }
            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } if self.paths.value(*destination).is_some() => {
                let projection = Projection::Element { index: *index };
                self.uninitialize_projection(*aggregate, projection, anchor, state, tree);
            }
            Instruction::VariantPayload {
                destination,
                variant,
                ..
            } if self.paths.value(*destination).is_some() => {
                self.uninitialize_value(*variant, anchor, state);
            }
            // move the owned storage a load reads
            Instruction::Load {
                destination,
                pointer,
                ..
            } if self.paths.value(*destination).is_some() => {
                if let Some(path) = self.paths.pointee(*pointer) {
                    state.move_path(path, anchor, &self.paths);
                }
            }
            _ => {}
        }

        // consume values transferred into instruction-owned storage
        for value in instruction.consumes(tree) {
            self.uninitialize_value(value, anchor, state);
        }

        // initialize the storage a callee writes through its arguments
        if let Instruction::Call { call, .. } = instruction {
            for argument in tree.get_values(call.arguments) {
                if let Some(path) = self.paths.initialized_pointee(*argument) {
                    state.initialize(path, &self.paths);
                }
            }
        }

        // initialize storage defined by the instruction
        if let Instruction::LocalSet { local, .. } = instruction
            && let Some(path) = self.paths.local(*local)
        {
            state.initialize(path, &self.paths);
        }
        if let Instruction::Store { pointer, .. } = instruction
            && let Some(path) = self.paths.pointee(*pointer)
        {
            state.initialize(path, &self.paths);
        }
        if let Some(destination) = instruction.destination()
            && let Some(path) = self.paths.value(destination)
        {
            state.initialize(path, &self.paths);
        }
    }

    /// Transfer one terminator.
    pub fn transfer_terminator(
        &self,
        terminator_id: LocalNodeId<Terminator>,
        state: &mut InitializationState,
        tree: &Tree,
    ) {
        let terminator = tree.get(terminator_id);
        let anchor = terminator_id.into_any();

        // consume values transferred out of the function
        for value in terminator.consumes(tree) {
            self.uninitialize_value(value, anchor, state);
        }
    }

    /// Transfer initialization through one control-flow edge.
    fn bind(&self, edge: Edge, target: &BlockTarget, state: &mut InitializationState, tree: &Tree) {
        let block = tree.get(edge.source);
        let terminator = tree.get(block.terminator);
        let arguments = target.arguments(tree);
        let parameters = terminator
            .target_parameters(tree, edge.successor, target)
            .unwrap_or_else(|| unreachable!("block target has invalid argument count"));
        let anchor = block.terminator.into_any();

        // collect every moved argument before assigning any successor parameter
        let bindings = parameters
            .iter()
            .zip(arguments)
            .filter_map(|(parameter, &argument)| {
                let argument = self.paths.value(argument)?;
                let parameter = self.paths.value(parameter.value)?;

                (argument != parameter).then_some((argument, parameter))
            })
            .collect::<Vec<_>>();
        state.bind(&bindings, anchor, &self.paths);

        // initialize the values produced by the selected successor edge
        let count = terminator.target_result_count(tree, edge.successor);
        for parameter in &tree.get(target.block).parameters[..count] {
            if let Some(path) = self.paths.value(parameter.value) {
                state.initialize(path, &self.paths);
            }
        }
    }

    /// Return initialization at function entry.
    fn parameter_state(&self, function: &Function) -> InitializationState {
        let mut state = InitializationState::new(self.paths.len());

        // initialize move-only parameters
        for parameter in &function.parameters {
            if let Some(path) = self.paths.value(parameter.value) {
                state.initialize(path, &self.paths);
            }
        }

        state
    }

    /// Mark one value unavailable.
    fn uninitialize_value(
        &self,
        value: Value,
        anchor: LocalNodeIdAny,
        state: &mut InitializationState,
    ) {
        if let Some(path) = self.paths.value(value) {
            state.move_path(path, anchor, &self.paths);
        }
    }

    /// Mark one projected place unavailable.
    fn uninitialize_projection(
        &self,
        aggregate: Value,
        projection: Projection,
        anchor: LocalNodeIdAny,
        state: &mut InitializationState,
        tree: &Tree,
    ) {
        let place = self.places.project(aggregate, projection);
        let is_variant = self
            .paths
            .value(aggregate)
            .is_some_and(|path| matches!(tree.get(self.paths.get(path).ty), Type::Variant { .. }));
        let path = if is_variant {
            self.paths.value(aggregate)
        } else {
            self.paths.containing(&place)
        };
        if let Some(path) = path {
            state.move_path(path, anchor, &self.paths);
        }
    }
}

impl Analysis for InitializationTable {
    const INVALIDATED_BY: Mutation = Mutation::VALUE
        .union(Mutation::CONTROL)
        .union(Mutation::LAYOUT);
}

/// Initialization of every move path at one program point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializationState {
    /// Paths initialized on every incoming edge.
    initialized: BitSet,
    /// Paths initialized on at least one incoming edge.
    maybe_initialized: BitSet,
    /// Operations that made paths unavailable.
    moved_at: Vec<Option<LocalNodeIdAny>>,
}

impl InitializationState {
    /// Create uninitialized state for every move path.
    fn new(path_count: usize) -> Self {
        Self {
            initialized: BitSet::new(path_count),
            maybe_initialized: BitSet::new(path_count),
            moved_at: vec![None; path_count],
        }
    }

    /// Return one path's initialization.
    pub fn get(&self, path: MovePathId) -> Initialization {
        if self.initialized.contains(path.index()) {
            Initialization::Initialized
        } else if self.maybe_initialized.contains(path.index()) {
            Initialization::MaybeInitialized
        } else {
            Initialization::Uninitialized
        }
    }

    /// Return the operation that made one path unavailable.
    pub fn moved_at(&self, path: MovePathId) -> Option<LocalNodeIdAny> {
        self.moved_at[path.index()]
    }

    /// Return one unavailable path description.
    fn unavailability(&self, path: MovePathId) -> Unavailability {
        Unavailability {
            initialization: self.get(path),
            moved_at: self.moved_at(path),
        }
    }

    /// Return an unavailable path required by one complete use.
    pub fn unavailable(&self, path: MovePathId, paths: &MoveTable) -> Option<MovePathId> {
        let mut current = Some(path);

        // require the path and every containing path
        while let Some(path) = current {
            if self.get(path) != Initialization::Initialized {
                return Some(path);
            }

            current = paths.get(path).parent;
        }

        // require every structural child for a whole-place use
        paths
            .descendants(path)
            .skip(1)
            .find(|child| self.get(*child) != Initialization::Initialized)
    }

    /// Return an unavailable path among one path and the paths containing it.
    pub fn unavailable_shallow(&self, path: MovePathId, paths: &MoveTable) -> Option<MovePathId> {
        let mut current = Some(path);

        // require the path and every containing path
        while let Some(path) = current {
            if self.get(path) != Initialization::Initialized {
                return Some(path);
            }

            current = paths.get(path).parent;
        }

        None
    }

    /// Return an unavailable path outside one replaced child.
    pub fn unavailable_replacement(
        &self,
        parent: MovePathId,
        replacement: MovePathId,
        paths: &MoveTable,
    ) -> Option<MovePathId> {
        let mut current = paths.get(parent).parent;

        // require storage containing the reconstructed aggregate
        while let Some(path) = current {
            if self.get(path) != Initialization::Initialized {
                return Some(path);
            }

            current = paths.get(path).parent;
        }

        // require every path outside the replacement subtree
        paths.descendants(parent).skip(1).find(|path| {
            *path != replacement
                && !paths.is_ancestor(replacement, *path)
                && self.get(*path) != Initialization::Initialized
        })
    }

    /// Mark one path and every child initialized.
    fn initialize(&mut self, path: MovePathId, paths: &MoveTable) {
        for path in paths.descendants(path) {
            self.set(path, Initialization::Initialized);
            self.moved_at[path.index()] = None;
        }

        // rebuild each complete containing aggregate
        let mut parent = paths.get(path).parent;
        while let Some(current) = parent {
            let is_initialized = paths
                .children(current)
                .iter()
                .all(|child| self.is_initialized(*child, paths));
            if !is_initialized {
                break;
            }

            self.set(current, Initialization::Initialized);
            self.moved_at[current.index()] = None;
            parent = paths.get(current).parent;
        }
    }

    /// Mark one path and every child uninitialized.
    pub fn uninitialize(&mut self, path: MovePathId, paths: &MoveTable) {
        for path in paths.descendants(path) {
            self.set(path, Initialization::Uninitialized);
            self.moved_at[path.index()] = None;
        }
    }

    /// Mark one path and every child moved.
    fn move_path(&mut self, path: MovePathId, moved_at: LocalNodeIdAny, paths: &MoveTable) {
        for path in paths.descendants(path) {
            self.set(path, Initialization::Uninitialized);
            self.moved_at[path.index()] = Some(moved_at);
        }
    }

    /// Move edge arguments into successor parameters simultaneously.
    fn bind(
        &mut self,
        bindings: &[(MovePathId, MovePathId)],
        moved_at: LocalNodeIdAny,
        paths: &MoveTable,
    ) {
        // read every source path before moving its storage
        let mut states = Vec::new();
        for &(argument, parameter) in bindings {
            for path in paths.descendants(argument) {
                let parameter = paths.map(path, argument, parameter);
                states.push((parameter, self.get(path), self.moved_at(path)));
            }
        }

        // consume source storage before initializing any destination
        for &(argument, _) in bindings {
            self.move_path(argument, moved_at, paths);
        }

        // assign each destination from the original edge state
        for (parameter, state, moved_at) in states {
            self.set(parameter, state);
            self.moved_at[parameter.index()] = moved_at;
        }
    }

    /// Return whether one complete path tree is initialized.
    pub fn is_initialized(&self, path: MovePathId, paths: &MoveTable) -> bool {
        self.get(path) == Initialization::Initialized
            && paths
                .children(path)
                .iter()
                .all(|child| self.is_initialized(*child, paths))
    }

    /// Set one path's initialization.
    fn set(&mut self, path: MovePathId, initialization: Initialization) {
        let index = path.index();

        match initialization {
            Initialization::Uninitialized => {
                self.initialized.remove(index);
                self.maybe_initialized.remove(index);
            }
            Initialization::Initialized => {
                self.initialized.insert(index);
                self.maybe_initialized.insert(index);
            }
            Initialization::MaybeInitialized => {
                self.initialized.remove(index);
                self.maybe_initialized.insert(index);
            }
        }
    }
}

impl Lattice for InitializationState {
    /// Merge initialization reaching one program point.
    fn meet(&self, other: &Self) -> Self {
        debug_assert_eq!(self.initialized.len(), other.initialized.len());
        let mut initialized = self.initialized.clone();
        initialized.intersect_with(&other.initialized);

        let mut maybe_initialized = self.maybe_initialized.clone();
        maybe_initialized.union_with(&other.maybe_initialized);

        // retain one diagnostic origin for each unavailable path
        let moved_at = self
            .moved_at
            .iter()
            .zip(&other.moved_at)
            .map(|(left, right)| left.or(*right))
            .collect();

        Self {
            initialized,
            maybe_initialized,
            moved_at,
        }
    }
}

/// Initialization of one move path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Initialization {
    /// The path cannot be read or dropped.
    Uninitialized,
    /// The path can be read, moved, or dropped.
    Initialized,
    /// The path is initialized on only some incoming control flow paths.
    MaybeInitialized,
}

#[cfg(test)]
mod tests {
    use crate::Initialization::{Initialized, MaybeInitialized, Uninitialized};
    use crate::Value;
    use crate::analyses::tests::TestModule;

    /// Merge local initialization from every reachable branch.
    #[test]
    fn test_merge_local_initialization() {
        let program = TestModule::new(
            r#"
function partial(v0: boolean, v1: ref<int32, unique, mutable, local>): void {
    local l0: ref<int32, unique, mutable, local>

entry(v0: boolean, v1: ref<int32, unique, mutable, local>):
    branch v0 => left | right

left:
    local.set l0, v1
    jump join

right:
    jump join

join:
    return
}

function complete(v0: boolean, v1: ref<int32, unique, mutable, local>): void {
    local l0: ref<int32, unique, mutable, local>

entry(v0: boolean, v1: ref<int32, unique, mutable, local>):
    branch v0 => left | right

left:
    local.set l0, v1
    jump join

right:
    local.set l0, v1
    jump join

join:
    return
}
"#,
        );

        for (name, expected) in [
            (
                "partial",
                [
                    (Uninitialized, Uninitialized),
                    (Uninitialized, Initialized),
                    (Uninitialized, Uninitialized),
                    (MaybeInitialized, MaybeInitialized),
                ],
            ),
            (
                "complete",
                [
                    (Uninitialized, Uninitialized),
                    (Uninitialized, Initialized),
                    (Uninitialized, Initialized),
                    (Initialized, Initialized),
                ],
            ),
        ] {
            let function = program.tree.get(program.function_id_by_name(name));
            let mut analyses = program.function_analyses();
            let table = analyses.initialization(function, &program.tree);
            let local = table.paths.local(function.locals()[0]).unwrap();
            let actual = function
                .blocks()
                .iter()
                .map(|&block| {
                    (
                        table.entry(block).unwrap().get(local),
                        table.exit(block).unwrap().get(local),
                    )
                })
                .collect::<Vec<_>>();

            assert_eq!(actual, expected, "{name}");
        }
    }

    /// Propagate a local move through the backedge and into the loop exit.
    #[test]
    fn test_propagate_loop_moves() {
        let program = TestModule::new(
            r#"
function test(v0: ref<int32, unique, mutable, local>, v1: boolean): void {
    local l0: ref<int32, unique, mutable, local>

entry(v0: ref<int32, unique, mutable, local>, v1: boolean):
    local.set l0, v0
    jump header

header:
    v2: ref<int32, unique, mutable, local> = local.get l0
    branch v1 => header | done

done:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.initialization(function, &program.tree);
        let local = table.paths.local(function.locals()[0]).unwrap();
        let moved = program.tree.get(function.block(1)).instructions[0].into_any();
        let actual = function
            .blocks()
            .iter()
            .map(|&block| {
                let entry = table.entry(block).unwrap();
                let exit = table.exit(block).unwrap();

                (
                    entry.get(local),
                    entry.moved_at(local),
                    exit.get(local),
                    exit.moved_at(local),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                (Uninitialized, None, Initialized, None),
                (MaybeInitialized, Some(moved), Uninitialized, Some(moved)),
                (Uninitialized, Some(moved), Uninitialized, Some(moved)),
            ]
        );
    }

    /// Ignore an unreachable initializer even when its block precedes the function entry.
    #[test]
    fn test_ignore_unreachable_initialization() {
        let mut program = TestModule::new(
            r#"
function test(v0: ref<int32, unique, mutable, local>): void {
    local l0: ref<int32, unique, mutable, local>

entry(v0: ref<int32, unique, mutable, local>):
    jump join

dead:
    local.set l0, v0
    jump join

join:
    return
}
"#,
        );
        let id = program.entry_function_id();
        let mut function = program.tree.get(id).clone();
        let [entry, dead, join]: [_; 3] = function.blocks().try_into().unwrap();
        function.replace_blocks(vec![dead, entry, join], &program.tree);
        program.tree.set(id, function);

        let function = program.tree.get(id);
        let mut analyses = program.function_analyses();
        let table = analyses.initialization(function, &program.tree);
        let local = table.paths.local(function.locals()[0]).unwrap();
        let actual = [entry, dead, join].map(|block| {
            (
                table.entry(block).map(|state| state.get(local)),
                table.exit(block).map(|state| state.get(local)),
            )
        });

        assert_eq!(
            actual,
            [
                (Some(Uninitialized), Some(Uninitialized)),
                (None, None),
                (Some(Uninitialized), Some(Uninitialized)),
            ]
        );
    }

    /// Preserve both owned parameters when a backedge swaps their values.
    #[test]
    fn test_preserve_swapped_block_arguments() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: ref<int32, unique, mutable, local>, v2: ref<int32, unique, mutable, local>): void {
entry(v0: boolean, v1: ref<int32, unique, mutable, local>, v2: ref<int32, unique, mutable, local>):
    branch v0 => entry(v0, v2, v1) | done

done:
    release v1
    release v2
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(function, &program.tree);
        let table = analyses.initialization(function, &program.tree);
        let values = [Value(1), Value(2)].map(|value| paths.value(value).unwrap());
        let actual = function
            .blocks()
            .iter()
            .map(|&block| {
                values.map(|value| {
                    (
                        table.entry(block).unwrap().get(value),
                        table.exit(block).unwrap().get(value),
                    )
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                [(Initialized, Initialized), (Initialized, Initialized)],
                [(Initialized, Uninitialized), (Initialized, Uninitialized)],
            ]
        );
    }

    /// Initialize values produced on successful allocation and invoke edges.
    #[test]
    fn test_initialize_generated_edge_results() {
        let program = TestModule::new(
            r#"
external function make(): ref<int32, unique, mutable, local>

function allocate(): void {
entry:
    new.zeroed.try int32 => done | failure

done(v0: ref<int32, unique, mutable, local>):
    release v0
    return

failure:
    return
}

function construct(): void {
entry:
    invoke make(): () => ref<int32, unique, mutable, local> => done | failure

done(v0: ref<int32, unique, mutable, local>):
    release v0
    return

failure:
    unwind.resume
}
"#,
        );
        for name in ["allocate", "construct"] {
            let function = program.tree.get(program.function_id_by_name(name));
            let mut analyses = program.function_analyses();
            let paths = analyses.moves(function, &program.tree);
            let table = analyses.initialization(function, &program.tree);
            let result = paths.value(Value(0)).unwrap();
            let actual = function
                .blocks()
                .iter()
                .map(|&block| {
                    (
                        table.entry(block).unwrap().get(result),
                        table.exit(block).unwrap().get(result),
                    )
                })
                .collect::<Vec<_>>();

            assert_eq!(
                actual,
                [
                    (Uninitialized, Uninitialized),
                    (Initialized, Uninitialized),
                    (Uninitialized, Uninitialized),
                ],
                "{name}"
            );
        }
    }
}
