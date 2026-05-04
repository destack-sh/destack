use destack_mir as mir;

use crate::program::{Instruction, Op};

use super::access::slice_element_access;
use super::lower::BlockLowerer;
use super::op::{
    select_compare_branch_op, select_element_load_op, select_element_store_op,
    select_field_load_op, select_field_store_op,
};
use super::pool::Pool;
use super::value::{pointer_class_for_value, reference_meta_for_value};

impl<'a> BlockLowerer<'a> {
    /// Try to fuse address formation with a following load or store.
    pub(super) fn try_fuse_addr_access(
        &self,
        inst: &mir::Instruction,
        next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
        pool: &mut Pool<'_>,
    ) -> Option<(Instruction, usize)> {
        let next_inst_id = next_inst_id?;
        let next_inst = self.tree.get(next_inst_id);

        match inst {
            mir::Instruction::FieldAddr {
                destination,
                aggregate: base,
                index,
                ..
            } => self.try_fuse_field_access(*destination, *base, *index, next_inst, pool),
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => self.try_fuse_element_access(*destination, *array, *index, next_inst, pool),
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.try_fuse_static_access(*destination, *global, next_inst),
            _ => None,
        }
    }

    /// Try to fuse field address formation with a following load or store.
    fn try_fuse_field_access(
        &self,
        destination: mir::ValueReference,
        base: mir::ValueReference,
        index: u32,
        next_inst: &mir::Instruction,
        pool: &mut Pool<'_>,
    ) -> Option<(Instruction, usize)> {
        let destination = destination.value()?;
        let base = base.value()?;

        if !self.is_single_use(destination) {
            return None;
        }

        let field = self.field_access_for_value(base, index).ok()?;

        match next_inst {
            mir::Instruction::Load {
                destination: load_dest,
                pointer,
                ..
            } if pointer.value()? == destination => {
                let op = select_field_load_op(self.value_layout_map(), base, field).ok()?;
                let instruction = if op == Op::LoadFrame {
                    let access = pool.frame_access(field.into());

                    Instruction::new(op, load_dest.value()?.id(), base.id(), access.0, 0)
                } else {
                    let field = pool.field_access(field);

                    Instruction::new(op, load_dest.value()?.id(), base.id(), field.0, 0)
                };

                Some((instruction, 2))
            }
            mir::Instruction::Store { pointer, value } if pointer.value()? == destination => {
                let op = select_field_store_op(self.value_layout_map(), base, field).ok()?;
                let instruction = if op == Op::StoreFrame {
                    let reference = reference_meta_for_value(self.value_layout_map(), destination);
                    let access = pool.frame_access(field.into());

                    Instruction::new(
                        op,
                        base.id(),
                        value.value()?.id(),
                        access.0,
                        reference.bits() as u32,
                    )
                } else {
                    let field = pool.field_access(field);
                    let reference = reference_meta_for_value(self.value_layout_map(), destination);

                    Instruction::new(
                        op,
                        base.id(),
                        value.value()?.id(),
                        reference.bits() as u32,
                        field.0,
                    )
                };

                Some((instruction, 2))
            }
            _ => None,
        }
    }

    /// Try to fuse element address formation with a following load or store.
    fn try_fuse_element_access(
        &self,
        destination: mir::ValueReference,
        array: mir::ValueReference,
        index: mir::ValueReference,
        next_inst: &mir::Instruction,
        pool: &mut Pool<'_>,
    ) -> Option<(Instruction, usize)> {
        let destination = destination.value()?;
        let array = array.value()?;

        if !self.is_single_use(destination) {
            return None;
        }

        let pointer_class = pointer_class_for_value(self.value_layout_map(), array);
        let projection_type = self.projection_type_for_value(array).ok().flatten();
        let is_slice = projection_type
            .and_then(|projection_type| {
                slice_element_access(self.tree, self.layouts(), projection_type, pointer_class)
            })
            .is_some();
        if is_slice {
            return None;
        }

        let array_length = self.array_length_for_value(array).ok()?;
        let mut element = self.element_access_for_value(array).ok()?;
        let reference = reference_meta_for_value(self.value_layout_map(), destination);
        element.reference = reference;

        match next_inst {
            mir::Instruction::Load {
                destination: load_dest,
                pointer,
                ..
            } if pointer.value()? == destination => {
                let op = select_element_load_op(self.value_layout_map(), array, element).ok()?;
                let instruction = if op == Op::LoadFrame {
                    let access = pool.frame_access(element.into_frame_access(0, array_length));

                    Instruction::new(
                        Op::LoadFrameElement,
                        load_dest.value()?.id(),
                        array.id(),
                        index.value()?.id(),
                        access.0,
                    )
                } else {
                    let element = pool.element_access(element);

                    Instruction::new(
                        op,
                        load_dest.value()?.id(),
                        array.id(),
                        index.value()?.id(),
                        element.0,
                    )
                };

                Some((instruction, 2))
            }
            mir::Instruction::Store { pointer, value } if pointer.value()? == destination => {
                let op = select_element_store_op(self.value_layout_map(), array, element).ok()?;
                let instruction = if op == Op::StoreFrame {
                    let access = pool.frame_access(element.into_frame_access(0, array_length));

                    Instruction::new(
                        Op::StoreFrameElement,
                        array.id(),
                        index.value()?.id(),
                        value.value()?.id(),
                        access.0,
                    )
                } else {
                    let element = pool.element_access(element);

                    Instruction::new(
                        op,
                        array.id(),
                        index.value()?.id(),
                        value.value()?.id(),
                        element.0,
                    )
                };

                Some((instruction, 2))
            }
            _ => None,
        }
    }

    /// Try to fuse static address formation with a following word load or store.
    fn try_fuse_static_access(
        &self,
        destination: mir::ValueReference,
        global: mir::GlobalReference,
        next_inst: &mir::Instruction,
    ) -> Option<(Instruction, usize)> {
        let destination = destination.value()?;
        let global = global.global()?;

        if !self.is_single_use(destination) {
            return None;
        }

        let global_id: mir::LocalNodeId<mir::Global> = mir::LocalNodeId::new(global.id);
        let global_def = self.tree.get(global_id);
        let global_type = global_def.ty.ty()?;
        let is_word = self.layout_for_type(global_type).ok()?.is_word();
        let reference = reference_meta_for_value(self.value_layout_map(), destination);

        match next_inst {
            mir::Instruction::Load {
                destination: load_dest,
                pointer,
                ..
            } if pointer.value()? == destination && is_word => Some((
                Instruction::new(Op::LoadStaticId, load_dest.value()?.id(), global.id, 0, 0),
                2,
            )),
            mir::Instruction::Store { pointer, value }
                if pointer.value()? == destination && is_word =>
            {
                Some((
                    Instruction::new(
                        Op::StoreStaticId,
                        global.id,
                        value.value()?.id(),
                        reference.bits() as u32,
                        0,
                    ),
                    2,
                ))
            }
            _ => None,
        }
    }

    /// Try to fuse compare and branch.
    pub(super) fn try_fuse_compare_branch(
        &self,
        block: &mir::Block,
        instructions: &mut Vec<Instruction>,
        pool: &mut Pool<'_>,
    ) -> Option<Instruction> {
        let terminator = self.tree.get(block.terminator);
        let mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } = terminator
        else {
            return None;
        };

        let condition = condition.value()?;
        if self.value_use_count(condition) != Some(1) {
            return None;
        }

        let last_inst_id = block.instructions.last()?;
        let last_inst = self.tree.get(*last_inst_id);
        let mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } = last_inst
        else {
            return None;
        };
        if !operator.is_comparison() {
            return None;
        }

        let destination = destination.value()?;
        let left = left.value()?;
        let right = right.value()?;
        if destination != condition {
            return None;
        }

        let left_type = self.value_type_for_value(left).ok()?;
        if !self.layout_for_type(left_type).ok()?.is_word() {
            return None;
        }

        instructions.pop();

        let then_target_block = then_target.block.block()?;
        let else_target_block = else_target.block.block()?;
        let then_arguments = then_target
            .arguments
            .iter()
            .map(|argument| argument.value())
            .collect::<Option<Vec<_>>>()?;
        let else_arguments = else_target
            .arguments
            .iter()
            .map(|argument| argument.value())
            .collect::<Option<Vec<_>>>()?;
        let then_index = self.block_index_by_id[&then_target_block];
        let else_index = self.block_index_by_id[&else_target_block];
        let then_parameters = self
            .block_parameter
            .get(then_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let else_parameters = self
            .block_parameter
            .get(else_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let then_moves = pool.move_range(then_parameters, &then_arguments).ok()?;
        let else_moves = pool.move_range(else_parameters, &else_arguments).ok()?;
        let then_edge = pool.edge(then_index as u32, then_moves);
        let else_edge = pool.edge(else_index as u32, else_moves);

        let left_layout = self.value_layout_map().get(left);
        let op = select_compare_branch_op(*operator, left_layout)?;

        Some(Instruction::new(
            op,
            left.id(),
            right.id(),
            then_edge.0,
            else_edge.0,
        ))
    }
}
