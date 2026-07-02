use destack_heap::{AllocationClass, AllocationPlan, AllocationShape};
use destack_mir as mir;

use destack_program::AddressSpace;
use destack_program::vm::{AllocationBranch, Edge, Instruction, Op, SliceAllocationBranch};

use crate::LinkResult;

use super::layout::StorageLayout;
use super::lower::BlockLowerer;
use super::pool::Pool;

/// Allocation byte initialization mode.
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
        destination: mir::Value,
        layout: mir::TypeId,
        result_type: mir::TypeId,
        initialization: AllocationInitialization,
    ) -> LinkResult<Instruction> {
        // resolve allocation target and layout
        let layout = self.layout_for_type(layout)?;
        let address_space = self.function.address_space_for_type(result_type)?;

        // precompute the heap allocation shape
        let (allocation, class) = self.allocation_plan(pool, address_space, layout)?;
        let op = self.allocation_op(
            address_space,
            class,
            allocation.is_noscan(),
            allocation.has_shared_reference(),
            initialization,
        )?;
        // pool the side-table shape consumed by the selected opcode
        let allocation = if is_small_allocation_op(op) {
            let Some(small) = allocation.small_allocation() else {
                return Err(self.invalid_input("small allocation plan"));
            };

            pool.small_allocation_plan(small).0
        } else {
            pool.allocation_plan(allocation).0
        };

        Ok(Instruction::new(
            op,
            self.cell_offset(destination)?,
            allocation,
            0,
            0,
        ))
    }

    /// Lower one fallible heap allocation.
    pub(super) fn lower_new_try(
        &self,
        pool: &mut Pool<'_, '_>,
        layout: mir::TypeId,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
        initialization: AllocationInitialization,
    ) -> LinkResult<Instruction> {
        let (result, success) = self.lower_allocation_success(pool, success)?;

        let layout = self.layout_for_type(layout)?;
        let address_space = self
            .operand_map()
            .address_space(result)
            .ok_or_else(|| self.invalid_pointer_type(format!("{result:?}")))?;
        let (allocation, _) = self.allocation_plan(pool, address_space, layout)?;
        let allocation = pool.allocation_plan(allocation);
        let failure = self.lower_block_edge(pool, failure)?;
        let record = AllocationBranch {
            destination: self.cell_offset(result)?,
            allocation,
            success,
            failure,
        };
        let op = self.allocation_branch_op(address_space, initialization)?;

        Ok(pool.instruction_with_side(op, record))
    }

    /// Lower one heap allocation completion.
    pub(super) fn lower_new_complete(
        &self,
        destination: mir::Value,
        value: mir::Value,
    ) -> LinkResult<Instruction> {
        let destination_slot = self.frame_value_slot(destination)?;
        let value_slot = self.frame_value_slot(value)?;

        if self.function.slot_is_cell(destination_slot) && self.function.slot_is_cell(value_slot) {
            return Ok(Instruction::new(
                Op::MoveCell,
                destination_slot.offset,
                value_slot.offset,
                0,
                0,
            ));
        }

        if destination_slot.byte_len() != value_slot.byte_len() {
            return Err(self.invalid_instruction("new complete byte length"));
        }

        Ok(Instruction::new(
            Op::MoveAggregate,
            destination_slot.offset,
            destination_slot.byte_len(),
            value_slot.offset,
            0,
        ))
    }

    /// Lower one slice allocation.
    pub(super) fn lower_new_slice(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        element: mir::TypeId,
        length: mir::Value,
        result_type: mir::TypeId,
        initialization: AllocationInitialization,
    ) -> LinkResult<Instruction> {
        // resolve descriptor, backing element, and dynamic length
        let element_layout = self.layout_for_type(element)?;
        let address_space = self.slice_backing_address_space(result_type)?;
        let (element, _) = self.allocation_plan(pool, address_space, element_layout)?;
        let access = self
            .slice_projection(result_type)
            .ok_or_else(|| self.invalid_instruction("slice projection"))?;
        let element = pool.allocation_plan(element);
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
                return Err(self.invalid_pointer_type(format!("{address_space:?}")));
            }
        };
        Ok(Instruction::new(
            op,
            self.value_offset(destination)?,
            self.cell_offset(length)?,
            element.0,
            access.0,
        ))
    }

    /// Lower one fallible slice allocation.
    pub(super) fn lower_new_slice_try(
        &self,
        pool: &mut Pool<'_, '_>,
        element: mir::TypeId,
        length: mir::Value,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
        initialization: AllocationInitialization,
    ) -> LinkResult<Instruction> {
        let (result, success) = self.lower_allocation_success(pool, success)?;
        let result_type = self.value_type_for_value(result)?;

        let element_layout = self.layout_for_type(element)?;
        let address_space = self.slice_backing_address_space(result_type)?;
        let (element, _) = self.allocation_plan(pool, address_space, element_layout)?;
        let element = pool.allocation_plan(element);
        let access = self
            .slice_projection(result_type)
            .ok_or_else(|| self.invalid_instruction("slice projection"))?;
        let failure = self.lower_block_edge(pool, failure)?;
        let record = SliceAllocationBranch {
            destination: self.value_offset(result)?,
            length: self.cell_offset(length)?,
            element,
            access: pool.slice_projection(access),
            success,
            failure,
        };
        let op = self.slice_allocation_branch_op(address_space, initialization)?;

        Ok(pool.instruction_with_side(op, record))
    }

    /// Lower one fallible allocation success edge and return its result parameter.
    fn lower_allocation_success(
        &self,
        pool: &mut Pool<'_, '_>,
        target: &mir::BlockTarget,
    ) -> LinkResult<(mir::Value, Edge)> {
        let target_block = target.block;
        let target_index = self.function.block_index_by_id[&target_block];
        let target_parameters = self.function.block_parameters[target_index].as_slice();
        let Some((&result, remaining_parameters)) = target_parameters.split_first() else {
            return Err(self.invalid_input("allocation success parameter"));
        };
        let arguments = self.function.target_arguments(target);
        let moves = pool.edge_moves(remaining_parameters, arguments)?;
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
    ) -> LinkResult<Edge> {
        let target_block = target.block;
        let target_index = self.function.block_index_by_id[&target_block];
        let target_parameters = self.function.block_parameters[target_index].as_slice();
        let arguments = self.function.target_arguments(target);
        let moves = pool.edge_moves(target_parameters, arguments)?;

        Ok(Edge {
            target: target_index as u32,
            moves,
        })
    }

    /// Lower one frame allocation.
    pub(super) fn lower_frame_alloc(
        &self,
        destination: mir::Value,
        layout: mir::TypeId,
        result_type: mir::TypeId,
        initialization: AllocationInitialization,
    ) -> LinkResult<Instruction> {
        // validate stack destination and frame allocation type
        // frame allocation must produce a frame allocation pointer
        self.validate_frame_allocation_type(result_type)?;

        // encode the exact layout into the instruction
        let layout = self.layout_for_type(layout)?;
        let byte_len = layout.byte_len() as u64;
        let alignment = encode_alignment_log2(layout.alignment());

        Ok(Instruction::new(
            match initialization {
                AllocationInitialization::Zeroed => Op::AllocateStackZeroed,
                AllocationInitialization::Uninit => Op::AllocateStackUninit,
            },
            self.cell_offset(destination)?,
            byte_len as u32,
            (byte_len >> 32) as u32,
            alignment,
        ))
    }

    /// Validate one frame allocation result type.
    fn validate_frame_allocation_type(&self, result_type: mir::TypeId) -> LinkResult<()> {
        let result_type = self.function.tree.repr_type(result_type);
        let mir::Type::Reference { space, .. } = self.function.tree.get(result_type) else {
            return Err(self.invalid_pointer_type(format!("{result_type:?}")));
        };

        match space {
            mir::Space::Frame => Ok(()),
            space => Err(self.invalid_pointer_type(format!("{space:?}"))),
        }
    }

    /// Lower one heap pin.
    pub(super) fn lower_pin(
        &self,
        destination: mir::Value,
        value: mir::Value,
    ) -> LinkResult<Instruction> {
        // select the heap family from the value operand
        let address_space = self
            .operand_map()
            .address_space(value)
            .ok_or_else(|| self.invalid_pointer_type(format!("{value:?}")))?;
        let op = match address_space {
            AddressSpace::Local => Op::PinHeap,
            AddressSpace::Shared => Op::PinSharedHeap,
            address_space => {
                return Err(self.invalid_pointer_type(format!("{address_space:?}")));
            }
        };

        Ok(Instruction::new(
            op,
            self.cell_offset(destination)?,
            self.cell_offset(value)?,
            0,
            0,
        ))
    }

    /// Lower one heap unpin.
    pub(super) fn lower_unpin(&self, value: mir::Value) -> LinkResult<Instruction> {
        // select the heap family from the value operand
        let address_space = self
            .operand_map()
            .address_space(value)
            .ok_or_else(|| self.invalid_pointer_type(format!("{value:?}")))?;
        let op = match address_space {
            AddressSpace::Local => Op::UnpinHeap,
            AddressSpace::Shared => Op::UnpinSharedHeap,
            address_space => {
                return Err(self.invalid_pointer_type(format!("{address_space:?}")));
            }
        };

        Ok(Instruction::new(op, self.cell_offset(value)?, 0, 0, 0))
    }

    /// Lower one unique heap free.
    pub(super) fn lower_free(&self, value: mir::Value) -> LinkResult<Instruction> {
        let value_type = self.value_type_for_value(value)?;
        let op = self.unique_free_op(value_type)?;

        Ok(Instruction::new(op, self.cell_offset(value)?, 0, 0, 0))
    }

    /// Return the address space for one slice backing allocation.
    fn slice_backing_address_space(
        &self,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<AddressSpace> {
        let mir::Type::Slice { kind, space, .. } = self.function.tree.get(result_type) else {
            return Err(self.type_mismatch("slice result type", format!("{result_type:?}")));
        };

        let address_space = self
            .function
            .address_space_for_reference(space.clone(), *kind);
        match address_space {
            AddressSpace::Local | AddressSpace::Shared => Ok(address_space),
            _ => Err(self.invalid_pointer_type(format!("{address_space:?}"))),
        }
    }

    /// Select one free operation for one unique reference type.
    fn unique_free_op(&self, ty: mir::LocalNodeId<mir::Type>) -> LinkResult<Op> {
        let ty = self.function.tree.repr_type(ty);
        let mir::Type::Reference {
            kind: mir::ReferenceKind::Unique,
            space,
            ..
        } = self.function.tree.get(ty)
        else {
            return Err(self.invalid_pointer_type(format!("{ty:?}")));
        };

        let address_space = self
            .function
            .address_space_for_reference(space.clone(), mir::ReferenceKind::Unique);
        match address_space {
            AddressSpace::Local => Ok(Op::FreeHeap),
            AddressSpace::Shared => Ok(Op::FreeSharedHeap),
            _ => Err(self.invalid_pointer_type(format!("{address_space:?}"))),
        }
    }
}

/// Encode one power-of-two alignment into an instruction field.
fn encode_alignment_log2(alignment: usize) -> u32 {
    alignment.trailing_zeros()
}

impl BlockLowerer<'_> {
    /// Build one allocation plan for a concrete MIR type.
    fn allocation_plan(
        &self,
        pool: &mut Pool<'_, '_>,
        address_space: AddressSpace,
        layout: &StorageLayout,
    ) -> LinkResult<(AllocationPlan, AllocationClass)> {
        // resolve the allocation class from the destination space
        let is_noscan = !layout.trace_map.has_reference();
        let trace_map = pool.trace_map(&layout.trace_map)?;
        let trace_id = if is_noscan { None } else { Some(trace_map) };
        let shape = AllocationShape::new(
            layout.byte_len(),
            layout.alignment(),
            trace_id,
            layout.trace_map.clone(),
        );
        let allocation = match address_space {
            AddressSpace::Local => self.function.heap_options.allocation_plan(&shape),
            AddressSpace::Shared => self.function.shared_heap_options.allocation_plan(&shape),
            _ => {
                return Err(self.invalid_pointer_type(format!("{address_space:?}")));
            }
        };
        let class = allocation.class;

        Ok((allocation, class))
    }

    /// Select one heap allocation operation from destination and size class.
    fn allocation_op(
        &self,
        address_space: AddressSpace,
        class: AllocationClass,
        is_noscan: bool,
        has_shared_reference: bool,
        initialization: AllocationInitialization,
    ) -> LinkResult<Op> {
        match (address_space, class.as_small(), is_noscan, initialization) {
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
            _ => Err(self.invalid_pointer_type(format!("{address_space:?}"))),
        }
    }

    /// Select one fallible heap allocation operation from destination space.
    fn allocation_branch_op(
        &self,
        address_space: AddressSpace,
        initialization: AllocationInitialization,
    ) -> LinkResult<Op> {
        match (address_space, initialization) {
            (AddressSpace::Local, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateHeapZeroedBranch)
            }
            (AddressSpace::Local, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateHeapUninitBranch)
            }
            (AddressSpace::Shared, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateSharedHeapZeroedBranch)
            }
            (AddressSpace::Shared, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateSharedHeapUninitBranch)
            }
            _ => Err(self.invalid_pointer_type(format!("{address_space:?}"))),
        }
    }

    /// Select one fallible slice allocation operation from destination space.
    fn slice_allocation_branch_op(
        &self,
        address_space: AddressSpace,
        initialization: AllocationInitialization,
    ) -> LinkResult<Op> {
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
            _ => Err(self.invalid_pointer_type(format!("{address_space:?}"))),
        }
    }
}

/// Return whether one allocation operation consumes a small allocation plan.
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
