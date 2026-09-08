use destack_core::FxIndexMap;

use destack_mir as mir;

use super::{BlockDrop, DeferredDrop, DropEmitter, DropPlan, EdgeDrop};

/// Inserter for planned MIR drops.
pub(in crate::elaborate) struct DropInserter<'a> {
    /// The MIR tree receiving explicit drops.
    tree: &'a mut mir::Tree,
    /// Canonical MIR drop table.
    drops: &'a mir::DropTable,
}

/// One planned destruction at an instruction boundary.
enum BlockPlacement {
    /// Destroy one initialized move path in the frame.
    Drop(mir::MovePathId),
    /// Hand the value behind one pointer to the collector.
    Deferred(mir::Value),
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
                deferred_drops,
            } = plan;
            self.insert_block_drops(function_id, &paths, block_drops, deferred_drops);
            self.insert_edge_drops(function_id, &paths, edge_drops);
        }
    }

    /// Insert destruction at instruction boundaries.
    fn insert_block_drops(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        paths: &mir::MoveTable,
        mut drops: FxIndexMap<mir::LocalNodeId<mir::Block>, Vec<BlockDrop>>,
        mut deferred: FxIndexMap<mir::LocalNodeId<mir::Block>, Vec<DeferredDrop>>,
    ) {
        // retain physical block order while instructions are inserted
        let blocks = self.tree.get(function_id).blocks().to_vec();

        // emit block drops in physical function order
        for block_id in blocks {
            // gather the frame drops and the deferred drops planned inside this block
            let mut placements = drops
                .swap_remove(&block_id)
                .unwrap_or_default()
                .into_iter()
                .map(|drop| (drop.index, BlockPlacement::Drop(drop.path)))
                .chain(
                    deferred
                        .swap_remove(&block_id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|drop| (drop.store, BlockPlacement::Deferred(drop.pointer))),
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
                    BlockPlacement::Drop(path) => {
                        DropEmitter::new(self.tree, self.drops, function_id, block_id, paths)
                            .emit(path)
                    }
                    BlockPlacement::Deferred(pointer) => self.emit_deferred(function_id, pointer),
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

    /// Hand the value one store overwrites to the collector, which frees the local heap at its safepoints alone.
    fn emit_deferred(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        pointer: mir::Value,
    ) -> Vec<mir::Instruction> {
        // intern the managed cell holding what the store overwrites
        let pointee = self
            .tree
            .get(function_id)
            .pointee_type(pointer, self.tree)
            .unwrap_or_else(|| unreachable!("deferred drop through a non-pointer value"));
        let cell_type = self.tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::LocalHeap,
            access: mir::Access::Mutable,
            pointee,
            nullability: mir::Nullability::None,
        });

        // reserve the loaded value and the cell receiving it
        let function = self.tree.get_mut(function_id);
        let overwritten = function.next_typed_value(pointee);
        let cell = function.next_typed_value(cell_type);

        vec![
            mir::Instruction::Load {
                destination: overwritten,
                pointer,
                result_type: pointee,
            },
            mir::Instruction::NewZeroed {
                destination: cell,
                storage_type: pointee,
                result_type: mir::TypeId::from(cell_type),
            },
            mir::Instruction::Store {
                pointer: cell,
                value: overwritten,
            },
        ]
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
                placements.push((edge.edge.target, DropPosition::Entry, edge.paths));
                continue;
            }

            // insert at a source with one outgoing edge
            let source = self.tree.get(edge.edge.source);
            let terminator = self.tree.get(source.terminator);
            if terminator.targets(self.tree, edge.edge.source).len() == 1 {
                placements.push((edge.edge.source, DropPosition::Terminator, edge.paths));
                continue;
            }

            // split one selected edge before inserting path-specific destruction
            let block =
                edge.edge
                    .split(&mut function, self.tree, &mut edge_blocks, &mut is_changed);
            placements.push((block, DropPosition::Terminator, edge.paths));
        }

        if is_changed {
            self.tree.set(function_id, function);
        }

        // emit destruction through values available in each insertion block
        for (block, position, planned_paths) in placements {
            let mut instructions = Vec::new();
            for path in planned_paths {
                instructions.extend(
                    DropEmitter::new(self.tree, self.drops, function_id, block, paths).emit(path),
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
