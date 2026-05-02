use destack_mir as mir;

use crate::program::{
    CompareAndBranch, ElementLoad, ElementStore, FieldLoad, FieldStore, Instruction, LoadFrame,
    LoadFrameElement, Opcode, StaticLoad, StaticStore, StoreFrame, StoreFrameElement,
};

use super::access::slice_element_access;
use super::lower::BlockLowerer;
use super::opcode::{
    select_compare_branch_opcode, select_element_load_opcode, select_element_store_opcode,
    select_field_load_opcode, select_field_store_opcode,
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

        let field_count = self.field_count_for_value(base).ok()?;
        let field = self.field_access_for_value(base, index).ok()?;

        match next_inst {
            mir::Instruction::Load {
                destination: load_dest,
                pointer,
                ..
            } if pointer.value()? == destination => {
                let opcode = select_field_load_opcode(self.value_layout_map(), base, field).ok()?;
                let instruction = if opcode == Opcode::LoadFrame {
                    Instruction::new(
                        opcode,
                        LoadFrame {
                            dest: load_dest.value()?,
                            base,
                            access: pool.frame_access(field.into()),
                        },
                    )
                } else {
                    Instruction::new(
                        opcode,
                        FieldLoad {
                            dest: load_dest.value()?,
                            base,
                            index,
                            field_count,
                            field: pool.field_access(field),
                        },
                    )
                };

                Some((instruction, 2))
            }
            mir::Instruction::Store { pointer, value } if pointer.value()? == destination => {
                let opcode =
                    select_field_store_opcode(self.value_layout_map(), base, field).ok()?;
                let instruction = if opcode == Opcode::StoreFrame {
                    Instruction::new(
                        opcode,
                        StoreFrame {
                            base,
                            value: value.value()?,
                            reference: reference_meta_for_value(
                                self.value_layout_map(),
                                destination,
                            ),
                            access: pool.frame_access(field.into()),
                        },
                    )
                } else {
                    Instruction::new(
                        opcode,
                        FieldStore {
                            base,
                            index,
                            value: value.value()?,
                            reference: reference_meta_for_value(
                                self.value_layout_map(),
                                destination,
                            ),
                            field_count,
                            field: pool.field_access(field),
                        },
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
        let aggregate_type = self.aggregate_type_for_value(array).ok().flatten();
        let is_slice = aggregate_type
            .and_then(|aggregate_type| {
                slice_element_access(self.tree, self.layouts(), aggregate_type, pointer_class)
            })
            .is_some();
        if is_slice {
            return None;
        }

        let array_length = self.array_length_for_value(array).ok()?;
        let element = self.element_access_for_value(array).ok()?;

        match next_inst {
            mir::Instruction::Load {
                destination: load_dest,
                pointer,
                ..
            } if pointer.value()? == destination => {
                let opcode =
                    select_element_load_opcode(self.value_layout_map(), array, element).ok()?;
                let instruction = if opcode == Opcode::LoadFrame {
                    Instruction::new(
                        Opcode::LoadFrameElement,
                        LoadFrameElement {
                            dest: load_dest.value()?,
                            base: array,
                            index: index.value()?,
                            access: pool.frame_access(element.into_frame_access(0, array_length)),
                        },
                    )
                } else {
                    Instruction::new(
                        opcode,
                        ElementLoad {
                            dest: load_dest.value()?,
                            array,
                            index: index.value()?,
                            array_length,
                            element: pool.element_access(element),
                        },
                    )
                };

                Some((instruction, 2))
            }
            mir::Instruction::Store { pointer, value } if pointer.value()? == destination => {
                let opcode =
                    select_element_store_opcode(self.value_layout_map(), array, element).ok()?;
                let instruction = if opcode == Opcode::StoreFrame {
                    Instruction::new(
                        Opcode::StoreFrameElement,
                        StoreFrameElement {
                            base: array,
                            index: index.value()?,
                            value: value.value()?,
                            reference: reference_meta_for_value(
                                self.value_layout_map(),
                                destination,
                            ),
                            access: pool.frame_access(element.into_frame_access(0, array_length)),
                        },
                    )
                } else {
                    Instruction::new(
                        opcode,
                        ElementStore {
                            array,
                            index: index.value()?,
                            value: value.value()?,
                            reference: reference_meta_for_value(
                                self.value_layout_map(),
                                destination,
                            ),
                            array_length,
                            element: pool.element_access(element),
                        },
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
                Instruction::new(
                    Opcode::LoadStaticId,
                    StaticLoad {
                        dest: load_dest.value()?,
                        global: global.id,
                    },
                ),
                2,
            )),
            mir::Instruction::Store { pointer, value }
                if pointer.value()? == destination && is_word =>
            {
                Some((
                    Instruction::new(
                        Opcode::StoreStaticId,
                        StaticStore {
                            global: global.id,
                            value: value.value()?,
                            reference,
                        },
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

        let left_layout = self.value_layout_map().get(left);
        let opcode = select_compare_branch_opcode(*operator, left_layout)?;

        Some(Instruction::new(
            opcode,
            CompareAndBranch {
                left,
                right,
                then_target: then_index as u32,
                then_moves,
                else_target: else_index as u32,
                else_moves,
            },
        ))
    }
}
