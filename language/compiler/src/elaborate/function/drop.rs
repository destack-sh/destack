use tspp_core::FxIndexMap;

use tspp_mir as mir;

use crate::elaborate::drop::{
    BlockDrop, DropAction, DropEmitter, DropPlan, EdgeDrop, OverwriteDrop, UnwindDrop,
};

/// Inserter for planned MIR drops.
pub(in crate::elaborate) struct DropInserter<'a> {
    /// The MIR tree receiving explicit drops.
    tree: &'a mut mir::Tree,
    /// Canonical MIR drop table.
    drops: &'a mir::DropTable,
}

/// One planned destruction at an instruction boundary.
enum BlockPlacement {
    /// Destroy a value or release its emptied allocation.
    Drop(DropAction),
    /// Destroy the value one store overwrites through a reference.
    Overwrite(mir::Place),
}

/// Position of destruction within an existing block.
enum DropPosition {
    /// At block entry.
    Entry,
    /// Before the block terminator.
    Terminator,
}

impl<'a> DropInserter<'a> {
    /// Create one MIR drop inserter.
    pub(in crate::elaborate) fn new(tree: &'a mut mir::Tree, drops: &'a mir::DropTable) -> Self {
        Self { tree, drops }
    }

    /// Insert explicit destruction for verified move-only places.
    pub(in crate::elaborate) fn insert(&mut self, plans: Vec<DropPlan>) {
        for plan in plans {
            let DropPlan {
                function: function_id,
                paths,
                block_drops,
                edge_drops,
                overwrite_drops,
                unwind_drops,
            } = plan;
            self.insert_block_drops(function_id, &paths, block_drops, overwrite_drops);
            self.insert_edge_drops(function_id, &paths, edge_drops);

            // split each block at its last call first, keeping earlier calls in place
            for unwind in unwind_drops.into_iter().rev() {
                self.insert_unwind_drops(function_id, &paths, unwind);
            }
        }
    }

    /// Insert destruction at instruction boundaries.
    fn insert_block_drops(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        paths: &mir::MoveTable,
        mut drops: FxIndexMap<mir::LocalNodeId<mir::Block>, Vec<BlockDrop>>,
        mut overwrites: FxIndexMap<mir::LocalNodeId<mir::Block>, Vec<OverwriteDrop>>,
    ) {
        // retain physical block order while instructions are inserted
        let blocks = self.tree.get(function_id).blocks().to_vec();

        // emit block drops in physical function order
        for block_id in blocks {
            // gather the frame drops and the overwrites planned inside this block
            let mut placements = drops
                .swap_remove(&block_id)
                .unwrap_or_default()
                .into_iter()
                .map(|drop| (drop.index, BlockPlacement::Drop(drop.action)))
                .chain(
                    overwrites
                        .swap_remove(&block_id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|drop| (drop.store, BlockPlacement::Overwrite(drop.place))),
                )
                .collect::<Vec<_>>();

            if placements.is_empty() {
                continue;
            }

            let mut inserted = 0;
            placements.sort_by_key(|(index, _)| *index);

            // preserve planned order at each instruction boundary
            for (index, placement) in placements {
                let instructions = match placement {
                    BlockPlacement::Drop(action) => {
                        DropEmitter::new(self.tree, self.drops, function_id, block_id, paths)
                            .emit(action)
                    }
                    BlockPlacement::Overwrite(place) => {
                        DropEmitter::new(self.tree, self.drops, function_id, block_id, paths)
                            .emit_overwrite(place)
                    }
                };
                let count = instructions.len();
                if count == 0 {
                    continue;
                }

                let instructions = instructions
                    .into_iter()
                    .map(|instruction| self.tree.insert(instruction))
                    .collect::<Vec<_>>();
                let block = self.tree.get_mut(block_id);
                let index = index + inserted;
                block.instructions.splice(index..index, instructions);
                inserted += count;
            }
        }
    }

    /// Insert destruction on individual control-flow edges.
    fn insert_edge_drops(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        paths: &mir::MoveTable,
        edges: Vec<EdgeDrop>,
    ) {
        // collect edge placements before updating the stored function
        let mut function = self.tree.get(function_id).clone();
        let mut incoming = FxIndexMap::default();
        let mut edge_blocks = FxIndexMap::default();
        let mut placements = Vec::new();
        let mut is_changed = false;

        // count incoming edges with multiplicity
        for &block_id in function.blocks() {
            let block = self.tree.get(block_id);
            let terminator = self.tree.get(block.terminator);
            for (_, target) in terminator.targets(self.tree, block_id) {
                *incoming.entry(target.block).or_insert(0usize) += 1;
            }
        }

        // choose one insertion block for every planned edge
        for edge in edges {
            // insert at a successor with one incoming edge
            if incoming.get(&edge.edge.target) == Some(&1) {
                let source = self.tree.get(edge.edge.source);
                let terminator = self.tree.get(source.terminator);
                let targets = terminator.targets(self.tree, edge.edge.source);
                let (_, target) = targets
                    .iter()
                    .find(|(candidate, _)| *candidate == edge.edge)
                    .unwrap_or_else(|| unreachable!("planned drop edge has no target"));

                // map source owners to the successor parameters
                let mapped = edge
                    .actions
                    .into_iter()
                    .map(|action| {
                        let path = DropPlan::map_target_path(
                            paths,
                            edge.edge.successor,
                            terminator,
                            target,
                            action.path(),
                            self.tree,
                        );

                        action.at_path(path)
                    })
                    .collect();
                placements.push((edge.edge.target, DropPosition::Entry, mapped));

                continue;
            }

            // insert at a source with one outgoing edge
            let source = self.tree.get(edge.edge.source);
            let terminator = self.tree.get(source.terminator);
            if terminator.targets(self.tree, edge.edge.source).len() == 1 {
                placements.push((edge.edge.source, DropPosition::Terminator, edge.actions));

                continue;
            }

            // split one selected edge before inserting path-specific destruction
            let block =
                edge.edge
                    .split(&mut function, self.tree, &mut edge_blocks, &mut is_changed);
            placements.push((block, DropPosition::Terminator, edge.actions));
        }

        if is_changed {
            self.tree.set(function_id, function);
        }

        // emit destruction through values available in each insertion block
        for (block, position, actions) in placements {
            let mut instructions = Vec::new();
            for action in actions {
                instructions.extend(
                    DropEmitter::new(self.tree, self.drops, function_id, block, paths).emit(action),
                );
            }
            if instructions.is_empty() {
                continue;
            }

            let instructions = instructions
                .into_iter()
                .map(|instruction| self.tree.insert(instruction))
                .collect::<Vec<_>>();
            let block = self.tree.get_mut(block);
            match position {
                DropPosition::Entry => {
                    block.instructions.splice(0..0, instructions);
                }
                DropPosition::Terminator => {
                    block.instructions.extend(instructions);
                }
            }
        }
    }

    /// Turn one call into an invoke whose unwind edge destroys the values it leaves owned.
    fn insert_unwind_drops(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        paths: &mir::MoveTable,
        unwind: UnwindDrop,
    ) {
        // find the call in its block
        let block_id = unwind.block;
        let index = self
            .tree
            .get(block_id)
            .instructions
            .iter()
            .position(|id| *id == unwind.call)
            .unwrap_or_else(|| unreachable!("planned unwind call is absent from its block"));
        let mir::Instruction::Call { destination, call } = self.tree.get(unwind.call).clone()
        else {
            unreachable!("planned unwind point is a call");
        };

        // move the instructions after the call into the normal continuation
        let block = self.tree.get_mut(block_id);
        let rest = block.instructions.split_off(index + 1);
        block.instructions.pop();
        let terminator = block.terminator;
        let parameters = destination
            .map(|value| {
                let ty = self
                    .tree
                    .get(function_id)
                    .value_type(value)
                    .unwrap_or_else(|| unreachable!("a call result without its type"));

                mir::BlockParameter { value, ty }
            })
            .into_iter()
            .collect();
        let mut normal = mir::Block::with_parameters(parameters, terminator);
        normal.instructions = rest;
        let normal = self.tree.insert(normal);

        // destroy the owned values on the unwind edge, then continue unwinding
        let resume = self.tree.insert(mir::Terminator::UnwindResume);
        let cleanup = self.tree.insert(mir::Block::new(resume));
        let mut instructions = Vec::new();
        for action in unwind.actions {
            instructions.extend(
                DropEmitter::new(self.tree, self.drops, function_id, cleanup, paths).emit(action),
            );
        }
        let instructions = instructions
            .into_iter()
            .map(|instruction| self.tree.insert(instruction))
            .collect();
        self.tree.get_mut(cleanup).instructions = instructions;

        // end the call's block in the invoke, keeping the call's source
        let no_arguments = self.tree.add_values(&[]);
        let invoke = self.tree.insert(mir::Terminator::Invoke {
            call,
            target: mir::BlockTarget::new(normal, no_arguments),
            unwind: mir::BlockTarget::new(cleanup, no_arguments),
        });
        if let Some(source) = self.tree.get_source(unwind.call.id) {
            self.tree.set_source(invoke.id, source);
        }
        if let Some(span) = self.tree.get_span_by_id(unwind.call.id) {
            self.tree.set_span_by_id(invoke.id, span);
        }
        self.tree.get_mut(block_id).terminator = invoke;

        // order the continuation and the cleanup after the call's block
        let mut function = self.tree.get(function_id).clone();
        function.insert_block_after(block_id, normal, self.tree);
        function.insert_block_after(normal, cleanup, self.tree);
        self.tree.set(function_id, function);
    }
}
