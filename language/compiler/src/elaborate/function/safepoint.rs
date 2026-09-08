use destack_core::FxIndexMap;
use destack_mir as mir;

/// Inserter for the holds and polls verified MIR needs at its safepoints.
pub(in crate::elaborate) struct SafepointInserter<'a> {
    /// The MIR tree receiving explicit holds and polls.
    tree: &'a mut mir::Tree,
    /// The functions whose points the safepoints name.
    functions: &'a [mir::LocalNodeId<mir::Function>],
}

impl<'a> SafepointInserter<'a> {
    /// Create one MIR safepoint inserter.
    pub(in crate::elaborate) fn new(
        tree: &'a mut mir::Tree,
        functions: &'a [mir::LocalNodeId<mir::Function>],
    ) -> Self {
        Self { tree, functions }
    }

    /// Poll at every polling safepoint, and hold the live handles past every safepoint.
    pub(in crate::elaborate) fn insert(&mut self, safepoints: &mir::SafepointTable) {
        for safepoint in safepoints.iter() {
            let live = safepoint.live.as_slice();
            match (safepoint.kind, safepoint.point) {
                // poll before the point, holding the handles past the poll
                (mir::SafepointKind::Poll, mir::Point::Instruction(instruction)) => {
                    let (_, block, index) = self.locate(instruction);
                    let poll = self.tree.insert(mir::Instruction::Poll);
                    self.tree.get_mut(block).instructions.insert(index, poll);
                    if !live.is_empty() {
                        self.hold_at(block, index + 1, live);
                    }
                }
                // poll at the block tail, holding the handles past the poll
                (mir::SafepointKind::Poll, mir::Point::Terminator(block)) => {
                    let poll = self.tree.insert(mir::Instruction::Poll);
                    let instructions = &mut self.tree.get_mut(block).instructions;
                    instructions.push(poll);
                    let index = instructions.len();
                    if !live.is_empty() {
                        self.hold_at(block, index, live);
                    }
                }
                // hold the handles past the park
                (mir::SafepointKind::Park, mir::Point::Instruction(instruction)) => {
                    let (_, block, index) = self.locate(instruction);
                    self.hold_at(block, index + 1, live);
                }
                // hold on every edge out of the parking terminator
                (mir::SafepointKind::Park, mir::Point::Terminator(block)) => {
                    let function = self.function_of(block);
                    self.hold_after_terminator(function, block, live);
                }
            }
        }
    }

    /// Hold the handles at one instruction index of a block.
    fn hold_at(&mut self, block: mir::LocalNodeId<mir::Block>, index: usize, live: &[mir::Value]) {
        let hold = self.hold(live);
        self.tree.get_mut(block).instructions.insert(index, hold);
    }

    /// Hold the handles on every edge out of one parking terminator.
    fn hold_after_terminator(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        block_id: mir::LocalNodeId<mir::Block>,
        live: &[mir::Value],
    ) {
        // stop before a tail call, which replaces the frame holding the handles
        let terminator_id = self.tree.get(block_id).terminator;
        if matches!(
            self.tree.get(terminator_id),
            mir::Terminator::TailCall { .. }
        ) {
            return;
        }

        // count incoming edges to find successors this block alone reaches
        let mut function = self.tree.get(function_id).clone();
        let mut incoming: FxIndexMap<mir::LocalNodeId<mir::Block>, usize> = FxIndexMap::default();
        for &candidate in function.blocks() {
            let terminator = self.tree.get(self.tree.get(candidate).terminator);
            for (_, target) in terminator.targets(self.tree, candidate) {
                *incoming.entry(target.block).or_insert(0) += 1;
            }
        }

        // hold at each successor's entry, splitting successors reached from elsewhere
        let edges = self
            .tree
            .get(terminator_id)
            .targets(self.tree, block_id)
            .into_iter()
            .map(|(edge, _)| edge)
            .collect::<Vec<_>>();
        let mut edge_blocks = FxIndexMap::default();
        let mut is_changed = false;
        let mut placements = Vec::with_capacity(edges.len());
        for edge in edges {
            let block = match incoming.get(&edge.target) {
                Some(1) => edge.target,
                _ => edge.split(&mut function, self.tree, &mut edge_blocks, &mut is_changed),
            };
            placements.push(block);
        }

        // publish the function once the edge splits have added their blocks
        if is_changed {
            self.tree.set(function_id, function);
        }

        // hold at the entry of each placement block
        for block in placements {
            self.hold_at(block, 0, live);
        }
    }

    /// Insert one hold instruction over the handles.
    fn hold(&mut self, live: &[mir::Value]) -> mir::LocalNodeId<mir::Instruction> {
        let values = self.tree.add_values(live);

        self.tree.insert(mir::Instruction::Hold { values })
    }

    /// Find the function, block, and index holding one instruction.
    fn locate(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> (
        mir::LocalNodeId<mir::Function>,
        mir::LocalNodeId<mir::Block>,
        usize,
    ) {
        self.functions
            .iter()
            .flat_map(|&function| {
                self.tree
                    .get(function)
                    .blocks()
                    .iter()
                    .map(move |&block| (function, block))
            })
            .find_map(|(function, block)| {
                self.tree
                    .get(block)
                    .instructions
                    .iter()
                    .position(|&candidate| candidate == instruction)
                    .map(|index| (function, block, index))
            })
            .unwrap_or_else(|| unreachable!("safepoint instruction is absent from its function"))
    }

    /// Find the function holding one block.
    fn function_of(&self, block: mir::LocalNodeId<mir::Block>) -> mir::LocalNodeId<mir::Function> {
        self.functions
            .iter()
            .copied()
            .find(|&function| self.tree.get(function).blocks().contains(&block))
            .unwrap_or_else(|| unreachable!("safepoint block is absent from its function"))
    }
}
