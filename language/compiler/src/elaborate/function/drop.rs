use destack_core::FxIndexMap;

use destack_mir as mir;

use crate::elaborate::drop::{
    BlockDrop, DropAction, DropEmitter, DropPlan, EdgeDrop, OverwriteDrop,
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
            } = plan;
            self.insert_block_drops(function_id, &paths, block_drops, overwrite_drops);
            self.insert_edge_drops(function_id, &paths, edge_drops);
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
}
