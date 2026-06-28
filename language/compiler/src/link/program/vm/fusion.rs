use destack_mir as mir;

use destack_program::vm::Instruction;

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
        let terminator = self.function.tree.get(block.terminator);
        let mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } = terminator
        else {
            return None;
        };

        if self.value_use_count(*condition) != Some(1) {
            return None;
        }

        let last_inst_id = block.instructions.last()?;
        let last_inst = self.function.tree.get(*last_inst_id);
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

        if destination != condition {
            return None;
        }

        let left_type = self.value_type_for_value(*left).ok()?;
        if !self.layout_for_type(left_type).ok()?.is_cell() {
            return None;
        }

        instructions.pop();

        let then_target_block = then_target.block;
        let else_target_block = else_target.block;
        let then_arguments = self.function.target_arguments(then_target);
        let else_arguments = self.function.target_arguments(else_target);
        let then_index = self.function.block_index_by_id[&then_target_block];
        let else_index = self.function.block_index_by_id[&else_target_block];
        let then_parameters = self.function.block_parameters[then_index].as_slice();
        let else_parameters = self.function.block_parameters[else_index].as_slice();
        let then_moves = pool.move_range(then_parameters, then_arguments).ok()?;
        let else_moves = pool.move_range(else_parameters, else_arguments).ok()?;
        let then_edge = pool.edge(then_index as u32, then_moves);
        let else_edge = pool.edge(else_index as u32, else_moves);

        let left_layout = self.operand_map().get(*left);
        let op = select_compare_branch_op(*operator, left_layout)?;

        Some(Instruction::new(
            op,
            self.cell_offset(*left).ok()?,
            self.cell_offset(*right).ok()?,
            then_edge.0,
            else_edge.0,
        ))
    }
}
