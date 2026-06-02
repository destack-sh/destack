use destack_heap::{AllocationClass, HeapOptions, SharedHeapOptions};
use destack_mir as mir;

use crate::program::{
    AddressSpace, AllocationBranch, AllocationSite, Edge, Instruction, Layout, Op,
    SliceAllocationBranch, SmallAllocationSite, address_space_from_reference, repr_type,
};
use crate::{Error, Result};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::memory::frame_value_slot;
use super::pool::Pool;
use super::projection::slice_projection;
use super::value::address_space_for_value;

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
        let address_space = address_space_for_value(self.value_shape_map(), destination)?;

        // precompute the heap allocation shape
        let (allocation, class) = allocation_site(
            pool,
            address_space,
            layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let op = allocation_op(
            address_space,
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
            cell_offset(self, destination)?,
            allocation,
            0,
            0,
        ))
    }

    /// Lower one fallible heap allocation.
    pub(super) fn lower_new_try(
        &self,
        pool: &mut Pool<'_, '_>,
        layout: mir::TypeReference,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
        initialization: AllocationInitialization,
    ) -> Result<Instruction> {
        let allocation_type = layout
            .ty()
            .ok_or_else(|| Error::invalid_program("new.try layout"))?;
        let (result, success) = self.lower_allocation_success(pool, success)?;

        let layout = self.layout_for_type(allocation_type)?;
        let address_space = address_space_for_value(self.value_shape_map(), result)?;
        let (allocation, _) = allocation_site(
            pool,
            address_space,
            layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let allocation = pool.allocation_site(allocation);
        let failure = self.lower_block_edge(pool, failure, "new.try failure")?;
        let record = AllocationBranch {
            destination: cell_offset(self, result)?,
            allocation,
            success,
            failure,
        };
        let op = allocation_branch_op(address_space, initialization)?;

        Ok(pool.instruction_with_side(op, record))
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

        if destination_slot.is_cell && value_slot.is_cell {
            return Ok(Instruction::new(
                Op::MoveCell,
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
        let address_space = slice_backing_address_space(self.tree, result_type)?;
        let (element, _) = allocation_site(
            pool,
            address_space,
            element_layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let access = slice_projection(self.tree, self.layouts(), result_type)
            .ok_or(Error::invalid_instruction())?;
        let element = pool.allocation_site(element);
        let access = pool.slice_projection(access);

        let op = match address_space {
            AddressSpace::Local => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSliceZeroed,
                AllocationInitialization::Uninit => Op::AllocateSliceUninit,
            },
            AddressSpace::Shared => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSharedSliceZeroed,
                AllocationInitialization::Uninit => Op::AllocateSharedSliceUninit,
            },
            _ => {
                return Err(Error::invalid_pointer_type(format!("{address_space:?}")));
            }
        };
        Ok(Instruction::new(
            op,
            value_offset(self, destination)?,
            cell_offset(self, length)?,
            element.0,
            access.0,
        ))
    }

    /// Lower one fallible slice allocation.
    pub(super) fn lower_new_slice_try(
        &self,
        pool: &mut Pool<'_, '_>,
        element: mir::TypeReference,
        length: mir::ValueReference,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
        initialization: AllocationInitialization,
    ) -> Result<Instruction> {
        let element_type = element
            .ty()
            .ok_or_else(|| Error::invalid_program("new.slice.try element type"))?;
        let (result, success) = self.lower_allocation_success(pool, success)?;
        let result_type = self.value_type_for_value(result)?;
        let length = length
            .value()
            .ok_or_else(|| Error::invalid_program("new.slice.try length"))?;

        let element_layout = self.layout_for_type(element_type)?;
        let address_space = slice_backing_address_space(self.tree, result_type)?;
        let (element, _) = allocation_site(
            pool,
            address_space,
            element_layout,
            self.heap_options,
            self.shared_heap_options,
        )?;
        let element = pool.allocation_site(element);
        let access = slice_projection(self.tree, self.layouts(), result_type)
            .ok_or(Error::invalid_instruction())?;
        let failure = self.lower_block_edge(pool, failure, "new.slice.try failure")?;
        let record = SliceAllocationBranch {
            destination: value_offset(self, result)?,
            length: cell_offset(self, length)?,
            element,
            access: pool.slice_projection(access),
            success,
            failure,
        };
        let op = slice_allocation_branch_op(address_space, initialization)?;

        Ok(pool.instruction_with_side(op, record))
    }

    /// Lower one fallible allocation success edge and return its result parameter.
    fn lower_allocation_success(
        &self,
        pool: &mut Pool<'_, '_>,
        target: &mir::BlockTarget,
    ) -> Result<(mir::Value, Edge)> {
        let target_block = (target.block)
            .block()
            .ok_or_else(|| Error::invalid_program("allocation success target"))?;
        let target_index = self.block_index_by_id[&target_block];
        let target_parameters = self.block_parameter[target_index].as_slice();
        let Some((&result, remaining_parameters)) = target_parameters.split_first() else {
            return Err(Error::invalid_program("allocation success parameter"));
        };
        let arguments = target
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::invalid_program("allocation success argument"))
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = pool.edge_moves(remaining_parameters, &arguments)?;
        let edge = Edge {
            target: target_index as u32,
            moves,
        };

        Ok((result, edge))
    }

    /// Lower one normal block edge.
    fn lower_block_edge(
        &self,
        pool: &mut Pool<'_, '_>,
        target: &mir::BlockTarget,
        context: &str,
    ) -> Result<Edge> {
        let target_block = (target.block)
            .block()
            .ok_or_else(|| Error::invalid_program(context))?;
        let target_index = self.block_index_by_id[&target_block];
        let target_parameters = self.block_parameter[target_index].as_slice();
        let arguments = target
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::invalid_program(context))
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = pool.edge_moves(target_parameters, &arguments)?;

        Ok(Edge {
            target: target_index as u32,
            moves,
        })
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
        let address_space = address_space_for_value(self.value_shape_map(), destination)?;
        if !matches!(address_space, AddressSpace::Stack) {
            return Err(Error::invalid_pointer_type(format!("{address_space:?}")));
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
            cell_offset(self, destination)?,
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

        // select the heap family from value shape
        let op = match address_space_for_value(self.value_shape_map(), value)? {
            AddressSpace::Local => Op::PinHeap,
            AddressSpace::Shared => Op::PinSharedHeap,
            address_space => {
                return Err(Error::invalid_pointer_type(format!("{address_space:?}")));
            }
        };

        Ok(Instruction::new(
            op,
            cell_offset(self, destination)?,
            cell_offset(self, value)?,
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

        // select the heap family from value shape
        let op = match address_space_for_value(self.value_shape_map(), value)? {
            AddressSpace::Local => Op::UnpinHeap,
            AddressSpace::Shared => Op::UnpinSharedHeap,
            address_space => {
                return Err(Error::invalid_pointer_type(format!("{address_space:?}")));
            }
        };

        Ok(Instruction::new(op, cell_offset(self, value)?, 0, 0, 0))
    }

    /// Lower one unique heap free.
    pub(super) fn lower_free(&self, value: mir::ValueReference) -> Result<Instruction> {
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("free value"))?;
        let value_type = self.value_type_for_value(value)?;
        let op = unique_free_op(self.tree, value_type)?;

        Ok(Instruction::new(op, cell_offset(self, value)?, 0, 0, 0))
    }
}

/// Encode one power-of-two alignment into an instruction field.
fn encode_alignment_log2(alignment: usize) -> u32 {
    alignment.trailing_zeros()
}

/// Return the address space for one slice backing allocation.
fn slice_backing_address_space(
    tree: &mir::Tree,
    result_type: mir::LocalNodeId<mir::Type>,
) -> Result<AddressSpace> {
    let mir::Type::Slice { kind, space, .. } = tree.get(result_type) else {
        return Err(Error::type_mismatch(
            "slice result type",
            format!("{result_type:?}"),
        ));
    };

    let address_space = address_space_from_reference(space.clone(), *kind);
    match address_space {
        AddressSpace::Local | AddressSpace::Shared => Ok(address_space),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Build one allocation site for a concrete MIR type.
fn allocation_site(
    pool: &mut Pool<'_, '_>,
    address_space: AddressSpace,
    layout: &Layout,
    heap_options: &HeapOptions,
    shared_heap_options: &SharedHeapOptions,
) -> Result<(AllocationSite, AllocationClass)> {
    // resolve the allocation class from the destination space
    let is_noscan = !layout.trace_map.has_reference();
    let has_shared_reference = layout.trace_map.has_shared_reference();
    let trace_map = pool.trace_map(&layout.trace_map)?;
    let trace_id = if is_noscan { None } else { Some(trace_map) };
    let class = match address_space {
        AddressSpace::Local => {
            heap_options.allocation_class(layout.byte_len, layout.alignment(), trace_id, is_noscan)
        }
        AddressSpace::Shared => shared_heap_options.allocation_class(
            layout.byte_len,
            layout.alignment(),
            trace_id,
            is_noscan,
        ),
        _ => {
            return Err(Error::invalid_pointer_type(format!("{address_space:?}")));
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
    address_space: AddressSpace,
    class: AllocationClass,
    is_noscan: bool,
    has_shared_reference: bool,
    initialization: AllocationInitialization,
) -> Result<Op> {
    match (address_space, class.small(), is_noscan, initialization) {
        (AddressSpace::Local, Some(_), _, AllocationInitialization::Zeroed)
            if has_shared_reference =>
        {
            Ok(Op::AllocateHeapSmallSharedEdgeZeroed)
        }
        (AddressSpace::Local, Some(_), _, AllocationInitialization::Uninit)
            if has_shared_reference =>
        {
            Ok(Op::AllocateHeapSmallSharedEdgeUninit)
        }
        (AddressSpace::Local, Some(_), true, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateHeapSmallNoscanZeroed)
        }
        (AddressSpace::Local, Some(_), true, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateHeapSmallNoscanUninit)
        }
        (AddressSpace::Local, Some(_), false, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateHeapSmallScanZeroed)
        }
        (AddressSpace::Local, Some(_), false, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateHeapSmallScanUninit)
        }
        (AddressSpace::Local, None, _, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateHeapZeroed)
        }
        (AddressSpace::Local, None, _, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateHeapUninit)
        }
        (AddressSpace::Shared, Some(_), _, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSharedHeapSmallZeroed)
        }
        (AddressSpace::Shared, Some(_), _, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSharedHeapSmallUninit)
        }
        (AddressSpace::Shared, None, _, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSharedHeapZeroed)
        }
        (AddressSpace::Shared, None, _, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSharedHeapUninit)
        }
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Select one fallible heap allocation operation from destination space.
fn allocation_branch_op(
    address_space: AddressSpace,
    initialization: AllocationInitialization,
) -> Result<Op> {
    match (address_space, initialization) {
        (AddressSpace::Local, AllocationInitialization::Zeroed) => Ok(Op::AllocateHeapZeroedBranch),
        (AddressSpace::Local, AllocationInitialization::Uninit) => Ok(Op::AllocateHeapUninitBranch),
        (AddressSpace::Shared, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSharedHeapZeroedBranch)
        }
        (AddressSpace::Shared, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSharedHeapUninitBranch)
        }
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}

/// Select one fallible slice allocation operation from destination space.
fn slice_allocation_branch_op(
    address_space: AddressSpace,
    initialization: AllocationInitialization,
) -> Result<Op> {
    match (address_space, initialization) {
        (AddressSpace::Local, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSliceZeroedBranch)
        }
        (AddressSpace::Local, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSliceUninitBranch)
        }
        (AddressSpace::Shared, AllocationInitialization::Zeroed) => {
            Ok(Op::AllocateSharedSliceZeroedBranch)
        }
        (AddressSpace::Shared, AllocationInitialization::Uninit) => {
            Ok(Op::AllocateSharedSliceUninitBranch)
        }
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
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

    let address_space = address_space_from_reference(space.clone(), mir::ReferenceKind::Unique);
    match address_space {
        AddressSpace::Local => Ok(Op::FreeHeap),
        AddressSpace::Shared => Ok(Op::FreeSharedHeap),
        _ => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
    }
}
