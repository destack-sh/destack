use std::mem;

use destack_artifact::MirOptimized;
use destack_mir as mir;

use crate::optimize::pipeline::FunctionPass;
use crate::{CompilerError, CompilerResult};

/// Insert collector write barriers after stores into heap storage.
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
        let mut inserter =
            BarrierInserter::new(&mut module.tree, &mut module.layouts, module.target);

        inserter.insert(function)
    }
}

/// Write barrier insertion for stores of managed references.
struct BarrierInserter<'a> {
    /// The MIR tree receiving the barriers.
    tree: &'a mut mir::Tree,
    /// The layouts naming the references each stored value holds.
    layouts: &'a mut mir::LayoutTable,
    /// The target ABI for generated reference and size types.
    target: mir::TargetLayout,
}

impl<'a> BarrierInserter<'a> {
    /// Create one MIR write barrier inserter.
    fn new(
        tree: &'a mut mir::Tree,
        layouts: &'a mut mir::LayoutTable,
        target: mir::TargetLayout,
    ) -> Self {
        Self {
            tree,
            layouts,
            target,
        }
    }

    /// Follow every store of heap references into heap storage with a barrier.
    fn insert(&mut self, function: mir::FunctionId) -> CompilerResult<mir::Mutation> {
        // record changes only when a store needs a barrier
        let mut mutation = mir::Mutation::NONE;

        // visit each store in the function
        for block in self.tree.get(function).blocks().to_vec() {
            let original = mem::take(&mut self.tree.get_mut(block).instructions);
            let mut rewritten = Vec::with_capacity(original.len());

            // preserve instruction order while inserting addresses and barriers
            for instruction in original {
                // select ordinary and atomic stores of heap references
                let (place, value) = match self.tree.get(instruction) {
                    mir::Instruction::Store { place, value }
                    | mir::Instruction::AtomicStore { place, value, .. }
                    | mir::Instruction::AtomicRmw { place, value, .. }
                    | mir::Instruction::AtomicCompareExchange {
                        place,
                        new_value: value,
                        ..
                    } => (place.clone(), *value),
                    _ => {
                        rewritten.push(instruction);
                        continue;
                    }
                };
                let Some(storage) = place
                    .storage(function, self.tree)
                    .filter(|storage| storage.heap_space().is_some())
                else {
                    rewritten.push(instruction);
                    continue;
                };
                let Some(stored) = self.stored_reference_bytes(function, value)? else {
                    rewritten.push(instruction);
                    continue;
                };

                // evaluate the written address once, before the store can change its reference path
                let Some(mir::PlaceType::Value(pointee)) = place.ty(function, self.tree) else {
                    return Err(CompilerError::Internal {
                        message: "a write barrier store does not select a value".to_string(),
                    });
                };
                let reference = self.tree.intern_type(mir::Type::Reference {
                    kind: mir::Reference::Raw,
                    lifetime: mir::Lifetime::empty(),
                    storage,
                    access: mir::Access::Mutable,
                    pointee,
                }, mir::Copy::Yes);
                let usize_type = self.tree.intern_type(mir::Type::Usize, mir::Copy::Yes);
                let mut layouts = mir::LayoutBuilder::new(self.tree, self.layouts, self.target);
                for ty in [reference, usize_type] {
                    layouts
                        .layout_type(ty)
                        .map_err(|error| CompilerError::Internal {
                            message: format!("write barrier type layout failed: {error}"),
                        })?;
                }
                let pointer = self.tree.get_mut(function).next_typed_value(reference);
                let address = self.tree.insert(mir::Instruction::Address {
                    destination: pointer,
                    place,
                    result_type: reference,
                });
                let place = match self.tree.get_mut(instruction) {
                    mir::Instruction::Store { place, .. }
                    | mir::Instruction::AtomicStore { place, .. }
                    | mir::Instruction::AtomicRmw { place, .. }
                    | mir::Instruction::AtomicCompareExchange { place, .. } => place,
                    _ => unreachable!("a selected store changed instruction kind"),
                };
                *place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
                rewritten.push(address);
                rewritten.push(instruction);

                // record the written range after the store
                let barrier = self.barrier(function, pointer, stored, usize_type);
                rewritten.extend(barrier);
                mutation = mir::Mutation::VALUE.union(mir::Mutation::LAYOUT);
            }

            self.tree
                .replace_block_instructions(function, block, rewritten);
        }

        Ok(mutation)
    }

    /// Return the stored size when a value contains heap references.
    fn stored_reference_bytes(
        &self,
        function: mir::FunctionId,
        value: mir::Value,
    ) -> CompilerResult<Option<u32>> {
        let function = self.tree.get(function);

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
        usize_type: mir::TypeId,
    ) -> [mir::LocalNodeId<mir::Instruction>; 3] {
        // allocate the values describing the written range
        let width = self.target.pointer_bits();
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
