use destack_mir as mir;

/// Inserter for the polls verified MIR needs at its safepoints.
pub(in crate::elaborate) struct SafepointInserter<'a> {
    /// The MIR tree receiving explicit polls.
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

    /// Poll before every safepoint.
    pub(in crate::elaborate) fn insert(&mut self, safepoints: &mir::SafepointTable) {
        for safepoint in safepoints.iter() {
            let poll = self.tree.insert(mir::Instruction::Poll);
            match safepoint.point {
                // poll before the instruction
                mir::Point::Instruction(instruction) => {
                    let (block, index) = self.locate(instruction);
                    self.tree.get_mut(block).instructions.insert(index, poll);
                }
                // poll at the block tail
                mir::Point::Terminator(block) => {
                    self.tree.get_mut(block).instructions.push(poll);
                }
            }
        }
    }

    /// Find the block and index holding one instruction.
    fn locate(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> (mir::LocalNodeId<mir::Block>, usize) {
        self.functions
            .iter()
            .flat_map(|&function| self.tree.get(function).blocks().iter().copied())
            .find_map(|block| {
                self.tree
                    .get(block)
                    .instructions
                    .iter()
                    .position(|&candidate| candidate == instruction)
                    .map(|index| (block, index))
            })
            .unwrap_or_else(|| unreachable!("safepoint instruction is absent from its function"))
    }
}
