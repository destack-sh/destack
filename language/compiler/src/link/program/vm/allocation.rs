use destack_heap::{AllocationClass, AllocationPlan, AllocationShape, DropId};
use destack_mir as mir;

use destack_program::AllocationInitialization;
use destack_program::vm::{AllocationBranch, Edge, Instruction, Op, SliceAllocationBranch};

use crate::LinkResult;

use super::layout::StorageLayout;
use super::lower::BlockLowerer;
use super::pool::Pool;

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
        let layout_type = layout;
        let layout = self.layout_for_type(layout_type)?;
        let space = self.function.space_for_type(result_type)?;
        let drop = self.managed_drop(result_type, layout_type);

        // precompute the heap allocation shape
        let (allocation, class) = self.allocation_plan(pool, space, layout, drop)?;
        let op = self.allocation_op(
            space,
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

        let layout_type = layout;
        let layout = self.layout_for_type(layout_type)?;
        let space = self
            .operand_map()
            .space(result)
            .ok_or_else(|| self.invalid_pointer_type(format!("{result:?}")))?;
        let result_type = self.value_type_for_value(result)?;
        let drop = self.managed_drop(result_type, layout_type);
        let (allocation, _) = self.allocation_plan(pool, space, layout, drop)?;
        let allocation = pool.allocation_plan(allocation);
        let failure = self.lower_block_edge(pool, failure)?;
        let record = AllocationBranch {
            destination: self.cell_offset(result)?,
            allocation,
            success,
            failure,
        };
        let op = self.allocation_branch_op(space, initialization)?;

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
        let space = self.slice_backing_space(result_type)?;
        let drop = self.managed_drop(result_type, element);
        let (element, _) = self.allocation_plan(pool, space, element_layout, drop)?;
        let access = self
            .slice_projection(result_type)
            .ok_or_else(|| self.invalid_instruction("slice projection"))?;
        let element = pool.allocation_plan(element);
        let access = pool.slice_projection(access);

        let op = match space {
            mir::Space::Local => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSliceZeroed,
                AllocationInitialization::Uninit => Op::AllocateSliceUninit,
            },
            mir::Space::Shared => match initialization {
                AllocationInitialization::Zeroed => Op::AllocateSharedSliceZeroed,
                AllocationInitialization::Uninit => Op::AllocateSharedSliceUninit,
            },
            _ => {
                return Err(self.invalid_pointer_type(format!("{space:?}")));
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
        let space = self.slice_backing_space(result_type)?;
        let drop = self.managed_drop(result_type, element);
        let (element, _) = self.allocation_plan(pool, space, element_layout, drop)?;
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
        let op = self.slice_allocation_branch_op(space, initialization)?;

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
        let alignment = layout.alignment_log2();

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
        let space = self
            .operand_map()
            .space(value)
            .ok_or_else(|| self.invalid_pointer_type(format!("{value:?}")))?;
        let op = match space {
            mir::Space::Local => Op::PinHeap,
            mir::Space::Shared => Op::PinSharedHeap,
            space => {
                return Err(self.invalid_pointer_type(format!("{space:?}")));
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
        let space = self
            .operand_map()
            .space(value)
            .ok_or_else(|| self.invalid_pointer_type(format!("{value:?}")))?;
        let op = match space {
            mir::Space::Local => Op::UnpinHeap,
            mir::Space::Shared => Op::UnpinSharedHeap,
            space => {
                return Err(self.invalid_pointer_type(format!("{space:?}")));
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

    /// Return the space for one slice backing allocation.
    pub(super) fn slice_backing_space(
        &self,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<mir::Space> {
        let result_type = self.function.tree.repr_type(result_type);
        let mir::Type::Slice { space, .. } = self.function.tree.get(result_type) else {
            return Err(self.type_mismatch("slice result type", format!("{result_type:?}")));
        };

        match space {
            mir::Space::Local | mir::Space::Shared => Ok(*space),
            _ => Err(self.invalid_pointer_type(format!("{space:?}"))),
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

        match space {
            mir::Space::Local => Ok(Op::FreeHeap),
            mir::Space::Shared => Ok(Op::FreeSharedHeap),
            _ => Err(self.invalid_pointer_type(format!("{space:?}"))),
        }
    }
}

impl BlockLowerer<'_> {
    /// Build one allocation plan for a concrete MIR type.
    fn allocation_plan(
        &self,
        pool: &mut Pool<'_, '_>,
        space: mir::Space,
        layout: &StorageLayout,
        drop: Option<DropId>,
    ) -> LinkResult<(AllocationPlan, AllocationClass)> {
        // resolve the allocation class from the destination space
        let is_noscan = !layout.trace_map.has_heap_reference();
        let trace_map = pool.trace_map(&layout.trace_map)?;
        let trace_id = if is_noscan { None } else { Some(trace_map) };
        let shape = AllocationShape::new(
            layout.byte_len(),
            layout.alignment(),
            trace_id,
            layout.trace_map.clone(),
        );
        let shape = match drop {
            Some(drop) => shape
                .with_drop(drop)
                .map_err(|error| self.invalid_input(error.to_string()))?,
            None => shape,
        };
        let allocation = match space {
            mir::Space::Local => self.function.heap_options.allocation_plan(&shape),
            mir::Space::Shared => self.function.shared_heap_options.allocation_plan(&shape),
            _ => {
                return Err(self.invalid_pointer_type(format!("{space:?}")));
            }
        };
        let class = allocation.class;

        Ok((allocation, class))
    }

    /// Return the drop identity for one managed allocation.
    fn managed_drop(&self, result: mir::TypeId, dropped: mir::TypeId) -> Option<DropId> {
        let result = self.function.type_linker().storage_type(result);
        let is_managed =
            self.function.tree.get(result).reference_kind() == Some(mir::ReferenceKind::Managed);

        if is_managed {
            self.function.program.program().drop_id(dropped)
        } else {
            None
        }
    }

    /// Select one heap allocation operation from destination and size class.
    fn allocation_op(
        &self,
        space: mir::Space,
        class: AllocationClass,
        is_noscan: bool,
        has_shared_reference: bool,
        initialization: AllocationInitialization,
    ) -> LinkResult<Op> {
        match (space, class.as_small(), is_noscan, initialization) {
            (mir::Space::Local, Some(_), _, AllocationInitialization::Zeroed)
                if has_shared_reference =>
            {
                Ok(Op::AllocateHeapSmallSharedEdgeZeroed)
            }
            (mir::Space::Local, Some(_), _, AllocationInitialization::Uninit)
                if has_shared_reference =>
            {
                Ok(Op::AllocateHeapSmallSharedEdgeUninit)
            }
            (mir::Space::Local, Some(_), true, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateHeapSmallNoscanZeroed)
            }
            (mir::Space::Local, Some(_), true, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateHeapSmallNoscanUninit)
            }
            (mir::Space::Local, Some(_), false, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateHeapSmallScanZeroed)
            }
            (mir::Space::Local, Some(_), false, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateHeapSmallScanUninit)
            }
            (mir::Space::Local, None, _, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateHeapZeroed)
            }
            (mir::Space::Local, None, _, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateHeapUninit)
            }
            (mir::Space::Shared, Some(_), _, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateSharedHeapSmallZeroed)
            }
            (mir::Space::Shared, Some(_), _, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateSharedHeapSmallUninit)
            }
            (mir::Space::Shared, None, _, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateSharedHeapZeroed)
            }
            (mir::Space::Shared, None, _, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateSharedHeapUninit)
            }
            _ => Err(self.invalid_pointer_type(format!("{space:?}"))),
        }
    }

    /// Select one fallible heap allocation operation from destination space.
    fn allocation_branch_op(
        &self,
        space: mir::Space,
        initialization: AllocationInitialization,
    ) -> LinkResult<Op> {
        match (space, initialization) {
            (mir::Space::Local, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateHeapZeroedBranch)
            }
            (mir::Space::Local, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateHeapUninitBranch)
            }
            (mir::Space::Shared, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateSharedHeapZeroedBranch)
            }
            (mir::Space::Shared, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateSharedHeapUninitBranch)
            }
            _ => Err(self.invalid_pointer_type(format!("{space:?}"))),
        }
    }

    /// Select one fallible slice allocation operation from destination space.
    fn slice_allocation_branch_op(
        &self,
        space: mir::Space,
        initialization: AllocationInitialization,
    ) -> LinkResult<Op> {
        match (space, initialization) {
            (mir::Space::Local, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateSliceZeroedBranch)
            }
            (mir::Space::Local, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateSliceUninitBranch)
            }
            (mir::Space::Shared, AllocationInitialization::Zeroed) => {
                Ok(Op::AllocateSharedSliceZeroedBranch)
            }
            (mir::Space::Shared, AllocationInitialization::Uninit) => {
                Ok(Op::AllocateSharedSliceUninitBranch)
            }
            _ => Err(self.invalid_pointer_type(format!("{space:?}"))),
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
