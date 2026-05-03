use destack_heap::HeapOptions;
use destack_mir as mir;

use crate::program::{
    AllocationLayout, Instruction, Layout, New, NewSlice, Opcode, PointerClass, RawAlloc, RawFree,
    StackAlloc, pointer_class_from_reference,
};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::{pointer_class_for_value, reference_meta_for_value};

/// Return the heap class for one slice backing allocation.
fn slice_backing_pointer_class(
    tree: &mir::Tree,
    result_type: mir::LocalNodeId<mir::Type>,
) -> Result<PointerClass> {
    let mir::Type::Slice {
        kind,
        address_space,
        ..
    } = tree.get(result_type)
    else {
        return Err(Error::TypeMismatch {
            expected: "slice result type".to_string(),
            actual: format!("{result_type:?}"),
        });
    };

    let pointer_class = pointer_class_from_reference(address_space.clone(), *kind);
    match pointer_class {
        PointerClass::Heap | PointerClass::SharedHeap => Ok(pointer_class),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Build one allocation layout for a concrete MIR type.
fn allocation_layout(
    pool: &mut Pool<'_>,
    pointer_class: PointerClass,
    layout: &Layout,
    heap_options: &HeapOptions,
    shared_heap_options: &HeapOptions,
) -> Result<AllocationLayout> {
    // resolve the allocation class from the destination space
    let is_noscan = !layout.reference_map.has_reference();
    let has_shared_reference = layout.reference_map.has_shared_reference();
    let class = match pointer_class {
        PointerClass::Heap => {
            heap_options.allocation_class(layout.byte_len, layout.alignment(), is_noscan)
        }
        PointerClass::SharedHeap => {
            shared_heap_options.allocation_class(layout.byte_len, layout.alignment(), is_noscan)
        }
        _ => {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer_class:?}"),
            });
        }
    };

    // intern layout side channels once during lowering
    let reference_map = pool.reference_map(layout.reference_map.clone());
    let class = pool.allocation_class(class);

    Ok(AllocationLayout {
        byte_len: layout.byte_len,
        alignment: layout.alignment(),
        reference_map,
        is_noscan,
        has_shared_reference,
        class,
    })
}

impl<'a> BlockLowerer<'a> {
    /// Lower one managed allocation.
    pub(super) fn lower_new(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
    ) -> Result<Instruction> {
        // resolve allocation target and layout
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "new destination".to_string(),
            })?;
        let allocation_type = layout.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "new layout".to_string(),
        })?;
        let layout = self.layout_for_type(allocation_type)?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);

        // select the target heap
        let opcode = match pointer_class {
            PointerClass::Heap => Opcode::AllocateHeap,
            PointerClass::SharedHeap => Opcode::AllocateSharedHeap,
            _ => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                });
            }
        };

        // precompute the heap allocation plan
        let allocation = allocation_layout(
            pool,
            pointer_class,
            layout,
            self.heap_options,
            self.shared_heap_options,
        )?;

        Ok(Instruction::new(
            opcode,
            New {
                dest: destination,
                allocation: pool.allocation_layout(allocation),
            },
        ))
    }

    /// Lower one slice allocation.
    pub(super) fn lower_new_slice(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        element: mir::TypeReference,
        length: mir::ValueReference,
        result_type: mir::TypeReference,
    ) -> Result<Instruction> {
        // resolve descriptor, backing element, and dynamic length
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "new.slice destination".to_string(),
            })?;
        let element_type = element.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "new.slice element type".to_string(),
        })?;
        let result_type = result_type
            .ty()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "new.slice result type".to_string(),
            })?;
        let length = length.value().ok_or_else(|| Error::MissingRepresentation {
            context: "new.slice length".to_string(),
        })?;

        // compile the backing element plan
        let element_layout = self.layout_for_type(element_type)?;
        let element_alignment = element_layout.alignment();
        let pointer_class = slice_backing_pointer_class(self.tree, result_type)?;
        let element = allocation_layout(
            pool,
            pointer_class,
            element_layout,
            self.heap_options,
            self.shared_heap_options,
        )?;

        Ok(Instruction::new(
            Opcode::AllocateSlice,
            NewSlice {
                dest: destination,
                length,
                pointer_class,
                element: pool.allocation_layout(element),
                element_alignment,
            },
        ))
    }

    /// Lower one raw allocation.
    pub(super) fn lower_raw_alloc(
        &self,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
    ) -> Result<Instruction> {
        // resolve raw destination and byte width
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "raw alloc destination".to_string(),
            })?;
        let layout = layout.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "raw alloc layout".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::AllocateRaw,
            RawAlloc {
                dest: destination,
                byte_len: self.byte_len_for_type(layout)?,
            },
        ))
    }

    /// Lower one raw free.
    pub(super) fn lower_raw_free(&self, pointer: mir::ValueReference) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "raw free pointer".to_string(),
            })?;

        Ok(Instruction::new(Opcode::FreeRaw, RawFree { pointer }))
    }

    /// Lower one stack allocation.
    pub(super) fn lower_stack_alloc(
        &self,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
    ) -> Result<Instruction> {
        // resolve stack destination and compiled type
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "stack alloc destination".to_string(),
            })?;
        let allocation_type = layout.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "stack alloc layout".to_string(),
        })?;

        Ok(Instruction::new(
            Opcode::AllocateStack,
            StackAlloc {
                dest: destination,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
                allocation_type,
            },
        ))
    }
}
