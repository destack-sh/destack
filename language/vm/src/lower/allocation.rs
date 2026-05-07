use destack_heap::{AllocationClass, HeapOptions, SmallAllocationLayout};
use destack_mir as mir;

use crate::program::{
    AllocationLayout, Instruction, Layout, Op, PointerClass, Projection,
    pointer_class_from_reference, repr_type, word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::projection::slice_projection;
use super::value::pointer_class_for_value;

impl<'a> BlockLowerer<'a> {
    /// Lower one heap allocation.
    pub(super) fn lower_new(
        &self,
        pool: &mut Pool<'_, '_>,
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

        // precompute the heap allocation shape
        let (allocation, small) = allocation_layout(
            pool,
            pointer_class,
            layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let allocation = pool.allocation_layout(allocation);
        let op = allocation_op(pointer_class, small)?;
        let slot_bytes = small.map_or(0, |small| small.slot_bytes() as u32);
        let bucket_index = small.map_or(0, |small| small.bucket_index() as u32);

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            allocation.0,
            slot_bytes,
            bucket_index,
        ))
    }

    /// Lower one slice allocation.
    pub(super) fn lower_new_slice(
        &self,
        pool: &mut Pool<'_, '_>,
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

        // compile the backing element shape
        let element_layout = self.layout_for_type(element_type)?;
        let pointer_class = slice_backing_pointer_class(self.tree, result_type)?;
        let (element, _) = allocation_layout(
            pool,
            pointer_class,
            element_layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let access = slice_projection(self.tree, self.layouts(), result_type)
            .ok_or(Error::InvalidInstruction)?;
        let element = pool.allocation_layout(element);
        let access = pool.slice_projection(access);

        let op = match pointer_class {
            PointerClass::Heap => Op::AllocateSlice,
            PointerClass::SharedHeap => Op::AllocateSharedSlice,
            _ => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                });
            }
        };
        Ok(Instruction::new(
            op,
            value_offset(self, destination)?,
            word_offset(self, length)?,
            element.0,
            access.0,
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
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);
        let op = match pointer_class {
            PointerClass::Raw => Op::AllocateRaw,
            PointerClass::SharedRaw => Op::AllocateSharedRaw,
            _ => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                });
            }
        };

        let layout = self.layout_for_type(layout)?;
        let byte_len = layout.byte_len as u64;
        let alignment = encode_alignment_log2(layout.alignment());

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            byte_len as u32,
            (byte_len >> 32) as u32,
            alignment,
        ))
    }

    /// Lower one raw free.
    pub(super) fn lower_raw_free(&self, pointer: mir::ValueReference) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "raw free pointer".to_string(),
            })?;
        let op = match pointer_class_for_value(self.value_layout_map(), pointer) {
            PointerClass::Raw => Op::FreeRaw,
            PointerClass::SharedRaw => Op::FreeSharedRaw,
            _ => {
                return Err(Error::InvalidPointerType {
                    actual: "non raw pointer".to_string(),
                });
            }
        };

        Ok(Instruction::new(op, word_offset(self, pointer)?, 0, 0, 0))
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

        // stack allocation must produce a stack pointer
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);
        if !matches!(pointer_class, PointerClass::Stack) {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer_class:?}"),
            });
        }

        // encode the exact layout into the instruction
        let layout = self.layout_for_type(allocation_type)?;
        let byte_len = layout.byte_len as u64;
        let alignment = encode_alignment_log2(layout.alignment());

        Ok(Instruction::new(
            Op::AllocateStack,
            word_offset(self, destination)?,
            byte_len as u32,
            (byte_len >> 32) as u32,
            alignment,
        ))
    }

    /// Lower one heap pin.
    pub(super) fn lower_pin(
        &self,
        destination: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve value ids
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "pin destination".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "pin value".to_string(),
        })?;

        // select the heap family from value layout
        let op = match pointer_class_for_value(self.value_layout_map(), value) {
            PointerClass::Heap => Op::PinHeap,
            PointerClass::SharedHeap => Op::PinSharedHeap,
            pointer_class => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                });
            }
        };

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            word_offset(self, value)?,
            0,
            0,
        ))
    }

    /// Lower one heap unpin.
    pub(super) fn lower_unpin(&self, value: mir::ValueReference) -> Result<Instruction> {
        // resolve value id
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "unpin value".to_string(),
        })?;

        // select the heap family from value layout
        let op = match pointer_class_for_value(self.value_layout_map(), value) {
            PointerClass::Heap => Op::UnpinHeap,
            PointerClass::SharedHeap => Op::UnpinSharedHeap,
            pointer_class => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                });
            }
        };

        Ok(Instruction::new(op, word_offset(self, value)?, 0, 0, 0))
    }

    /// Lower one owned-value drop.
    pub(super) fn lower_drop(
        &self,
        pool: &mut Pool<'_, '_>,
        value: mir::ValueReference,
    ) -> Result<Vec<Instruction>> {
        // resolve value and MIR type
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "drop value".to_string(),
        })?;
        let value_type = repr_type(self.tree, self.value_type_for_value(value)?);

        // lower references by ownership and address space
        match self.tree.get(value_type) {
            mir::Type::Reference {
                kind,
                address_space,
                pointee,
                ..
            } => self.lower_reference_drop(value, *kind, address_space.clone(), *pointee),
            mir::Type::Slice {
                kind,
                address_space,
                ..
            } => self.lower_slice_drop(pool, value, *kind, address_space.clone(), value_type),
            _ => Err(Error::TypeMismatch {
                expected: "droppable value".to_string(),
                actual: format!("{value_type:?}"),
            }),
        }
    }

    /// Lower one reference drop.
    fn lower_reference_drop(
        &self,
        value: mir::Value,
        kind: mir::ReferenceKind,
        address_space: mir::AddressSpace,
        pointee: mir::TypeReference,
    ) -> Result<Vec<Instruction>> {
        // managed references are released by tracing
        if matches!(kind, mir::ReferenceKind::Managed) {
            return Ok(Vec::new());
        }

        // owned and stack references have concrete release operations
        let instruction = match (kind, address_space) {
            (mir::ReferenceKind::Owned, mir::AddressSpace::Local) => {
                Instruction::new(Op::DropHeap, word_offset(self, value)?, 0, 0, 0)
            }
            (mir::ReferenceKind::Owned, mir::AddressSpace::Shared) => {
                Instruction::new(Op::DropSharedHeap, word_offset(self, value)?, 0, 0, 0)
            }
            (mir::ReferenceKind::Raw, mir::AddressSpace::Stack) => {
                let pointee = pointee.ty().ok_or_else(|| Error::MissingRepresentation {
                    context: "stack drop pointee".to_string(),
                })?;
                let byte_len = self.byte_len_for_type(pointee)? as u64;

                Instruction::new(
                    Op::DropStack,
                    word_offset(self, value)?,
                    byte_len as u32,
                    (byte_len >> 32) as u32,
                    0,
                )
            }
            (_kind, address_space) => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{address_space:?}"),
                });
            }
        };

        Ok(vec![instruction])
    }

    /// Lower one slice drop.
    fn lower_slice_drop(
        &self,
        pool: &mut Pool<'_, '_>,
        value: mir::Value,
        kind: mir::ReferenceKind,
        address_space: mir::AddressSpace,
        slice_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<Vec<Instruction>> {
        // managed slice backing storage is released by tracing
        if matches!(kind, mir::ReferenceKind::Managed) {
            return Ok(Vec::new());
        }

        // owned slices release the backing allocation selected by address space
        let op = match (kind, address_space) {
            (mir::ReferenceKind::Owned, mir::AddressSpace::Local) => Op::DropSlice,
            (mir::ReferenceKind::Owned, mir::AddressSpace::Shared) => Op::DropSharedSlice,
            (_kind, address_space) => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{address_space:?}"),
                });
            }
        };

        // describe the slice backing pointer once during lowering
        let layout = self.layout_for_type(slice_type)?;
        let slice = layout.slice().ok_or(Error::InvalidInstruction)?;
        let access = Projection::fixed(
            slice.data.ty,
            slice.data.offset,
            slice.data.byte_len,
            word_layout_from_type(self.tree, slice.data.ty),
        );
        let access = pool.projection(access);

        Ok(vec![Instruction::new(
            op,
            value_offset(self, value)?,
            access.0,
            0,
            0,
        )])
    }
}

/// Encode one power-of-two alignment into an instruction field.
fn encode_alignment_log2(alignment: usize) -> u32 {
    alignment.trailing_zeros()
}

/// Return the pointer class for one slice backing allocation.
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
    pool: &mut Pool<'_, '_>,
    pointer_class: PointerClass,
    layout: &Layout,
    heap_options: &HeapOptions,
    shared_heap_options: &HeapOptions,
) -> Result<(AllocationLayout, Option<SmallAllocationLayout>)> {
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

    // intern allocation metadata once during lowering
    let reference_map = pool.reference_map(layout.reference_map.clone());
    let small = match class {
        AllocationClass::Small(small) if is_noscan => Some(small),
        _ => None,
    };
    let class = pool.allocation_class(class);

    let allocation = AllocationLayout {
        byte_len: layout.byte_len,
        alignment: layout.alignment(),
        reference_map,
        is_noscan,
        has_shared_reference,
        class,
    };

    Ok((allocation, small))
}

/// Select one heap allocation operation from destination and size class.
fn allocation_op(pointer_class: PointerClass, small: Option<SmallAllocationLayout>) -> Result<Op> {
    match (pointer_class, small.is_some()) {
        (PointerClass::Heap, true) => Ok(Op::AllocateHeapSmallNoscan),
        (PointerClass::Heap, false) => Ok(Op::AllocateHeap),
        (PointerClass::SharedHeap, true) => Ok(Op::AllocateSharedHeapSmallNoscan),
        (PointerClass::SharedHeap, false) => Ok(Op::AllocateSharedHeap),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}
