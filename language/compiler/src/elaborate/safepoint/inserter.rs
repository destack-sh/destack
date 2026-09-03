use destack_core::FxIndexMap;
use destack_mir as mir;

/// Inserter for the pins and polls verified MIR needs at its safepoints.
pub(in crate::elaborate) struct SafepointInserter<'a> {
    /// The MIR tree receiving explicit pins and polls.
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

    /// Pin over every parking safepoint and poll the runtime at every other one.
    pub(in crate::elaborate) fn insert(&mut self, safepoints: &mir::SafepointTable) {
        for safepoint in safepoints.iter() {
            match (safepoint.point, safepoint.pins.as_slice()) {
                // poll before a safepoint that holds nothing live
                (mir::Point::Instruction(instruction), []) => {
                    let (_, block, index) = self.locate(instruction);
                    let poll = self.tree.insert(mir::Instruction::Poll);
                    self.tree.get_mut(block).instructions.insert(index, poll);
                }
                (mir::Point::Terminator(block), []) => {
                    let poll = self.tree.insert(mir::Instruction::Poll);
                    self.tree.get_mut(block).instructions.push(poll);
                }
                // pin across a safepoint that parks with live references
                (mir::Point::Instruction(instruction), pins) => {
                    let (function, block, _) = self.locate(instruction);
                    self.pin_instruction(function, block, instruction, pins);
                }
                (mir::Point::Terminator(block), pins) => {
                    let function = self.function_of(block);
                    self.pin_terminator(function, block, pins);
                }
            }
        }
    }

    /// Bracket one parking instruction with pins and unpins.
    fn pin_instruction(
        &mut self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction: mir::LocalNodeId<mir::Instruction>,
        values: &[mir::Value],
    ) {
        let (pins, pinned) = self.pin_values(function, values);
        let unpins = self.unpin_values(&pinned);

        // wrap the instruction in place, unpinning right after it
        let instructions = &mut self.tree.get_mut(block).instructions;
        let index = instructions
            .iter()
            .position(|candidate| *candidate == instruction)
            .unwrap_or_else(|| unreachable!("pinned instruction is absent from its block"));
        instructions.splice(index + 1..index + 1, unpins);
        instructions.splice(index..index, pins);
    }

    /// Pin before one parking terminator and unpin on every edge out of it.
    fn pin_terminator(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        block_id: mir::LocalNodeId<mir::Block>,
        values: &[mir::Value],
    ) {
        let (pins, pinned) = self.pin_values(function_id, values);
        self.tree.get_mut(block_id).instructions.extend(pins);

        // stop before a tail call, which replaces the frame holding the pins
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

        // unpin at each successor's entry, splitting successors reached from elsewhere
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

        // unpin at the entry of each placement block
        for block in placements {
            let unpins = self.unpin_values(&pinned);
            self.tree.get_mut(block).instructions.splice(0..0, unpins);
        }
    }

    /// Insert one pin instruction per reference, answering the instructions and the pinned values.
    fn pin_values(
        &mut self,
        function: mir::LocalNodeId<mir::Function>,
        values: &[mir::Value],
    ) -> (Vec<mir::LocalNodeId<mir::Instruction>>, Vec<mir::Value>) {
        let mut pins = Vec::with_capacity(values.len());
        let mut pinned = Vec::with_capacity(values.len());
        for &value in values {
            let result_type = self.tree.get(function).expect_value_type(value);
            let destination = self.tree.get_mut(function).next_typed_value(result_type);
            pins.push(self.tree.insert(mir::Instruction::Pin {
                destination,
                value,
                result_type: mir::TypeId::from(result_type),
            }));
            pinned.push(destination);
        }

        (pins, pinned)
    }

    /// Insert one unpin instruction per pinned reference.
    fn unpin_values(&mut self, pinned: &[mir::Value]) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        pinned
            .iter()
            .map(|&value| self.tree.insert(mir::Instruction::Unpin { value }))
            .collect()
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
