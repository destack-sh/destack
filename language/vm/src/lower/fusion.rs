use destack_mir as mir;

use crate::program::Instruction;

use super::frame::cell_offset;
use super::lower::BlockLowerer;
use super::op::select_compare_branch_op;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Try to fuse compare and branch.
    pub(super) fn try_fuse_compare_branch(
        &self,
        block: &mir::Block,
        instructions: &mut Vec<Instruction>,
        pool: &mut Pool<'_, '_>,
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
        if !self.layout_for_type(left_type).ok()?.is_cell() {
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
        let then_parameters = self.block_parameter[then_index].as_slice();
        let else_parameters = self.block_parameter[else_index].as_slice();
        let then_moves = pool.move_range(then_parameters, &then_arguments).ok()?;
        let else_moves = pool.move_range(else_parameters, &else_arguments).ok()?;
        let then_edge = pool.edge(then_index as u32, then_moves);
        let else_edge = pool.edge(else_index as u32, else_moves);

        let left_layout = self.value_shape_map().get(left);
        let op = select_compare_branch_op(*operator, left_layout)?;

        Some(Instruction::new(
            op,
            cell_offset(self, left).ok()?,
            cell_offset(self, right).ok()?,
            then_edge.0,
            else_edge.0,
        ))
    }
}
