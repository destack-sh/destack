use destack_artifact::MirOptimized;
use destack_mir as mir;

use crate::optimize::pipeline::FunctionPass;
use crate::{CompilerError, CompilerResult};

/// Insert collector write barriers after stores into managed storage.
pub(in crate::optimize) struct InsertWriteBarriers;

impl FunctionPass for InsertWriteBarriers {
    /// Record managed references written by the selected function.
    fn run(
        &self,
        function: mir::FunctionId,
        module: &mut MirOptimized,
        _analyses: &mut mir::FunctionCache,
    ) -> CompilerResult<mir::Mutation> {
        // preserve generated destructor bodies
        if module.drops.is_destructor(function) {
            return Ok(mir::Mutation::NONE);
        }

        // insert barriers using the final store types and layouts
        let pointer_bits = module.target.pointer_bits();
        let mut inserter = BarrierInserter::new(&mut module.tree, &module.layouts, pointer_bits);

        inserter.insert(function)
    }
}

/// Write barrier insertion for stores of managed references.
struct BarrierInserter<'a> {
    /// The MIR tree receiving the barriers.
    tree: &'a mut mir::Tree,
    /// The layouts naming the references each stored value holds.
    layouts: &'a mir::LayoutTable,
    /// The target pointer width in bits, sizing the range constants.
    pointer_bits: u16,
}

impl<'a> BarrierInserter<'a> {
    /// Create one MIR write barrier inserter.
    fn new(tree: &'a mut mir::Tree, layouts: &'a mir::LayoutTable, pointer_bits: u16) -> Self {
        Self {
            tree,
            layouts,
            pointer_bits,
        }
    }

    /// Follow every store of a reference-holding value into managed storage with a barrier.
    fn insert(&mut self, function: mir::FunctionId) -> CompilerResult<mir::Mutation> {
        // record changes only when a store needs a barrier
        let mut mutation = mir::Mutation::NONE;

        // visit each store in the function
        for block in self.tree.get(function).blocks().to_vec() {
            let mut index = 0;

            // scan instructions and skip inserted barriers
            while index < self.tree.get(block).instructions.len() {
                // read the next instruction before inserting its barrier
                let instruction = self.tree.get(block).instructions[index];
                index += 1;

                // select stores of managed references
                let mir::Instruction::Store { pointer, value } = *self.tree.get(instruction) else {
                    continue;
                };
                let Some(stored) = self.stored_reference_bytes(function, pointer, value)? else {
                    continue;
                };

                // record the written range after the store
                let barrier = self.barrier(function, pointer, stored);
                let instructions = &mut self.tree.get_mut(block).instructions;
                instructions.splice(index..index, barrier);
                index += 3;
                mutation = mir::Mutation::VALUE.union(mir::Mutation::LAYOUT);
            }
        }

        Ok(mutation)
    }

    /// Return the stored size when a store into managed storage contains heap references.
    fn stored_reference_bytes(
        &self,
        function: mir::FunctionId,
        pointer: mir::Value,
        value: mir::Value,
    ) -> CompilerResult<Option<u32>> {
        // select pointers into managed heap storage
        let function = self.tree.get(function);
        let pointer_type = self.tree.get(function.expect_value_type(pointer));
        let is_managed = pointer_type
            .reference_storage()
            .and_then(mir::Storage::heap_space)
            .is_some();
        if !is_managed {
            return Ok(None);
        }

        // require the stored type's elaborated layout
        let ty = function.expect_value_type(value);
        let layout = self
            .layouts
            .type_layout(ty)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("write barrier store type {ty:?} has no layout"),
            })?;

        Ok(layout.trace_map.has_heap_reference().then_some(layout.size))
    }

    /// Build the constants and the barrier recording one written range.
    fn barrier(
        &mut self,
        function: mir::LocalNodeId<mir::Function>,
        pointer: mir::Value,
        byte_len: u32,
    ) -> [mir::LocalNodeId<mir::Instruction>; 3] {
        // allocate the values describing the written range
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
