use destack_core::FxIndexMap;

use destack_mir as mir;
use destack_source::{ProvenanceId, ProvenanceJournal};

use super::{BlockDrop, DropEmitter, DropPlan, EdgeDrop};

/// Inserter for planned MIR drops.
pub(in crate::elaborate) struct DropInserter<'a, 'p> {
    /// The MIR tree receiving explicit drops.
    tree: &'a mut mir::Tree,
    /// The provenance transformations produced by elaboration.
    provenance: &'a mut ProvenanceJournal<'p>,
    /// Canonical MIR drop table.
    drops: &'a mir::DropTable,
}

/// Position of destruction within an existing block.
enum DropPosition {
    /// At block entry.
    Entry,
    /// Before the block terminator.
    Terminator,
}

impl<'a, 'p> DropInserter<'a, 'p> {
    /// Create one MIR drop inserter.
    pub(in crate::elaborate) fn new(
        tree: &'a mut mir::Tree,
        provenance: &'a mut ProvenanceJournal<'p>,
        drops: &'a mir::DropTable,
    ) -> Self {
        Self {
            tree,
            provenance,
            drops,
        }
    }

    /// Insert explicit destruction for verified move-only places.
    pub(in crate::elaborate) fn insert(&mut self, plans: Vec<DropPlan>) {
        for plan in plans {
            let DropPlan {
                function: function_id,
                paths,
                block_drops,
                edge_drops,
            } = plan;
            let mut function = self.tree.get(function_id).clone();
            let mut is_changed = self.insert_block_drops(&mut function, &paths, block_drops);
            is_changed |= self.insert_edge_drops(&mut function, &paths, edge_drops);

            // commit the function and its complete instruction index
            if is_changed {
                function.rebuild_instruction_index(self.tree);
                self.tree.rewrite(function_id, function, self.provenance);
            }
        }
    }

    /// Insert destruction at instruction boundaries.
    fn insert_block_drops(
        &mut self,
        function: &mut mir::Function,
        paths: &mir::MoveTable,
        mut drops: FxIndexMap<mir::LocalNodeId<mir::Block>, Vec<BlockDrop>>,
    ) -> bool {
        // retain physical block order while instructions are inserted
        let blocks = function.blocks().to_vec();
        let mut is_changed = false;

        // emit block drops in physical function order
        for block_id in blocks {
            let Some(mut drops) = drops.swap_remove(&block_id) else {
                continue;
            };
            let mut inserted = 0;
            let mut block = self.tree.get(block_id).clone();
            drops.sort_by_key(|drop| drop.index);

            // preserve planned order at each instruction index
            for drop in drops {
                let instructions =
                    DropEmitter::new(self.tree, self.drops, function, block_id, paths)
                        .emit(drop.path);
                let count = instructions.len();
                if count == 0 {
                    continue;
                }

                let source =
                    if let Some(instruction) = block.instructions.get(drop.index + inserted) {
                        self.tree.provenance(*instruction)
                    } else {
                        self.tree.provenance(block.terminator)
                    };
                let instructions = self.insert_instructions(instructions, source);
                let index = drop.index + inserted;
                block.instructions.splice(index..index, instructions);
                inserted += count;
            }

            // store the changed block as one elaboration provenance
            if inserted > 0 {
                self.tree.rewrite(block_id, block, self.provenance);
                is_changed = true;
            }
        }

        is_changed
    }

    /// Insert destruction on individual control-flow edges.
    fn insert_edge_drops(
        &mut self,
        function: &mut mir::Function,
        paths: &mir::MoveTable,
        edges: Vec<EdgeDrop>,
    ) -> bool {
        // collect edge placements before updating blocks
        let mut incoming = FxIndexMap::default();
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
            let source_terminator = self.tree.get(edge.edge.source).terminator;
            let source = self.tree.provenance(source_terminator);

            // insert at a successor with one incoming edge
            if incoming.get(&edge.edge.target) == Some(&1) {
                placements.push((edge.edge.target, DropPosition::Entry, edge.paths, source));
                continue;
            }

            // insert at a source with one outgoing edge
            let source_block = self.tree.get(edge.edge.source);
            let terminator = self.tree.get(source_block.terminator);
            if terminator.targets(self.tree, edge.edge.source).len() == 1 {
                placements.push((
                    edge.edge.source,
                    DropPosition::Terminator,
                    edge.paths,
                    source,
                ));
                continue;
            }

            // split one selected edge before inserting path-specific destruction
            let split = edge.edge.split(function, self.tree, self.provenance);
            placements.push((split.source, DropPosition::Terminator, edge.paths, source));
            is_changed = true;
        }

        // emit destruction through values available in each insertion block
        for (block_id, position, planned_paths, source) in placements {
            let mut instructions = Vec::new();
            for path in planned_paths {
                instructions.extend(
                    DropEmitter::new(self.tree, self.drops, function, block_id, paths).emit(path),
                );
            }
            if instructions.is_empty() {
                continue;
            }

            let instructions = self.insert_instructions(instructions, source);
            let mut block = self.tree.get(block_id).clone();
            match position {
                DropPosition::Entry => {
                    block.instructions.splice(0..0, instructions);
                }
                DropPosition::Terminator => {
                    block.instructions.extend(instructions);
                }
            }
            self.tree.rewrite(block_id, block, self.provenance);
            is_changed = true;
        }

        is_changed
    }

    /// Insert instructions produced from one source provenance.
    fn insert_instructions(
        &mut self,
        instructions: Vec<mir::Instruction>,
        source: ProvenanceId,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        instructions
            .into_iter()
            .map(|instruction| {
                let provenance = self.provenance.generate(&[source]);

                self.tree.insert(instruction, provenance)
            })
            .collect()
    }
}
