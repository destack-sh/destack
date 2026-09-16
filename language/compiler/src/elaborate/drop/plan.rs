use destack_core::FxIndexMap;
use std::sync::Arc;

use destack_mir as mir;

/// Destruction of an initialized value or its emptied allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::elaborate) enum DropAction {
    /// Destroy the initialized value and release any allocation it owns.
    Drop(mir::MovePathId),
    /// Release an allocation after its initialized contents have been removed.
    Release(mir::MovePathId),
}

impl DropAction {
    /// Return the move path selected by this action.
    pub(in crate::elaborate) fn path(self) -> mir::MovePathId {
        match self {
            Self::Drop(path) | Self::Release(path) => path,
        }
    }

    /// Map this action to another move path.
    pub(in crate::elaborate) fn at_path(self, path: mir::MovePathId) -> Self {
        match self {
            Self::Drop(_) => Self::Drop(path),
            Self::Release(_) => Self::Release(path),
        }
    }
}

/// Planned destruction for one function.
pub(in crate::elaborate) struct DropPlan {
    /// The function receiving explicit drops.
    pub(in crate::elaborate) function: mir::FunctionId,
    /// Independently movable ownership paths.
    pub(in crate::elaborate) paths: Arc<mir::MoveTable>,
    /// Planned drops inside each block.
    pub(in crate::elaborate) block_drops: FxIndexMap<mir::BlockId, Vec<BlockDrop>>,
    /// Planned edge-specific drops.
    pub(in crate::elaborate) edge_drops: Vec<EdgeDrop>,
    /// Planned destruction of the values stores overwrite inside each block.
    pub(in crate::elaborate) overwrite_drops: FxIndexMap<mir::BlockId, Vec<OverwriteDrop>>,
}

/// Destruction of the value one store overwrites through a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::elaborate) struct OverwriteDrop {
    /// Index of the overwriting store inside the block's instructions.
    pub(in crate::elaborate) store: usize,
    /// The storage the store overwrites.
    pub(in crate::elaborate) place: mir::Place,
}

/// Ownership analyses used to build one drop plan.
struct DropAnalysis<'a> {
    /// The identity of the function being planned.
    function_id: mir::FunctionId,
    /// The function being planned.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// SSA and local liveness.
    liveness: Arc<mir::LivenessTable>,
    /// Canonical places derived by address values.
    places: Arc<mir::PlaceTable>,
    /// Independently movable ownership paths.
    paths: Arc<mir::MoveTable>,
    /// Move-path initialization across the function.
    initialization: Arc<mir::InitializationTable>,
    /// Verified ownership retention.
    retention: &'a mir::RetentionTable,
    /// Canonical MIR drop table.
    drops: &'a mir::DropTable,

    /// Planned drops inside each block.
    block_drops: FxIndexMap<mir::BlockId, Vec<BlockDrop>>,
    /// Planned edge-specific drops.
    edge_drops: Vec<EdgeDrop>,
    /// Planned destruction of the values stores overwrite inside each block.
    overwrite_drops: FxIndexMap<mir::BlockId, Vec<OverwriteDrop>>,
}

/// Destruction planned at one instruction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::elaborate) struct BlockDrop {
    /// Instruction index where destruction is inserted.
    pub(in crate::elaborate) index: usize,
    /// Destruction selected for this instruction position.
    pub(in crate::elaborate) action: DropAction,
}

/// Destruction inserted on one control-flow edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::elaborate) struct EdgeDrop {
    /// The control-flow edge.
    pub(in crate::elaborate) edge: mir::Edge,
    /// Destruction selected for this edge.
    pub(in crate::elaborate) actions: Vec<DropAction>,
}

impl DropPlan {
    /// Return the types of the values planned overwrites destroy.
    pub(in crate::elaborate) fn overwrite_types(&self, tree: &mir::Tree) -> Vec<mir::TypeId> {
        let function = self.function;
        self.overwrite_drops
            .values()
            .flatten()
            .map(|drop| match drop.place.ty(function, tree) {
                Some(mir::PlaceType::Value(ty)) => ty,
                _ => unreachable!("an overwritten place selects one value"),
            })
            .collect()
    }

    /// Build planned destruction for one function.
    pub(in crate::elaborate) fn build(
        function_id: mir::FunctionId,
        function: &mir::Function,
        tree: &mir::Tree,
        retention: &mir::RetentionTable,
        drops: &mir::DropTable,
    ) -> Self {
        DropAnalysis::build(function_id, function, tree, retention, drops)
    }

    /// Return the stored types reached by this plan.
    pub(in crate::elaborate) fn roots(&self) -> impl Iterator<Item = mir::TypeId> + '_ {
        let blocks = self.block_drops.values().flatten().map(|drop| drop.action);
        let edges = self
            .edge_drops
            .iter()
            .flat_map(|drop| drop.actions.iter().copied());

        blocks.chain(edges).filter_map(|action| match action {
            DropAction::Drop(path) => Some(self.paths.get(path).ty),
            DropAction::Release(_) => None,
        })
    }

    /// Map one source path onto its target block parameter.
    pub(in crate::elaborate) fn map_target_path(
        paths: &mir::MoveTable,
        successor: mir::Successor,
        terminator: &mir::Terminator,
        target: &mir::BlockTarget,
        path: mir::MovePathId,
        tree: &mir::Tree,
    ) -> mir::MovePathId {
        let arguments = target.arguments(tree);
        let parameters = terminator
            .target_parameters(tree, successor, target)
            .unwrap_or_else(|| unreachable!("verified block target has invalid argument count"));

        // map both the owner value and its independently tracked pointee
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            let bindings = [
                (paths.value(argument), paths.value(parameter.value)),
                (paths.pointee(argument), paths.pointee(parameter.value)),
            ];
            for (source, destination) in bindings {
                let (Some(source), Some(destination)) = (source, destination) else {
                    continue;
                };
                if path == source || paths.is_ancestor(source, path) {
                    return paths.map(path, source, destination);
                }
            }
        }

        path
    }
}

impl<'a> DropAnalysis<'a> {
    /// Build planned destruction for one function.
    fn build(
        function_id: mir::FunctionId,
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        retention: &'a mir::RetentionTable,
        drops: &'a mir::DropTable,
    ) -> DropPlan {
        // derive the ownership analyses shared by every planning step
        let mut analyses = mir::FunctionCache::new();
        let liveness = analyses.liveness(function_id, tree);
        let places = analyses.place(function_id, tree);
        let paths = analyses.moves(function_id, tree);
        let initialization = analyses.initialization(function_id, tree);
        let mut analysis = Self {
            function_id,
            function,
            tree,
            liveness,
            places,
            paths,
            initialization,
            retention,
            drops,
            block_drops: FxIndexMap::default(),
            edge_drops: Vec::new(),
            overwrite_drops: FxIndexMap::default(),
        };

        // plan block-local and edge-specific destruction
        let exits = analysis.plan_block_drops();
        analysis.plan_edge_drops(&exits);
        analysis.merge_common_edge_drops();

        DropPlan {
            function: function_id,
            paths: analysis.paths,
            block_drops: analysis.block_drops,
            edge_drops: analysis.edge_drops,
            overwrite_drops: analysis.overwrite_drops,
        }
    }

    /// Plan straight-line destruction in every reachable block.
    fn plan_block_drops(
        &mut self,
    ) -> FxIndexMap<mir::LocalNodeId<mir::Block>, mir::InitializationState> {
        // hold immutable analyses across mutable plan updates
        let liveness = self.liveness.clone();
        let tree = self.tree;
        let mut exits = FxIndexMap::default();

        for &block in self.function.blocks() {
            let Some(mut state) = self.initialization.entry(block).cloned() else {
                continue;
            };
            let mut live = liveness.cursor(tree, block);

            // destroy dead parameters or leave them to incoming edges
            for root in self.owners().collect::<Vec<_>>() {
                if !self.is_owner_live(root, &live, self.retention.block(block)) {
                    if Some(block) == self.function.entry() {
                        self.plan_drop(block, 0, root, &mut state);
                    } else {
                        state.uninitialize(root, &self.paths);
                    }
                }
            }

            self.plan_block(block, &mut state, &mut live);
            exits.insert(block, state);
        }

        exits
    }

    /// Plan destruction inside one block.
    fn plan_block(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        state: &mut mir::InitializationState,
        live: &mut mir::LivenessCursor<'_>,
    ) {
        let block = self.tree.get(block_id);

        // walk instructions in execution order
        for (index, instruction_id) in block.instructions.iter().enumerate() {
            let instruction = self.tree.get(*instruction_id);
            self.plan_assignment(block_id, index, instruction, state);

            // collect roots read or defined by this instruction
            let mut candidates = instruction
                .reads(self.tree)
                .into_iter()
                .filter_map(|value| self.paths.value(value))
                .map(|path| self.paths.root(path))
                .collect::<Vec<_>>();

            // advance ownership and liveness past the instruction
            self.initialization
                .transfer_instruction(*instruction_id, state, self.tree);
            live.advance(instruction, self.tree);

            // include newly defined ownership
            if let Some(destination) = instruction.destination()
                && let Some(path) = self.paths.value(destination)
            {
                candidates.push(self.paths.root(path));
            }
            if let mir::Instruction::Store { place, .. } = instruction
                && let Some(path) = self.paths.place(&self.places.resolve_place(place))
            {
                candidates.push(self.paths.root(path));
            }

            // visit each changed owner once
            candidates.sort_unstable();
            candidates.dedup();

            // destroy owners after their final retained use
            for root in candidates.into_iter().rev() {
                let retained = self.retention.instruction(*instruction_id);
                if self.is_owner_live(root, live, retained) {
                    continue;
                }

                self.plan_drop(block_id, index + 1, root, state);
            }
        }

        // consume values transferred out of the function
        let terminator = self.tree.get(block.terminator);
        self.initialization
            .transfer_terminator(block.terminator, state, self.tree);

        // final blocks destroy every remaining owned path
        if terminator.successors(self.tree).is_empty() {
            let used = terminator.reads(self.tree);
            let roots = self.owners().rev().collect::<Vec<_>>();
            for root in roots {
                let mir::PlaceOrigin::Value(value) = self.paths.get(root).place.origin else {
                    self.plan_drop(block_id, block.instructions.len(), root, state);
                    continue;
                };
                if !used.contains(&value) {
                    self.plan_drop(block_id, block.instructions.len(), root, state);
                }
            }
        }
    }

    /// Plan destruction of the owned value one assignment replaces.
    fn plan_assignment(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        index: usize,
        instruction: &mir::Instruction,
        state: &mut mir::InitializationState,
    ) {
        let path = match instruction {
            // v2: Pair = field.set v0, 1, v1
            mir::Instruction::FieldSet {
                aggregate, field, ..
            } => {
                let projection = mir::Projection::Field { index: *field };
                let place = mir::Place::value(*aggregate).with_projection(projection);

                self.paths.place(&place)
            }
            // v2: [File; 4] = element.set v0, 1, v1
            mir::Instruction::ElementSet {
                aggregate, index, ..
            } => {
                let projection = mir::Projection::Element { index: *index };
                let place = mir::Place::value(*aggregate).with_projection(projection);

                self.paths.place(&place)
            }
            // plan destruction at the store's selected place
            mir::Instruction::Store { place, .. } => {
                let selected = self.places.resolve_place(place);
                let path = self.paths.place(&selected);
                if path.is_none() {
                    self.plan_overwrite(block, index, place);
                }

                path
            }
            _ => None,
        };
        let Some(path) = path else {
            return;
        };

        self.plan_drop(block, index, path, state);
    }

    /// Destroy the value one store overwrites through a reference, ahead of the store.
    fn plan_overwrite(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        index: usize,
        place: &mir::Place,
    ) {
        let Some(mir::PlaceType::Value(pointee)) = place.ty(self.function_id, self.tree) else {
            unreachable!("a verified store selects one value");
        };

        // a store into uninitialized storage starts its lifetime, leaving no old value
        let pointee_type = self.tree.get(pointee);
        if matches!(pointee_type, mir::Type::Uninit { .. }) {
            return;
        }

        // destroy the old value when it owns storage
        let is_owned = self
            .drops
            .requires_destructor(pointee, mir::Storage::Frame, self.tree)
            || pointee_type.is_unique_storage();
        if !is_owned {
            return;
        }

        self.overwrite_drops
            .entry(block)
            .or_default()
            .push(OverwriteDrop {
                store: index,
                place: place.clone(),
            });
    }

    /// Plan destruction of every initialized subtree inside one path.
    fn plan_drop(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        index: usize,
        root: mir::MovePathId,
        state: &mut mir::InitializationState,
    ) {
        let actions = self.initialized_drop_actions(root, state);

        for action in actions {
            self.block_drops
                .entry(block)
                .or_default()
                .push(BlockDrop { index, action });
            state.uninitialize(action.path(), &self.paths);
        }
    }

    /// Select drops for initialized contents and releases for emptied allocations.
    fn initialized_drop_actions(
        &self,
        root: mir::MovePathId,
        state: &mir::InitializationState,
    ) -> Vec<DropAction> {
        // drop a completely initialized subtree as one value
        if state.is_initialized(root, &self.paths) {
            return vec![DropAction::Drop(root)];
        }

        // destroy the definite children of a partially initialized value
        let children = self.paths.children(root);
        let mut actions = Vec::new();
        for child in children.iter().rev() {
            actions.extend(self.initialized_drop_actions(*child, state));
        }

        // release the allocation the emptied token still owns
        match state.get(root) {
            mir::Initialization::Initialized if self.paths.pointee_of(root).is_some() => {
                actions.push(DropAction::Release(root));
            }
            mir::Initialization::MaybeInitialized if children.is_empty() => {
                unreachable!("verified MIR contains conditionally initialized ownership");
            }
            mir::Initialization::Initialized
            | mir::Initialization::MaybeInitialized
            | mir::Initialization::Uninitialized => {}
        }

        actions
    }

    /// Plan ownership discarded on individual successor edges.
    fn plan_edge_drops(
        &mut self,
        exits: &FxIndexMap<mir::LocalNodeId<mir::Block>, mir::InitializationState>,
    ) {
        for &predecessor in self.function.blocks() {
            let Some(exit) = exits.get(&predecessor) else {
                continue;
            };
            let block = self.tree.get(predecessor);
            let terminator = self.tree.get(block.terminator);

            for (edge, target) in terminator.targets(self.tree, predecessor) {
                let Some(entry) = self.initialization.entry(target.block) else {
                    continue;
                };

                // read the exact target entry state
                let live = self.liveness.cursor(self.tree, target.block);
                let retained = self.retention.block(target.block);
                let mut dropped = Vec::new();

                // destroy each initialized subtree not carried into this successor
                for root in self.owners().rev() {
                    let target_owner = DropPlan::map_target_path(
                        &self.paths,
                        edge.successor,
                        terminator,
                        target,
                        root,
                        self.tree,
                    );
                    let target_root = self.paths.root(target_owner);
                    let is_live = self.is_owner_live(target_root, &live, retained);

                    // compare each initialized path with its successor state
                    for action in self.initialized_drop_actions(root, exit) {
                        let path = action.path();
                        let target_path = DropPlan::map_target_path(
                            &self.paths,
                            edge.successor,
                            terminator,
                            target,
                            path,
                            self.tree,
                        );
                        let is_initialized = entry.is_initialized(target_path, &self.paths);
                        if !is_initialized || !is_live {
                            dropped.push(action);
                        }
                    }
                }
                if !dropped.is_empty() {
                    self.edge_drops.push(EdgeDrop {
                        edge,
                        actions: dropped,
                    });
                }
            }
        }
    }

    /// Merge destruction shared by every outgoing edge into its source block.
    fn merge_common_edge_drops(&mut self) {
        for &predecessor in self.function.blocks() {
            // read ownership transferred by the terminator
            let terminator = self.tree.get(self.tree.get(predecessor).terminator);
            let transferred = terminator.uses(self.tree);
            let edge_count = terminator.targets(self.tree, predecessor).len();
            if edge_count == 0 {
                continue;
            }

            let mut common = self
                .edge_drops
                .iter()
                .find(|drop| drop.edge.source == predecessor)
                .map(|edge| edge.actions.clone())
                .unwrap_or_default();

            // keep transferred owners alive until their successor receives them
            common.retain(|action| {
                let is_transferred = transferred.iter().any(|value| {
                    [self.paths.value(*value), self.paths.pointee(*value)]
                        .into_iter()
                        .flatten()
                        .any(|root| {
                            action.path() == root || self.paths.is_ancestor(root, action.path())
                        })
                });
                if is_transferred {
                    return false;
                }

                self.edge_drops
                    .iter()
                    .filter(|drop| drop.edge.source == predecessor)
                    .filter(|edge| edge.actions.contains(action))
                    .count()
                    == edge_count
            });
            if common.is_empty() {
                continue;
            }

            let index = self.tree.get(predecessor).instructions.len();
            for action in &common {
                self.block_drops
                    .entry(predecessor)
                    .or_default()
                    .push(BlockDrop {
                        index,
                        action: *action,
                    });
            }
            for edge in self
                .edge_drops
                .iter_mut()
                .filter(|drop| drop.edge.source == predecessor)
            {
                edge.actions.retain(|action| !common.contains(action));
            }
        }

        self.edge_drops.retain(|edge| !edge.actions.is_empty());
    }

    /// Return independently owned values and locals in move-path order.
    fn owners(&self) -> impl DoubleEndedIterator<Item = mir::MovePathId> + '_ {
        self.paths
            .roots()
            .filter(|path| self.paths.get(*path).place.path.is_root())
    }

    /// Return whether one owner is live or retained at the current operation.
    fn is_owner_live(
        &self,
        owner: mir::MovePathId,
        live: &mir::LivenessCursor<'_>,
        retained: &[mir::MovePathId],
    ) -> bool {
        let is_live = match self.paths.get(owner).place.origin {
            mir::PlaceOrigin::Value(value) => {
                live.contains_value(value)
                    || live
                        .find_representation(mir::PlaceOrigin::Value(value), &self.places)
                        .is_some()
            }
            mir::PlaceOrigin::Local(local) => live.contains_local(local),
            mir::PlaceOrigin::Global(_) => unreachable!("a planned drop of global storage"),
        };

        is_live || retained.contains(&owner)
    }
}
