use destack_mir as mir;

use crate::Word;
use crate::program::{Instruction, Opcode, Operands, ValueRepr};

use super::access::slice_element_access;
use super::lower::BlockLowerer;
use super::opcode::{
    select_compare_branch_const_opcode, select_compare_branch_opcode, select_element_load_opcode,
    select_element_store_opcode, select_field_load_opcode, select_field_store_opcode,
    select_specialized_const_int_opcode, swap_compare_operator,
};
use super::pool::Pool;
use super::repr::{pointer_class_for_value, reference_meta_for_value};

impl<'a> BlockLowerer<'a> {
    /// Try to fuse address formation with a following load or store.
    pub(super) fn try_fuse_addr_access(
        &self,
        inst: &mir::Instruction,
        next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    ) -> Option<(Instruction, usize)> {
        let next_inst_id = next_inst_id?;
        let next_inst = self.tree.get(next_inst_id);

        match inst {
            mir::Instruction::FieldAddr {
                destination,
                aggregate: base,
                index,
                ..
            } => self.try_fuse_field_access(*destination, *base, *index, next_inst),
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => self.try_fuse_element_access(*destination, *array, *index, next_inst),
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
            } if pointer.value()? == destination => Some((
                Instruction {
                    opcode: select_field_load_opcode(self.value_repr_map(), base, field).ok()?,
                    operands: Operands::FieldLoad {
                        dest: load_dest.value()?,
                        base,
                        index,
                        field_count,
                        field,
                    },
                },
                2,
            )),
            mir::Instruction::Store { pointer, value } if pointer.value()? == destination => {
                Some((
                    Instruction {
                        opcode: select_field_store_opcode(self.value_repr_map(), base, field)
                            .ok()?,
                        operands: Operands::FieldStore {
                            base,
                            index,
                            value: value.value()?,
                            reference: reference_meta_for_value(self.value_repr_map(), destination),
                            field_count,
                            field,
                        },
                    },
                    2,
                ))
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
    ) -> Option<(Instruction, usize)> {
        let destination = destination.value()?;
        let array = array.value()?;

        if !self.is_single_use(destination) {
            return None;
        }

        let pointer_class = pointer_class_for_value(self.value_repr_map(), array);
        let indexed_type = self.indexed_type_for_value(array).ok().flatten();
        let is_slice = indexed_type
            .and_then(|pointee_type| {
                slice_element_access(self.tree, self.layouts(), pointee_type, pointer_class)
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
            } if pointer.value()? == destination => Some((
                Instruction {
                    opcode: select_element_load_opcode(self.value_repr_map(), array, element)
                        .ok()?,
                    operands: Operands::ElementLoad {
                        dest: load_dest.value()?,
                        array,
                        index: index.value()?,
                        array_length,
                        element,
                    },
                },
                2,
            )),
            mir::Instruction::Store { pointer, value } if pointer.value()? == destination => {
                Some((
                    Instruction {
                        opcode: select_element_store_opcode(self.value_repr_map(), array, element)
                            .ok()?,
                        operands: Operands::ElementStore {
                            array,
                            index: index.value()?,
                            value: value.value()?,
                            reference: reference_meta_for_value(self.value_repr_map(), destination),
                            array_length,
                            element,
                        },
                    },
                    2,
                ))
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
        let reference = reference_meta_for_value(self.value_repr_map(), destination);

        match next_inst {
            mir::Instruction::Load {
                destination: load_dest,
                pointer,
                ..
            } if pointer.value()? == destination && is_word => Some((
                Instruction {
                    opcode: Opcode::StaticLoad,
                    operands: Operands::StaticLoad {
                        dest: load_dest.value()?,
                        global: global.id,
                    },
                },
                2,
            )),
            mir::Instruction::Store { pointer, value }
                if pointer.value()? == destination && is_word =>
            {
                Some((
                    Instruction {
                        opcode: Opcode::StaticStore,
                        operands: Operands::StaticStore {
                            global: global.id,
                            value: value.value()?,
                            reference,
                        },
                    },
                    2,
                ))
            }
            _ => None,
        }
    }

    /// Try to fuse constant right binary operations.
    pub(super) fn try_fuse_const_binary(
        &self,
        inst: &mir::Instruction,
        next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    ) -> Option<(Instruction, usize)> {
        let next_inst_id = next_inst_id?;

        let mir::Instruction::Const { destination, value } = inst else {
            return None;
        };
        let destination = destination.value()?;
        if !self.is_single_use(destination) {
            return None;
        }

        let next_inst = self.tree.get(next_inst_id);
        let mir::Instruction::Binary {
            destination: binary_destination,
            operator,
            left,
            right,
        } = next_inst
        else {
            return None;
        };

        if right.value()? != destination || operator.is_comparison() {
            return None;
        }

        let left = left.value()?;
        let binary_destination = binary_destination.value()?;
        let Some(ValueRepr::Int { width, signed }) = self.value_repr_map().get(left) else {
            return None;
        };
        if width > Word::BIT_LEN as u16 {
            return None;
        }

        let opcode = select_specialized_const_int_opcode(*operator, signed)?;

        Some((
            Instruction {
                opcode,
                operands: Operands::BinaryConstRightSpecialized {
                    dest: binary_destination,
                    left,
                    right_const: Word::from(value),
                    width: width as u8,
                },
            },
            2,
        ))
    }

    /// Try to fuse compare and branch.
    pub(super) fn try_fuse_compare_branch(
        &self,
        block: &mir::Block,
        instructions: &mut Vec<Instruction>,
        pool: &mut Pool,
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

        let mut right_const = None;
        let mut left_value = left;
        let mut operator = *operator;
        let mut pop_const = false;
        if let Some(prev_inst_id) = block
            .instructions
            .get(block.instructions.len().saturating_sub(2))
        {
            let prev_inst = self.tree.get(*prev_inst_id);
            if let mir::Instruction::Const { destination, value } = prev_inst {
                let destination = destination.value()?;
                if self.value_use_count(destination) == Some(1) {
                    if destination == right {
                        right_const = Some(Word::from(value));
                        pop_const = true;
                    } else if destination == left {
                        right_const = Some(Word::from(value));
                        left_value = right;
                        operator = swap_compare_operator(operator);
                        pop_const = true;
                    }
                }
            }
        }

        instructions.pop();
        if pop_const {
            instructions.pop();
        }

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
        let then_moves = pool.move_range(then_parameters, &then_arguments);
        let else_moves = pool.move_range(else_parameters, &else_arguments);

        if let Some(right_const) = right_const {
            return Some(Instruction {
                opcode: select_compare_branch_const_opcode(operator),
                operands: Operands::CompareAndBranchConst {
                    left: left_value,
                    right_const,
                    operator,
                    then_target: then_index as u32,
                    then_moves,
                    else_target: else_index as u32,
                    else_moves,
                },
            });
        }

        Some(Instruction {
            opcode: select_compare_branch_opcode(operator),
            operands: Operands::CompareAndBranch {
                left,
                right,
                operator,
                then_target: then_index as u32,
                then_moves,
                else_target: else_index as u32,
                else_moves,
            },
        })
    }
}
