use std::sync::Arc;

use destack_core::BitSet;
use smallvec::SmallVec;

use crate::{
    Analysis, Block, BlockTarget, Callee, CastOperator, ControlTable, DataflowTable, Edge,
    ForwardTransfer, Function, FunctionId, FunctionKind, Instruction, Intrinsic, Lattice,
    LocalNodeId, LocalNodeIdAny, MovePathId, MoveTable, Mutation, Place, PlaceTable, PlaceType,
    Projection, Substitution, Terminator, Tree, Type, TypeId, Value,
};

/// Move-path initialization across one MIR function.
#[derive(Debug)]
pub struct InitializationTable {
    /// The analysed function.
    function: FunctionId,
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
        function: FunctionId,
        control: &ControlTable,
        paths: Arc<MoveTable>,
        places: Arc<PlaceTable>,
        tree: &Tree,
    ) -> Self {
        let table = Self {
            function,
            paths,
            places,
            flow: DataflowTable::new(),
        };
        let entry = table.parameter_state(tree.get(function));
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
            Instruction::FieldGet {
                aggregate, field, ..
            } => {
                let projection = Projection::Field { index: *field };
                self.collect_projection(*aggregate, projection, state, &mut unavailable, tree);
            }
            Instruction::Load { place, .. } | Instruction::AtomicLoad { place, .. } => {
                self.collect_address(place, state, &mut unavailable);
                self.collect_place(place, state, &mut unavailable, tree);
            }
            // address uninitialized storage freely, rejecting an address of moved storage
            Instruction::Address { place, .. } => {
                self.collect_address(place, state, &mut unavailable);
                if let Some(path) = self.unavailable_place(place, state, tree)
                    && state.moved_at(path).is_some()
                {
                    Self::report(&mut unavailable, state.unavailability(path));
                }
            }
            Instruction::Store { place, value } | Instruction::AtomicStore { place, value, .. } => {
                self.collect_address(place, state, &mut unavailable);
                self.collect_value(*value, state, &mut unavailable);
            }
            // require only the reference token when reinterpreting its referent's initialization
            _ if let Some((argument, _)) = self.reinterpretation(instruction, tree) => {
                self.collect_handle(argument, state, &mut unavailable);
            }
            Instruction::ElementGet {
                aggregate, index, ..
            } => {
                let projection = Projection::Element { index: *index };
                self.collect_projection(*aggregate, projection, state, &mut unavailable, tree);
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
            _ => {
                for value in instruction.reads(tree) {
                    self.collect_value(value, state, &mut unavailable);
                }
            }
        }

        unavailable
    }

    /// Record one unavailable use once.
    fn report(unavailable: &mut SmallVec<[Unavailability; 4]>, entry: Unavailability) {
        if !unavailable.contains(&entry) {
            unavailable.push(entry);
        }
    }

    /// Return unavailable paths read or passed on by one terminator.
    pub fn terminator_unavailability(
        &self,
        source: LocalNodeId<Block>,
        terminator: &Terminator,
        state: &InitializationState,
        tree: &Tree,
    ) -> SmallVec<[Unavailability; 4]> {
        let mut unavailable = SmallVec::new();

        // require every value read by the terminator and every value it passes to a successor
        let arguments = terminator
            .targets(tree, source)
            .into_iter()
            .flat_map(|(_, target)| target.arguments(tree).iter().copied());
        for value in terminator.reads(tree).into_iter().chain(arguments) {
            self.collect_value(value, state, &mut unavailable);
        }

        unavailable
    }

    /// Project a moved value or the storage reached through a copyable address.
    fn project(&self, value: Value, projection: Projection) -> Place {
        match self.paths.value(value) {
            Some(root) => self
                .paths
                .get(root)
                .place
                .clone()
                .with_projection(projection),
            None => self.places.project(value, projection),
        }
    }

    /// Return the reference one instruction reinterprets across initialization.
    fn reinterpretation(&self, instruction: &Instruction, tree: &Tree) -> Option<(Value, bool)> {
        // select the reference each reinterpreting instruction reads and the one it defines
        let (argument, destination) = match instruction {
            Instruction::Cast {
                destination,
                operator: CastOperator::Bitcast,
                argument,
                ..
            } => (*argument, *destination),
            Instruction::Intrinsic {
                destination: Some(destination),
                intrinsic: Intrinsic::Transmute,
                arguments,
            } => match tree.get_values(*arguments) {
                [argument] => (*argument, *destination),
                _ => return None,
            },
            _ => return None,
        };

        // read the referent type on each side and strip one Uninit layer off it
        let function = tree.get(self.function);
        let pointee = |value: Value| {
            let ty = tree.storage_type(function.expect_value_type(value));
            match tree.type_definition(ty) {
                Type::Reference { pointee, .. } => Some(Substitution::resolve(*pointee, tree)),
                _ => None,
            }
        };
        let (source, target) = (pointee(argument)?, pointee(destination)?);
        let unwrapped = |ty: TypeId| match tree.get(ty) {
            Type::Uninit { value } => Some(Substitution::resolve(*value, tree)),
            _ => None,
        };

        // assert initialization when the source drops Uninit
        if unwrapped(source) == Some(target) {
            Some((argument, true))
        }
        // withdraw it when the target adds Uninit
        else if unwrapped(target) == Some(source) {
            Some((argument, false))
        }
        // leave every other conversion alone
        else {
            None
        }
    }

    /// Require the references and indices used to address a place.
    fn collect_address(
        &self,
        place: &Place,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        for value in place.uses() {
            self.collect_handle(value, state, unavailable);
        }

        // require each stored reference token before following it
        let mut prefix = Place::new(place.origin);
        for projection in &place.path.projections {
            if *projection == Projection::Deref {
                let reference = self.places.resolve_place(&prefix);
                if let Some(path) = self.paths.containing(&reference)
                    && let Some(path) = state.unavailable_shallow(path, &self.paths)
                {
                    Self::report(unavailable, state.unavailability(path));
                }
            }
            prefix.push(projection.clone());
        }
    }

    /// Collect one unavailable projected place.
    fn collect_projection(
        &self,
        base: Value,
        projection: Projection,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
        tree: &Tree,
    ) {
        let place = self.project(base, projection);
        self.collect_place(&place, state, unavailable, tree);
    }

    /// Collect one unavailable place.
    fn collect_place(
        &self,
        place: &Place,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
        tree: &Tree,
    ) {
        if let Some(path) = self.unavailable_place(place, state, tree) {
            Self::report(unavailable, state.unavailability(path));
        }
    }

    /// Return the path one use of a place finds unavailable.
    ///
    /// A use through a borrow sees the borrowed referent alone.
    fn unavailable_place(
        &self,
        place: &Place,
        state: &InitializationState,
        tree: &Tree,
    ) -> Option<MovePathId> {
        let resolved = self.places.resolve_place(place);
        let path = self.paths.containing(&resolved)?;

        // bound the use at the referent of the last borrow it reads through
        let root = match self.borrowed_referent(place, tree) {
            None => None,
            Some(None) => return None,
            Some(Some(referent)) => {
                if referent != path && !self.paths.is_ancestor(referent, path) {
                    return None;
                }

                Some(referent)
            }
        };

        // leave a move behind a borrow to the borrow checker
        let unavailable = state.unavailable_under(path, root, &self.paths)?;
        if root.is_some() && state.moved_at(unavailable).is_some() {
            return None;
        }

        Some(unavailable)
    }

    /// Return the tracked referent of the last borrowed reference one place reads through.
    fn borrowed_referent(&self, place: &Place, tree: &Tree) -> Option<Option<MovePathId>> {
        let mut referent = None;
        let mut prefix = Place::new(place.origin);

        // keep the referent of each dereference through a shared reference
        for projection in &place.path.projections {
            if *projection == Projection::Deref
                && let Some(PlaceType::Value(reference)) = prefix.ty(self.function, tree)
                && !tree.get(tree.storage_type(reference)).is_unique_storage()
            {
                let resolved = self
                    .places
                    .resolve_place(&prefix.clone().with_projection(Projection::Deref));
                referent = Some(self.paths.place(&resolved));
            }
            prefix.push(projection.clone());
        }

        referent
    }

    /// Collect one unavailable value.
    fn collect_value(
        &self,
        value: Value,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        if let Some(path) = self.paths.value(value)
            && let Some(path) = state.unavailable(path, &self.paths)
        {
            Self::report(unavailable, state.unavailability(path));
        }
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
        let place = self.project(aggregate, projection);
        let Some(replacement) = self.paths.place(&place) else {
            unreachable!("aggregate replacement has no move path");
        };
        let Some(path) = state.unavailable_replacement(parent, replacement, &self.paths) else {
            return;
        };

        Self::report(unavailable, state.unavailability(path));
    }

    /// Require an initialized reference value without reading its pointee.
    fn collect_handle(
        &self,
        value: Value,
        state: &InitializationState,
        unavailable: &mut SmallVec<[Unavailability; 4]>,
    ) {
        if let Some(path) = self.paths.value(value)
            && let Some(path) = state.unavailable_shallow(path, &self.paths)
        {
            Self::report(unavailable, state.unavailability(path));
        }
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

        // uninitialize storage consumed by reads of move-only values
        match instruction {
            Instruction::FieldGet {
                aggregate, field, ..
            } => {
                let projection = Projection::Field { index: *field };
                self.uninitialize_projection(*aggregate, projection, anchor, state, tree);
            }
            Instruction::ElementGet {
                aggregate, index, ..
            } => {
                let projection = Projection::Element { index: *index };
                self.uninitialize_projection(*aggregate, projection, anchor, state, tree);
            }
            Instruction::VariantPayload { variant, .. } => {
                self.uninitialize_value(*variant, anchor, state);
            }
            // move the owned storage a load reads
            Instruction::Load { place, .. } => {
                let place = self.places.resolve_place(place);
                if let Some(path) = self.paths.place(&place) {
                    state.move_path(path, anchor, &self.paths);
                }
            }
            _ => {}
        }

        // consume values transferred into instruction-owned storage
        for value in instruction.consumes(tree) {
            self.uninitialize_value(value, anchor, state);
        }

        // initialize the stored place and the defined value, each with everything it owns
        if let Instruction::Store { place, .. } = instruction {
            let place = self.places.resolve_place(place);
            if let Some(path) = self.paths.place(&place) {
                state.initialize(path, &self.paths);
            }
        }
        if let Some(destination) = instruction.destination()
            && let Some(path) = self.paths.value(destination)
        {
            state.initialize(path, &self.paths);
        }

        // initialize the referent a reference conversion asserts initialized
        if let Some((_, true)) = self.reinterpretation(instruction, tree)
            && let Some(destination) = instruction.destination()
        {
            self.initialize_referent(destination, state);
        }

        // a constructor call initializes the storage its receiver names
        if let Instruction::Call { call, .. } = instruction
            && let Callee::Direct { function, .. } = &call.callee
            && tree.get(*function).kind == FunctionKind::Constructor
            && let Some(receiver) = tree.get_values(call.arguments).first()
        {
            self.initialize_referent(*receiver, state);
        }
    }

    /// Initialize the storage one reference names.
    fn initialize_referent(&self, reference: Value, state: &mut InitializationState) {
        let referent = Place::value(reference).with_projection(Projection::Deref);
        let referent = self.places.resolve_place(&referent);
        if let Some(path) = self.paths.place(&referent) {
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

        // initialize move-only parameters with everything they own
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
        let place = self.project(aggregate, projection);
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
        self.unavailable_under(path, None, paths)
    }

    /// Return an unavailable path required by one complete use within one containing path.
    pub fn unavailable_under(
        &self,
        path: MovePathId,
        root: Option<MovePathId>,
        paths: &MoveTable,
    ) -> Option<MovePathId> {
        let mut current = Some(path);

        // require the path and every containing path up to the root
        while let Some(path) = current {
            if self.get(path) != Initialization::Initialized {
                return Some(path);
            }

            current = match root {
                Some(root) if root == path => None,
                _ => paths.get(path).parent,
            };
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
        // require the untracked elements of a sparsely represented aggregate
        if !paths.get(parent).is_exhaustive && self.get(parent) != Initialization::Initialized {
            return Some(parent);
        }

        // check the containing storage paths
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
            if !is_initialized || !paths.get(current).is_exhaustive {
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

    /// Mark one path and every child moved, a copy staying as it is.
    fn move_path(&mut self, path: MovePathId, moved_at: LocalNodeIdAny, paths: &MoveTable) {
        if paths.get(path).is_copy {
            return;
        }

        for path in paths.descendants(path) {
            self.set(path, Initialization::Uninitialized);
            self.moved_at[path.index()] = Some(moved_at);
        }
    }

    /// Move edge arguments into successor parameters.
    ///
    /// Each parameter starts initialized with everything it owns.
    fn bind(
        &mut self,
        bindings: &[(MovePathId, MovePathId)],
        moved_at: LocalNodeIdAny,
        paths: &MoveTable,
    ) {
        // consume every argument before initializing any parameter
        for &(argument, _) in bindings {
            self.move_path(argument, moved_at, paths);
        }

        // initialize every parameter with everything it owns
        for &(_, parameter) in bindings {
            self.initialize(parameter, paths);
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

        // update the bits for the selected initialization state
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

        // merge possible initialization from either incoming state
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
    use super::Unavailability;
    use crate::Initialization::{Initialized, MaybeInitialized, Uninitialized};
    use crate::analyses::tests::TestModule;
    use crate::{Place, Projection, Value};

    /// Merge local initialization from every reachable branch.
    #[test]
    fn test_require_initialization_on_every_incoming_branch() {
        let program = TestModule::new(
            r#"
function partial(v0: boolean, v1: ref<int32, unique, mutable, local>): void {
    local l0: ref<int32, unique, mutable, local>

entry(v0: boolean, v1: ref<int32, unique, mutable, local>):
    branch v0 => left | right

left:
    store l0, v1
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
    store l0, v1
    jump join

right:
    store l0, v1
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
            let function = program.function_id_by_name(name);
            let mut analyses = program.function_analyses();
            let table = analyses.initialization(program.function_id_by_name(name), &program.tree);
            let local = table
                .paths
                .local(program.tree.get(function).locals()[0])
                .unwrap();
            let actual = program
                .tree
                .get(function)
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
    fn test_preserve_moved_storage_across_backedges() {
        let program = TestModule::new(
            r#"
function test(v0: ref<int32, unique, mutable, local>, v1: boolean): void {
    local l0: ref<int32, unique, mutable, local>

entry(v0: ref<int32, unique, mutable, local>, v1: boolean):
    store l0, v0
    jump header

header:
    v2: ref<int32, unique, mutable, local> = load l0
    branch v1 => header | done

done:
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let local = table
            .paths
            .local(program.tree.get(function).locals()[0])
            .unwrap();
        let moved = program
            .tree
            .get(program.tree.get(function).block(1))
            .instructions[0]
            .into_any();
        let actual = program
            .tree
            .get(function)
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
    store l0, v0
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

        let function = id;
        let mut analyses = program.function_analyses();
        let table = analyses.initialization(id, &program.tree);
        let local = table
            .paths
            .local(program.tree.get(function).locals()[0])
            .unwrap();
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
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let values = [Value(1), Value(2)].map(|value| paths.value(value).unwrap());
        let actual = program
            .tree
            .get(function)
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
    fn test_initialize_results_only_on_success_edges() {
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
            let function = program.function_id_by_name(name);
            let mut analyses = program.function_analyses();
            let paths = analyses.moves(program.function_id_by_name(name), &program.tree);
            let table = analyses.initialization(program.function_id_by_name(name), &program.tree);
            let result = paths.value(Value(0)).unwrap();
            let actual = program
                .tree
                .get(function)
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

    /// Keep an initialization token consumed after producing its unique reference.
    #[test]
    fn test_consume_unique_initialization_token() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: uninit<ref<int32, unique, mutable, local>> = new.uninit int32
    v1: ref<int32, unique, mutable, local> = new.complete v0
    jump done

done:
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let initialization = analyses.initialization(program.entry_function_id(), &program.tree);
        let state = initialization
            .entry(program.tree.get(function).block(1))
            .unwrap();
        let actual = [Value(0), Value(1)].map(|value| state.get(paths.value(value).unwrap()));

        assert_eq!(actual, [Uninitialized, Initialized]);
    }

    /// Report a partially moved block argument at its jump and initialize the parameter wholly.
    #[test]
    fn test_report_a_partially_moved_block_argument() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>): void {
entry(v0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>):
    v1: ref<int32, unique, mutable, local> = load (*v0)
    jump done(v0)

done(v2: ref<ref<int32, unique, mutable, local>, unique, mutable, local>):
    v3: ref<int32, unique, mutable, local> = load (*v2)
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let entry = program.tree.get(program.tree.get(function).block(0));
        let mut state = table
            .entry(program.tree.get(function).block(0))
            .unwrap()
            .clone();
        assert_eq!(state.get(paths.value(Value(0)).unwrap()), Initialized);
        assert_eq!(state.get(paths.pointee(Value(0)).unwrap()), Initialized);
        for &instruction in &entry.instructions {
            table.transfer_instruction(instruction, &mut state, &program.tree);
        }
        let unavailable = table.terminator_unavailability(
            program.tree.get(function).block(0),
            program.tree.get(entry.terminator),
            &state,
            &program.tree,
        );
        assert_eq!(
            unavailable.as_slice(),
            &[Unavailability {
                initialization: Uninitialized,
                moved_at: Some(entry.instructions[0].into_any()),
            }]
        );

        let block = program.tree.get(program.tree.get(function).block(1));
        let state = table.entry(program.tree.get(function).block(1)).unwrap();
        let owner = paths.value(Value(2)).unwrap();
        let pointee = paths.pointee(Value(2)).unwrap();
        assert_eq!(
            [
                state.get(paths.value(Value(0)).unwrap()),
                state.get(owner),
                state.get(pointee)
            ],
            [Uninitialized, Initialized, Initialized],
        );
        let unavailable = table.instruction_unavailability(
            program.tree.get(block.instructions[0]),
            state,
            &program.tree,
        );
        assert_eq!(unavailable.as_slice(), &[]);
    }

    /// Initialize borrowed storage only after its explicit reference conversion.
    #[test]
    fn test_initialize_pointees_after_explicit_assume_init() {
        let program = TestModule::new(
            r#"
external function inspect<'a>(ref<uninit<ref<int32, unique, mutable, local>>, borrowed, 'a, mutable, frame>): void

function test(): void {
    local l0: ref<int32, unique, mutable, local>

entry:
    v0: ref<ref<int32, unique, mutable, local>, borrowed, 'frame, mutable, frame> = address l0
    v1: ref<uninit<ref<int32, unique, mutable, local>>, borrowed, 'frame, mutable, frame> = cast.bit v0 -> ref<uninit<ref<int32, unique, mutable, local>>, borrowed, 'frame, mutable, frame>
    v4: ref<uninit<ref<int32, unique, mutable, local>>, borrowed, 'frame, mutable, frame> = copy v1
    call inspect(v4): <'a>(ref<uninit<ref<int32, unique, mutable, local>>, borrowed, 'a, mutable, frame>) => void
    jump pending

pending:
    v2: ref<ref<int32, unique, mutable, local>, borrowed, 'frame, mutable, frame> = intrinsic.memory.raw.transmute(v1)
    jump complete

complete:
    v3: ref<int32, unique, mutable, local> = load l0
    release v3
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let local = paths.local(program.tree.get(function).locals()[0]).unwrap();
        let actual = program
            .tree
            .get(function)
            .blocks()
            .iter()
            .map(|&block| {
                (
                    table.entry(block).unwrap().get(local),
                    table.exit(block).unwrap().get(local),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                (Uninitialized, Uninitialized),
                (Uninitialized, Initialized),
                (Initialized, Uninitialized),
            ]
        );
        for &block in program.tree.get(function).blocks() {
            let mut state = table.entry(block).unwrap().clone();
            for &instruction in &program.tree.get(block).instructions {
                assert_eq!(
                    table
                        .instruction_unavailability(
                            program.tree.get(instruction),
                            &state,
                            &program.tree
                        )
                        .as_slice(),
                    &[],
                    "{:?}",
                    program.tree.get(instruction),
                );
                table.transfer_instruction(instruction, &mut state, &program.tree);
            }
        }
    }

    /// Track moved pointees after different owners merge through a local.
    #[test]
    fn test_move_pointee_from_merged_local() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: ref<ref<int32, unique, mutable, local>, unique, mutable, local>, v2: ref<ref<int32, unique, mutable, local>, unique, mutable, local>): void {
    local l0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>

entry(v0: boolean, v1: ref<ref<int32, unique, mutable, local>, unique, mutable, local>, v2: ref<ref<int32, unique, mutable, local>, unique, mutable, local>):
    branch v0 => left | right

left:
    store l0, v1
    jump join

right:
    store l0, v2
    jump join

join:
    v3: ref<ref<int32, unique, mutable, local>, unique, mutable, local> = load l0
    v4: ref<int32, unique, mutable, local> = load (*v3)
    v5: ref<int32, unique, mutable, local> = load (*v3)
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let block = program.tree.get(function).block(3);
        let instructions = &program.tree.get(block).instructions;
        let mut state = table.entry(block).unwrap().clone();
        let pointee = paths.pointee(Value(3)).unwrap();
        assert_eq!(state.get(pointee), Uninitialized);
        table.transfer_instruction(instructions[0], &mut state, &program.tree);
        assert_eq!(state.get(pointee), Initialized);

        for &instruction in &instructions[1..2] {
            assert_eq!(
                table
                    .instruction_unavailability(
                        program.tree.get(instruction),
                        &state,
                        &program.tree
                    )
                    .as_slice(),
                &[]
            );
            table.transfer_instruction(instruction, &mut state, &program.tree);
        }
        assert_eq!(state.get(pointee), Uninitialized);
        assert_eq!(
            table
                .instruction_unavailability(
                    program.tree.get(instructions[2]),
                    &state,
                    &program.tree
                )
                .as_slice(),
            &[Unavailability {
                initialization: Uninitialized,
                moved_at: Some(instructions[1].into_any())
            }],
        );
    }

    /// Reinitialize an owned pointee after moving its previous value.
    #[test]
    fn test_replace_moved_pointee() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>, v1: ref<int32, unique, mutable, local>): void {
entry(v0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>, v1: ref<int32, unique, mutable, local>):
    v2: ref<int32, unique, mutable, local> = load (*v0)
    store (*v0), v1
    v3: ref<int32, unique, mutable, local> = load (*v0)
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let entry = program.tree.get(function).block(0);
        let pointee = paths.pointee(Value(0)).unwrap();
        let mut state = table.entry(entry).unwrap().clone();
        let mut actual = vec![state.get(pointee)];

        for &instruction in &program.tree.get(entry).instructions {
            assert_eq!(
                table
                    .instruction_unavailability(
                        program.tree.get(instruction),
                        &state,
                        &program.tree
                    )
                    .as_slice(),
                &[]
            );
            table.transfer_instruction(instruction, &mut state, &program.tree);
            actual.push(state.get(pointee));
        }
        assert_eq!(
            actual,
            [Initialized, Uninitialized, Initialized, Uninitialized]
        );
    }

    /// Keep an untouched field initialized after moving its sibling out of an aggregate.
    #[test]
    fn test_move_one_field_without_consuming_its_sibling() {
        let program = TestModule::new(
            r#"
function test(v0: (ref<int32, unique, mutable, local>, ref<int32, unique, mutable, local>)): void {
entry(v0: (ref<int32, unique, mutable, local>, ref<int32, unique, mutable, local>)):
    v1: ref<int32, unique, mutable, local> = field.get v0, 0
    jump done

done:
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let table = analyses.initialization(program.entry_function_id(), &program.tree);
        let state = table.entry(program.tree.get(function).block(1)).unwrap();
        let aggregate = table.paths.value(Value(0)).unwrap();
        let children = table.paths.children(aggregate);
        let actual = children
            .iter()
            .map(|&path| (table.paths.get(path).place.clone(), state.get(path)))
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                (
                    Place::value(Value(0)).with_projection(Projection::Field { index: 0 }),
                    Uninitialized
                ),
                (
                    Place::value(Value(0)).with_projection(Projection::Field { index: 1 }),
                    Initialized
                ),
            ]
        );
        assert_eq!(
            state.unavailable(aggregate, &table.paths),
            Some(children[0])
        );
        assert_eq!(state.unavailable(children[1], &table.paths), None);
        assert_eq!(state.get(table.paths.value(Value(1)).unwrap()), Initialized);
    }
}
