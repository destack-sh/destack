use destack_core::FxIndexMap;
use std::sync::Arc;

use destack_mir as mir;

/// Planned destruction for one function.
pub(in crate::elaborate) struct DropPlan {
    /// The function receiving explicit drops.
    pub(super) function: mir::FunctionId,
    /// Independently movable ownership paths.
    pub(super) paths: Arc<mir::MoveTable>,
    /// Planned drops inside each block.
    pub(super) block_drops: FxIndexMap<mir::BlockId, Vec<BlockDrop>>,
    /// Planned edge-specific drops.
    pub(super) edge_drops: Vec<EdgeDrop>,
    /// Planned drops handed to the collector inside each block.
    pub(super) deferred_drops: FxIndexMap<mir::BlockId, Vec<DeferredDrop>>,
}

/// Destruction handed to the collector before one store overwrites storage through a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeferredDrop {
    /// Index of the overwriting store inside the block's instructions.
    pub(super) store: usize,
    /// The pointer the store writes through.
    pub(super) pointer: mir::Value,
}

/// Ownership analyses used to build one drop plan.
struct DropAnalysis<'a> {
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
    /// Planned drops handed to the collector inside each block.
    deferred_drops: FxIndexMap<mir::BlockId, Vec<DeferredDrop>>,
}

/// Destruction planned at one instruction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlockDrop {
    /// Instruction index where destruction is inserted.
    pub(super) index: usize,
    /// Maximal initialized path to destroy.
    pub(super) path: mir::MovePathId,
}

/// Destruction inserted on one control-flow edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EdgeDrop {
    /// The control-flow edge.
    pub(super) edge: mir::Edge,
    /// Maximal initialized paths to destroy.
    pub(super) paths: Vec<mir::MovePathId>,
}

impl DropPlan {
    /// Return the types the plan hands to the collector, whose allocations need destructors.
    pub(in crate::elaborate) fn deferred_types(&self, tree: &mir::Tree) -> Vec<mir::TypeId> {
        let function = tree.get(self.function);
        self.deferred_drops
            .values()
            .flatten()
            .filter_map(|drop| function.pointee_type(drop.pointer, tree))
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
        let blocks = self
            .block_drops
            .values()
            .flatten()
            .map(|drop| self.paths.get(drop.path).ty);
        let edges = self
            .edge_drops
            .iter()
            .flat_map(|drop| &drop.paths)
            .map(|path| self.paths.get(*path).ty);

        blocks.chain(edges)
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
        let liveness = analyses.liveness(function, tree);
        let places = analyses.place(function, tree);
        let paths = analyses.moves(function, tree);
        let initialization = analyses.initialization(function, tree);
        let mut analysis = Self {
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
            deferred_drops: FxIndexMap::default(),
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
            deferred_drops: analysis.deferred_drops,
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
            let mut live = liveness.block(tree, block);

            // destroy dead parameters or leave them to incoming edges
            for root in self.paths.roots().collect::<Vec<_>>() {
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
        live: &mut mir::LiveSet<'_>,
    ) {
        let block = self.tree.get(block_id);

        // walk instructions in execution order
        for (index, instruction_id) in block.instructions.iter().enumerate() {
            let instruction = self.tree.get(*instruction_id);
            self.plan_overwrite(block_id, index, instruction, state);

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
            if let mir::Instruction::LocalSet { local, .. } = instruction
                && let Some(path) = self.paths.local(*local)
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
            let roots = self.paths.roots().rev().collect::<Vec<_>>();
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

    /// Plan destruction before replacing an initialized place.
    fn plan_overwrite(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        index: usize,
        instruction: &mir::Instruction,
        state: &mut mir::InitializationState,
    ) {
        let path = match instruction {
            // local.set l0, v1
            mir::Instruction::LocalSet { local, .. } => self.paths.local(*local),
            // v2: Pair = field.set v0, 1, v1
            mir::Instruction::FieldSet {
                aggregate, field, ..
            } => {
                let projection = mir::Projection::Field { index: *field };
                let place = self.places.project(*aggregate, projection);

                self.paths.place(&place)
            }
            // v2: [File; 4] = element.set v0, 1, v1
            mir::Instruction::ElementSet {
                aggregate, index, ..
            } => {
                let projection = mir::Projection::Element { index: *index };
                let place = self.places.project(*aggregate, projection);

                self.paths.place(&place)
            }
            // store v0, v1: owned storage drops here, storage reached through a reference defers
            mir::Instruction::Store { pointer, .. } => {
                let path = self.paths.pointee(*pointer).or_else(|| {
                    let place = self.places.get(*pointer).clone();

                    self.paths.place(&place)
                });
                if path.is_none() {
                    self.plan_deferred(block, index, *pointer);
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

    /// Hand the value one store overwrites through a reference to the collector.
    fn plan_deferred(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        index: usize,
        pointer: mir::Value,
    ) {
        let Some(pointee) = self.function.pointee_type(pointer, self.tree) else {
            return;
        };

        // a store into uninitialized storage starts its lifetime, leaving no old value
        let pointee_type = self.tree.get(pointee);
        if matches!(pointee_type, mir::Type::Uninit { .. }) {
            return;
        }

        // defer the old value when the pointee owns storage
        let is_owned = self
            .drops
            .requires_destructor(pointee, mir::Storage::Frame, self.tree)
            || pointee_type.is_unique_storage();
        if !is_owned {
            return;
        }

        self.deferred_drops
            .entry(block)
            .or_default()
            .push(DeferredDrop {
                store: index,
                pointer,
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
        let paths = self.initialized_drop_paths(root, state);

        for path in paths {
            self.block_drops
                .entry(block)
                .or_default()
                .push(BlockDrop { index, path });
            state.uninitialize(path, &self.paths);
        }
    }

    /// Return maximal initialized subtrees in destruction order.
    fn initialized_drop_paths(
        &self,
        root: mir::MovePathId,
        state: &mir::InitializationState,
    ) -> Vec<mir::MovePathId> {
        // drop a completely initialized subtree as one value
        if state.is_initialized(root, &self.paths) {
            return vec![root];
        }

        // decompose partially initialized aggregates into definite children
        let children = self.paths.children(root);
        let mut initialized = Vec::new();
        for child in children.iter().rev() {
            initialized.extend(self.initialized_drop_paths(*child, state));
        }
        if !children.is_empty() {
            return initialized;
        }

        // require a definite state for every ownership leaf
        if state.get(root) == mir::Initialization::MaybeInitialized {
            unreachable!("verified MIR contains conditionally initialized ownership");
        }

        initialized
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
                let live = self.liveness.block(self.tree, target.block);
                let retained = self.retention.block(target.block);
                let mut dropped = Vec::new();

                // destroy each initialized subtree not carried into this successor
                for root in self.paths.roots().rev() {
                    for path in self.initialized_drop_paths(root, exit) {
                        let target_path =
                            self.map_target_path(edge.successor, terminator, target, path);
                        let target_root = self.paths.root(target_path);
                        let is_initialized = entry.is_initialized(target_path, &self.paths);
                        let is_live = self.is_owner_live(target_root, &live, retained);
                        if !is_initialized || !is_live {
                            dropped.push(path);
                        }
                    }
                }
                if !dropped.is_empty() {
                    self.edge_drops.push(EdgeDrop {
                        edge,
                        paths: dropped,
                    });
                }
            }
        }
    }

    /// Merge destruction shared by every outgoing edge into its source block.
    fn merge_common_edge_drops(&mut self) {
        for &predecessor in self.function.blocks() {
            let edge_count = self
                .tree
                .get(self.tree.get(predecessor).terminator)
                .targets(self.tree, predecessor)
                .len();
            if edge_count == 0 {
                continue;
            }

            let mut common = self
                .edge_drops
                .iter()
                .find(|drop| drop.edge.source == predecessor)
                .map(|edge| edge.paths.clone())
                .unwrap_or_default();
            common.retain(|path| {
                self.edge_drops
                    .iter()
                    .filter(|drop| drop.edge.source == predecessor)
                    .filter(|edge| edge.paths.contains(path))
                    .count()
                    == edge_count
            });
            if common.is_empty() {
                continue;
            }

            let index = self.tree.get(predecessor).instructions.len();
            for path in &common {
                self.block_drops
                    .entry(predecessor)
                    .or_default()
                    .push(BlockDrop { index, path: *path });
            }
            for edge in self
                .edge_drops
                .iter_mut()
                .filter(|drop| drop.edge.source == predecessor)
            {
                edge.paths.retain(|path| !common.contains(path));
            }
        }

        self.edge_drops.retain(|edge| !edge.paths.is_empty());
    }

    /// Map one source path onto its target block parameter.
    fn map_target_path(
        &self,
        successor: mir::Successor,
        terminator: &mir::Terminator,
        target: &mir::BlockTarget,
        path: mir::MovePathId,
    ) -> mir::MovePathId {
        let arguments = target.arguments(self.tree);
        let parameters = terminator
            .target_parameters(self.tree, successor, target)
            .unwrap_or_else(|| unreachable!("verified block target has invalid argument count"));

        // map ownership transferred through one block parameter
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            let Some(argument) = self.paths.value(argument) else {
                continue;
            };
            let Some(parameter) = self.paths.value(parameter.value) else {
                continue;
            };
            if path != argument && !self.paths.is_ancestor(argument, path) {
                continue;
            }

            let parameter = self.paths.map(path, argument, parameter);
            return parameter;
        }

        path
    }

    /// Return whether one owner is live or retained at the current operation.
    fn is_owner_live(
        &self,
        owner: mir::MovePathId,
        live: &mir::LiveSet<'_>,
        retained: &[mir::MovePathId],
    ) -> bool {
        let origin = self.paths.get(owner).place.origin;
        let is_live = live.find_representation(origin, &self.places).is_some();

        is_live || retained.contains(&owner)
    }
}
