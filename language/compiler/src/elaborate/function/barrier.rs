use destack_mir as mir;

/// Inserter for the write barriers stores of references into managed storage need.
pub(in crate::elaborate) struct BarrierInserter<'a> {
    /// The MIR tree receiving the barriers.
    tree: &'a mut mir::Tree,
    /// The layouts naming the references each stored value holds.
    layouts: &'a mir::LayoutTable,
    /// The target pointer width in bits, sizing the range constants.
    pointer_bits: u16,
}

impl<'a> BarrierInserter<'a> {
    /// Create one MIR write barrier inserter.
    pub(in crate::elaborate) fn new(
        tree: &'a mut mir::Tree,
        layouts: &'a mir::LayoutTable,
        pointer_bits: u16,
    ) -> Self {
        Self {
            tree,
            layouts,
            pointer_bits,
        }
    }

    /// Follow every store of a reference-holding value into managed storage with a barrier.
    pub(in crate::elaborate) fn insert(&mut self, functions: &[mir::LocalNodeId<mir::Function>]) {
        for &function in functions {
            for block in self.tree.get(function).blocks().to_vec() {
                let mut index = 0;
                while index < self.tree.get(block).instructions.len() {
                    let instruction = self.tree.get(block).instructions[index];
                    index += 1;
                    let mir::Instruction::Store { pointer, value } = *self.tree.get(instruction)
                    else {
                        continue;
                    };
                    let Some(stored) = self.stored_reference_bytes(function, pointer, value) else {
                        continue;
                    };

                    // record the written range after the store
                    let barrier = self.barrier(function, pointer, stored);
                    let instructions = &mut self.tree.get_mut(block).instructions;
                    instructions.splice(index..index, barrier);
                    index += 3;
                }
            }
        }
    }

    /// Return the byte length one store writes into managed storage when the value holds
    /// references.
    fn stored_reference_bytes(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        pointer: mir::Value,
        value: mir::Value,
    ) -> Option<u32> {
        let function = self.tree.get(function);
        let pointer_type = self.tree.get(function.expect_value_type(pointer));
        pointer_type.reference_storage()?.heap_space()?;
        let layout = self
            .layouts
            .type_layout(function.expect_value_type(value))?;

        layout.trace_map.has_heap_reference().then_some(layout.size)
    }

    /// Build the constants and the barrier recording one written range.
    fn barrier(
        &mut self,
        function: mir::LocalNodeId<mir::Function>,
        pointer: mir::Value,
        byte_len: u32,
    ) -> [mir::LocalNodeId<mir::Instruction>; 3] {
        let usize_type = self.tree.intern_type(mir::Type::Usize);
        let width = self.pointer_bits;
        let offset = self.tree.get_mut(function).next_typed_value(usize_type);
        let length = self.tree.get_mut(function).next_typed_value(usize_type);
        [
            self.tree.insert(mir::Instruction::Const {
                destination: offset,
                value: mir::Constant::UInt { value: 0, width },
            }),
            self.tree.insert(mir::Instruction::Const {
                destination: length,
                value: mir::Constant::UInt {
                    value: byte_len as u128,
                    width,
                },
            }),
            self.tree.insert(mir::Instruction::BarrierWrite {
                object: pointer,
                offset,
                byte_len: length,
            }),
        ]
    }
}
