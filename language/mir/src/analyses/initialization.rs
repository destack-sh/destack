use std::collections::HashMap;
use std::sync::Arc;

use destack_core::BitSet;

use crate as mir;

use super::{Analysis, Dataflow, ForwardTransfer, FunctionCache, Lattice, Mutation};

/// Move-path initialization across one MIR function.
#[derive(Debug)]
pub struct InitializationTable {
    /// Canonical move paths.
    paths: Arc<mir::MoveTable>,
    /// Canonical places.
    places: Arc<mir::PlaceTable>,
    /// Initialization at reachable block entries and exits.
    flow: Dataflow<InitializationState>,
}

impl InitializationTable {
    /// Build initialization for one function.
    pub fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        control: &mir::ControlTable,
        paths: Arc<mir::MoveTable>,
        places: Arc<mir::PlaceTable>,
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
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&InitializationState> {
        self.flow.entry(block)
    }

    /// Return initialization at one reachable block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&InitializationState> {
        self.flow.exit(block)
    }

    /// Transfer one instruction.
    pub fn transfer_instruction(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        state: &mut InitializationState,
        tree: &mir::Tree,
    ) {
        let instruction = tree.get(instruction_id);
        let anchor = instruction_id.into_any();

        // move storage read into a move-only destination
        match instruction {
            mir::Instruction::LocalGet { destination, local }
                if self.paths.value(*destination).is_some() =>
            {
                if let Some(path) = self.paths.local(*local) {
                    state.move_path(path, anchor, &self.paths);
                }
            }
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } if self.paths.value(*destination).is_some() => {
                let projection = mir::Projection::Field { index: *field };
                self.uninitialize_projection(*aggregate, projection, anchor, state, tree);
            }
            mir::Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } if self.paths.value(*destination).is_some() => {
                let projection = mir::Projection::Element { index: *index };
                self.uninitialize_projection(*aggregate, projection, anchor, state, tree);
            }
            mir::Instruction::VariantPayload {
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
        if let mir::Instruction::LocalSet { local, .. } = instruction
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
        terminator_id: mir::LocalNodeId<mir::Terminator>,
        state: &mut InitializationState,
        tree: &mir::Tree,
    ) {
        let terminator = tree.get(terminator_id);
        let anchor = terminator_id.into_any();

        // consume values transferred out of the function
        for value in terminator.consumes(tree) {
            self.uninitialize_value(value, anchor, state);
        }
    }

    /// Transfer initialization through one control-flow edge.
    fn bind(
        &self,
        edge: mir::Edge,
        target: &mir::BlockTarget,
        state: &mut InitializationState,
        tree: &mir::Tree,
    ) {
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
    fn parameter_state(&self, function: &mir::Function) -> InitializationState {
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
        value: mir::Value,
        anchor: mir::LocalNodeIdAny,
        state: &mut InitializationState,
    ) {
        if let Some(path) = self.paths.value(value) {
            state.move_path(path, anchor, &self.paths);
        }
    }

    /// Mark one projected place unavailable.
    fn uninitialize_projection(
        &self,
        aggregate: mir::Value,
        projection: mir::Projection,
        anchor: mir::LocalNodeIdAny,
        state: &mut InitializationState,
        tree: &mir::Tree,
    ) {
        let place = self.places.project(aggregate, projection);
        let is_variant = self.paths.value(aggregate).is_some_and(|path| {
            matches!(tree.get(self.paths.get(path).ty), mir::Type::Variant { .. })
        });
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
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &mut FunctionCache,
    ) -> Self {
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
    moved_at: HashMap<mir::MovePathId, mir::LocalNodeIdAny>,
}

impl InitializationState {
    /// Create uninitialized state for every move path.
    fn new(path_count: usize) -> Self {
        Self {
            initialized: BitSet::new(path_count),
            maybe_initialized: BitSet::new(path_count),
            moved_at: HashMap::new(),
        }
    }

    /// Return one path's initialization.
    pub fn get(&self, path: mir::MovePathId) -> Initialization {
        if self.initialized.contains(path.index()) {
            Initialization::Initialized
        } else if self.maybe_initialized.contains(path.index()) {
            Initialization::MaybeInitialized
        } else {
            Initialization::Uninitialized
        }
    }

    /// Return the operation that made one path unavailable.
    pub fn moved_at(&self, path: mir::MovePathId) -> Option<mir::LocalNodeIdAny> {
        self.moved_at.get(&path).copied()
    }

    /// Return an unavailable path required by one complete use.
    pub fn unavailable(
        &self,
        path: mir::MovePathId,
        paths: &mir::MoveTable,
    ) -> Option<mir::MovePathId> {
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
        parent: mir::MovePathId,
        replacement: mir::MovePathId,
        paths: &mir::MoveTable,
    ) -> Option<mir::MovePathId> {
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
    fn initialize(&mut self, path: mir::MovePathId, paths: &mir::MoveTable) {
        for path in paths.descendants(path) {
            self.set(path, Initialization::Initialized);
            self.moved_at.remove(&path);
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
            self.moved_at.remove(&current);
            parent = paths.get(current).parent;
        }
    }

    /// Mark one path and every child uninitialized.
    pub fn uninitialize(&mut self, path: mir::MovePathId, paths: &mir::MoveTable) {
        for path in paths.descendants(path) {
            self.set(path, Initialization::Uninitialized);
            self.moved_at.remove(&path);
        }
    }

    /// Mark one path and every child moved.
    fn move_path(
        &mut self,
        path: mir::MovePathId,
        at: mir::LocalNodeIdAny,
        paths: &mir::MoveTable,
    ) {
        for path in paths.descendants(path) {
            self.set(path, Initialization::Uninitialized);
            self.moved_at.insert(path, at);
        }
    }

    /// Transfer one move path tree into another.
    fn bind(
        &mut self,
        argument: mir::MovePathId,
        parameter: mir::MovePathId,
        at: mir::LocalNodeIdAny,
        paths: &mir::MoveTable,
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
            if let Some(moved_at) = moved_at {
                self.moved_at.insert(parameter, moved_at);
            } else {
                self.moved_at.remove(&parameter);
            }
        }

        self.move_path(argument, at, paths);
    }

    /// Return whether one complete path tree is initialized.
    pub fn is_initialized(&self, path: mir::MovePathId, paths: &mir::MoveTable) -> bool {
        self.get(path) == Initialization::Initialized
            && paths
                .children(path)
                .iter()
                .all(|child| self.is_initialized(*child, paths))
    }

    /// Set one path's initialization.
    fn set(&mut self, path: mir::MovePathId, initialization: Initialization) {
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
        let mut moved_at = self.moved_at.clone();
        for (&path, &anchor) in &other.moved_at {
            moved_at.entry(path).or_insert(anchor);
        }

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
