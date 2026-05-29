use destack_heap::{AllocationClass, HeapOptions, SharedHeapOptions};
use destack_mir as mir;

use crate::program::{
    AllocationSite, Instruction, Layout, Op, PointerClass, SmallAllocationSite,
    pointer_class_from_reference, repr_type,
};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::memory::frame_value_slot;
use super::pool::Pool;
use super::projection::slice_projection;
use super::value::pointer_class_for_value;

#[derive(Clone, Copy)]
pub(super) enum AllocationInitialization {
    /// Initialize the allocation to zero bytes.
    Zeroed,
    /// Leave allocation bytes uninitialized.
    Uninit,
}

impl<'a> BlockLowerer<'a> {
    /// Lower one heap allocation.
    pub(super) fn lower_new(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
        initialization: AllocationInitialization,
    ) -> Result<Instruction> {
        // resolve allocation target and layout
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("new destination"))?;
        let allocation_type = layout
            .ty()
            .ok_or_else(|| Error::invalid_program("new layout"))?;
        let layout = self.layout_for_type(allocation_type)?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);

        // precompute the heap allocation shape
        let (allocation, class) = allocation_site(
            pool,
            pointer_class,
            layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let op = allocation_op(
            pointer_class,
            class,
            allocation.heap.is_noscan,
            allocation.heap.has_shared_reference,
            initialization,
        )?;
        // pool the side-table shape consumed by the selected opcode
        let allocation = if is_small_allocation_op(op) {
            let Some(small) = allocation.heap.small_site() else {
                return Err(Error::invalid_program("small allocation site"));
            };

            let allocation = SmallAllocationSite {
                heap: allocation.heap,
                small,
                trace_map: allocation.trace_map,
            };
            pool.small_allocation_site(allocation).0
        } else {
            pool.allocation_site(allocation).0
        };

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            allocation,
            0,
            0,
        ))
    }

    /// Lower one heap allocation completion.
    pub(super) fn lower_new_complete(
        &self,
        destination: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("new.complete destination"))?;
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("new.complete value"))?;
        let destination_slot = frame_value_slot(self, destination)?;
        let value_slot = frame_value_slot(self, value)?;

        if destination_slot.is_word && value_slot.is_word {
            return Ok(Instruction::new(
                Op::MoveWord,
                destination_slot.offset,
                value_slot.offset,
                0,
                0,
            ));
        }

        if destination_slot.byte_len != value_slot.byte_len {
            return Err(Error::invalid_instruction());
        }

        Ok(Instruction::new(
            Op::MoveFrame,
            destination_slot.offset,
            destination_slot.byte_len,
            value_slot.offset,
            0,
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
        initialization: AllocationInitialization,
    ) -> Result<Instruction> {
        // resolve descriptor, backing element, and dynamic length
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("new.slice destination"))?;
        let element_type = element
            .ty()
            .ok_or_else(|| Error::invalid_program("new.slice element type"))?;
        let result_type = result_type
            .ty()
            .ok_or_else(|| Error::invalid_program("new.slice result type"))?;
        let length = length
            .value()
            .ok_or_else(|| Error::invalid_program("new.slice length"))?;

        // compile the backing element shape
        let element_layout = self.layout_for_type(element_type)?;
        let pointer_class = slice_backing_pointer_class(self.tree, result_type)?;
        let (element, _) = allocation_site(
            pool,
            pointer_class,
            element_layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let access = slice_projection(self.tree, self.layouts(), result_type)
            .ok_or(Error::invalid_instruction())?;
        let element = pool.allocation_site(element);
        let access = pool.slice_projection(access);

        let op = match pointer_class {
            PointerClass::Heap => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSliceZeroed,
                AllocationInitialization::Uninit => Op::AllocateSliceUninit,
            },
            PointerClass::SharedHeap => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSharedSliceZeroed,
                AllocationInitialization::Uninit => Op::AllocateSharedSliceUninit,
            },
            _ => {
                return Err(Error::invalid_pointer_type(format!("{pointer_class:?}")));
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
        initialization: AllocationInitialization,
    ) -> Result<Instruction> {
        // resolve raw destination and byte width
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("raw alloc destination"))?;
        let layout = layout
            .ty()
            .ok_or_else(|| Error::invalid_program("raw alloc layout"))?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);
        let op = match pointer_class {
            PointerClass::Raw => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateRawZeroed,
                AllocationInitialization::Uninit => Op::AllocateRawUninit,
            },
            PointerClass::SharedRaw => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSharedRawZeroed,
                AllocationInitialization::Uninit => Op::AllocateSharedRawUninit,
            },
            _ => {
                return Err(Error::invalid_pointer_type(format!("{pointer_class:?}")));
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
            .ok_or_else(|| Error::invalid_program("raw free pointer"))?;
        let op = match pointer_class_for_value(self.value_layout_map(), pointer) {
            PointerClass::Raw => Op::FreeRaw,
            PointerClass::SharedRaw => Op::FreeSharedRaw,
            _ => {
                return Err(Error::invalid_pointer_type("non raw pointer"));
            }
        };

        Ok(Instruction::new(op, word_offset(self, pointer)?, 0, 0, 0))
    }

    /// Lower one frame allocation.
    pub(super) fn lower_frame_alloc(
        &self,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
        initialization: AllocationInitialization,
    ) -> Result<Instruction> {
        // resolve stack destination and compiled type
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("frame alloc destination"))?;
        let allocation_type = layout
            .ty()
            .ok_or_else(|| Error::invalid_program("frame alloc layout"))?;

        // frame allocation must produce a frame allocation pointer
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);
        if !matches!(pointer_class, PointerClass::Stack) {
            return Err(Error::invalid_pointer_type(format!("{pointer_class:?}")));
        }

        // encode the exact layout into the instruction
        let layout = self.layout_for_type(allocation_type)?;
        let byte_len = layout.byte_len as u64;
        let alignment = encode_alignment_log2(layout.alignment());

        Ok(Instruction::new(
            match initialization {
                AllocationInitialization::Zeroed => Op::AllocateStackZeroed,
                AllocationInitialization::Uninit => Op::AllocateStackUninit,
            },
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
            .ok_or_else(|| Error::invalid_program("pin destination"))?;
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("pin value"))?;

        // select the heap family from value layout
        let op = match pointer_class_for_value(self.value_layout_map(), value) {
            PointerClass::Heap => Op::PinHeap,
            PointerClass::SharedHeap => Op::PinSharedHeap,
            pointer_class => {
                return Err(Error::invalid_pointer_type(format!("{pointer_class:?}")));
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
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("unpin value"))?;

        // select the heap family from value layout
        let op = match pointer_class_for_value(self.value_layout_map(), value) {
            PointerClass::Heap => Op::UnpinHeap,
            PointerClass::SharedHeap => Op::UnpinSharedHeap,
            pointer_class => {
                return Err(Error::invalid_pointer_type(format!("{pointer_class:?}")));
            }
        };

        Ok(Instruction::new(op, word_offset(self, value)?, 0, 0, 0))
    }

    /// Lower one unique heap free.
    pub(super) fn lower_free(&self, value: mir::ValueReference) -> Result<Instruction> {
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("free value"))?;
        let value_type = self.value_type_for_value(value)?;
        let op = unique_free_op(self.tree, value_type)?;

        Ok(Instruction::new(op, word_offset(self, value)?, 0, 0, 0))
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
    let mir::Type::Slice { kind, space, .. } = tree.get(result_type) else {
        return Err(Error::type_mismatch(
            "slice result type",
            format!("{result_type:?}"),
        ));
    };

    let pointer_class = pointer_class_from_reference(space.clone(), *kind);
    match pointer_class {
        PointerClass::Heap | PointerClass::SharedHeap => Ok(pointer_class),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Build one allocation site for a concrete MIR type.
fn allocation_site(
    pool: &mut Pool<'_, '_>,
    pointer_class: PointerClass,
    layout: &Layout,
    heap_options: &HeapOptions,
    shared_heap_options: &SharedHeapOptions,
) -> Result<(AllocationSite, AllocationClass)> {
    // resolve the allocation class from the destination space
    let is_noscan = !layout.trace_map.has_reference();
    let has_shared_reference = layout.trace_map.has_shared_reference();
    let trace_map = pool.trace_map(&layout.trace_map)?;
    let trace_id = if is_noscan { None } else { Some(trace_map) };
    let class = match pointer_class {
        PointerClass::Heap => {
            heap_options.allocation_class(layout.byte_len, layout.alignment(), trace_id, is_noscan)
        }
        PointerClass::SharedHeap => shared_heap_options.allocation_class(
            layout.byte_len,
            layout.alignment(),
            trace_id,
            is_noscan,
        ),
        _ => {
            return Err(Error::invalid_pointer_type(format!("{pointer_class:?}")));
        }
    };

    let allocation = AllocationSite {
        heap: destack_heap::AllocationSite {
            byte_len: layout.byte_len,
            alignment: layout.alignment(),
            trace_id,
            is_noscan,
            has_shared_reference,
            class,
        },
        trace_map,
    };

    Ok((allocation, class))
}

/// Select one heap allocation operation from destination and size class.
fn allocation_op(
    pointer_class: PointerClass,
    class: AllocationClass,
    is_noscan: bool,
    has_shared_reference: bool,
    initialization: AllocationInitialization,
) -> Result<Op> {
    match (pointer_class, class.small(), is_noscan, initialization) {
        (PointerClass::Heap, Some(_), _, AllocationInitialization::Zeroed)
            if has_shared_reference =>
        {
            Ok(Op::AllocateHeapSmallSharedEdgeZeroed)
        }
        (PointerClass::Heap, Some(_), _, AllocationInitialization::Uninit)
            if has_shared_reference =>
        {
            Ok(Op::AllocateHeapSmallSharedEdgeUninit)
        }
        (PointerClass::Heap, Some(_), true, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateHeapSmallNoscanZeroed)
        }
        (PointerClass::Heap, Some(_), true, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateHeapSmallNoscanUninit)
        }
        (PointerClass::Heap, Some(_), false, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateHeapSmallScanZeroed)
        }
        (PointerClass::Heap, Some(_), false, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateHeapSmallScanUninit)
        }
        (PointerClass::Heap, None, _, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateHeapZeroed)
        }
        (PointerClass::Heap, None, _, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateHeapUninit)
        }
        (PointerClass::SharedHeap, Some(_), _, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSharedHeapSmallZeroed)
        }
        (PointerClass::SharedHeap, Some(_), _, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSharedHeapSmallUninit)
        }
        (PointerClass::SharedHeap, None, _, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSharedHeapZeroed)
        }
        (PointerClass::SharedHeap, None, _, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSharedHeapUninit)
        }
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}

/// Return whether one allocation operation consumes a small allocation site.
fn is_small_allocation_op(op: Op) -> bool {
    matches!(
        op,
        Op::AllocateHeapSmallNoscanZeroed
            | Op::AllocateHeapSmallNoscanUninit
            | Op::AllocateHeapSmallScanZeroed
            | Op::AllocateHeapSmallScanUninit
            | Op::AllocateHeapSmallSharedEdgeZeroed
            | Op::AllocateHeapSmallSharedEdgeUninit
            | Op::AllocateSharedHeapSmallZeroed
            | Op::AllocateSharedHeapSmallUninit
    )
}

/// Select one free operation for one unique heap reference type.
fn unique_free_op(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<Op> {
    let ty = repr_type(tree, ty);
    let mir::Type::Reference {
        kind: mir::ReferenceKind::Unique,
        space,
        ..
    } = tree.get(ty)
    else {
        return Err(Error::invalid_pointer_type(format!("{ty:?}")));
    };

    let pointer_class = pointer_class_from_reference(space.clone(), mir::ReferenceKind::Unique);
    match pointer_class {
        PointerClass::Heap => Ok(Op::FreeHeap),
        PointerClass::SharedHeap => Ok(Op::FreeSharedHeap),
        _ => Err(Error::invalid_pointer_type(format!("{pointer_class:?}"))),
    }
}
