use std::sync::Arc;

use destack_core::BitSet;
use smallvec::SmallVec;

use crate::{
    Block, BlockTarget, ControlTable, Edge, Function, Instruction, LocalNodeId, LocalNodeIdAny,
    MovePathId, MoveTable, Place, PlaceTable, Projection, Terminator, Tree, Type, Value,
};

use super::{Analysis, Dataflow, ForwardTransfer, FunctionCache, Lattice, Mutation};

/// Move-path initialization across one MIR function.
#[derive(Debug)]
pub struct InitializationTable {
    /// Move paths.
    paths: Arc<MoveTable>,
    /// Places derived by address values.
    places: Arc<PlaceTable>,
    /// Initialization at reachable block entries and exits.
    flow: Dataflow<InitializationState>,
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
    pub fn build(
        function: &Function,
        tree: &Tree,
        control: &ControlTable,
        paths: Arc<MoveTable>,
        places: Arc<PlaceTable>,
    ) -> Self {
        let table = Self {
            paths,
            places,
            flow: Dataflow::new(),
        };
        let entry = table.parameter_state(function);
        let flow = Dataflow::forward(
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
            }
            | Instruction::FieldAddr {
                aggregate, field, ..
            } => {
                let projection = Projection::Field { index: *field };
                self.collect_projection(*aggregate, projection, state, &mut unavailable);
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

                // require the source value where the projected place names the
                //  view this instruction defines over an untracked base
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
            _ => {}
        }

        // consume values transferred into instruction-owned storage
        for value in instruction.consumes(tree) {
            self.uninitialize_value(value, anchor, state);
        }

        // initialize storage defined by the instruction
        if let Instruction::LocalSet { local, .. } = instruction
            && let Some(path) = self.paths.local(*local)
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

        // bind move-only arguments to successor parameters
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            let Some(argument) = self.paths.value(argument) else {
                continue;
            };
            let Some(parameter) = self.paths.value(parameter.value) else {
                continue;
            };

            state.bind(argument, parameter, anchor, &self.paths);
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

impl InitializationTable {
    /// Compute initialization for one function.
    pub(crate) fn compute(function: &Function, tree: &Tree, analyses: &mut FunctionCache) -> Self {
        let control = analyses.control(function, tree);
        let paths = analyses.moves(function, tree);
        let places = analyses.place(function, tree);

        Self::build(function, tree, &control, paths, places)
    }
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

    /// Transfer one move path tree into another.
    fn bind(
        &mut self,
        argument: MovePathId,
        parameter: MovePathId,
        moved_at: LocalNodeIdAny,
        paths: &MoveTable,
    ) {
        if argument == parameter {
            return;
        }

        let states = paths
            .descendants(argument)
            .map(|path| {
                let parameter = paths.map(path, argument, parameter);

                (parameter, self.get(path), self.moved_at(path))
            })
            .collect::<Vec<_>>();

        // transfer each structural path into its matching parameter path
        for (parameter, state, moved_at) in states {
            self.set(parameter, state);
            self.moved_at[parameter.index()] = moved_at;
        }

        self.move_path(argument, moved_at, paths);
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
