use std::collections::HashMap;

use destack_mir as mir;

use crate::elaborate::ElaborateState;

use super::plan::{DropPlan, DropPoint};

impl ElaborateState<'_> {
    /// Insert explicit last use drops for verified owned values.
    pub(in crate::elaborate) fn insert_drops(&mut self) {
        let functions = self
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        // build and insert drops for each function
        for function_id in functions {
            if self.is_destructor(function_id) {
                continue;
            }

            let function = self.tree.get(function_id).clone();
            if function.entry().is_none() {
                continue;
            }

            let hook_receiver = self.hook_receiver(function_id);
            let drops = DropPlan::build(&function, &self.tree, hook_receiver);
            self.insert_function_drops(function_id, drops);
        }
    }

    /// Return whether one function is a generated destructor.
    fn is_destructor(&self, function_id: mir::LocalNodeId<mir::Function>) -> bool {
        self.drops
            .destructors
            .values()
            .any(|function| *function == function_id)
    }

    /// Return the value borrowed by a user-authored drop hook.
    fn hook_receiver(&self, function_id: mir::LocalNodeId<mir::Function>) -> Option<mir::Value> {
        self.drops
            .hooks
            .values()
            .any(|function| *function == function_id)
            .then_some(mir::Value::new(0))
    }

    /// Insert drops into one function body.
    fn insert_function_drops(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        drops_by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<DropPoint>>,
    ) {
        for (block_id, drops) in drops_by_block {
            let mut inserted = 0;
            for drop in drops {
                let instructions = self.instructions_for_drop(function_id, drop.value);
                let count = instructions.len();
                if count == 0 {
                    continue;
                }

                let instruction_ids = instructions
                    .into_iter()
                    .map(|instruction| self.tree.insert(instruction))
                    .collect::<Vec<_>>();

                let block = self.tree.get_mut(block_id);
                block.instructions.splice(
                    drop.index + inserted..drop.index + inserted,
                    instruction_ids,
                );
                inserted += count;
            }
        }
    }

    /// Build executable drop instructions for one complete value.
    fn instructions_for_drop(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
    ) -> Vec<mir::Instruction> {
        let Some(ty) = self.tree.get(function_id).value_type(value) else {
            return Vec::new();
        };

        self.instructions_for_value(function_id, value, ty)
    }

    /// Build drop instructions for a concrete SSA value.
    fn instructions_for_value(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Vec<mir::Instruction> {
        let has_destructor = self.drops.destructor(ty).is_some();
        let mut instructions = self.instructions_for_contents(function_id, value, ty);

        // release trivial owned storage not already handled by a destructor
        if self.tree.get(ty).is_unique_storage() && !has_destructor {
            instructions.push(mir::Instruction::Free { value });
        }

        instructions
    }

    /// Build drop instructions for one value's contents.
    fn instructions_for_contents(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Vec<mir::Instruction> {
        if self.drops.destructor(ty).is_some()
            || matches!(self.tree.get(ty), mir::Type::Dynamic { .. })
        {
            return vec![mir::Instruction::Drop { value }];
        }

        let mir::Type::Reference {
            kind: mir::ReferenceKind::Unique,
            pointee,
            ..
        } = self.tree.get(ty)
        else {
            return Vec::new();
        };

        self.instructions_for_unique_reference_contents(function_id, value, *pointee)
    }

    /// Build drop instructions for one unique reference's pointee.
    fn instructions_for_unique_reference_contents(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> Vec<mir::Instruction> {
        let mut instructions = Vec::new();

        // drop pointee contents before caller releases the allocation
        if self.type_emits_drop_code(pointee) {
            let loaded = self.next_typed_value(function_id, pointee);
            instructions.push(mir::Instruction::Load {
                destination: loaded,
                pointer: value,
                result_type: pointee,
            });
            instructions.extend(self.instructions_for_value(function_id, loaded, pointee));
        }

        instructions
    }

    /// Return whether dropping a value of this type emits MIR.
    fn type_emits_drop_code(&self, ty: mir::LocalNodeId<mir::Type>) -> bool {
        if self.drops.destructor(ty).is_some() {
            return true;
        }
        if self.tree.get(ty).is_unique_storage() {
            return true;
        }

        match self.tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                ..
            } => self.type_emits_drop_code(*pointee),
            mir::Type::Dynamic { .. } => true,
            _ => false,
        }
    }

    /// Allocate one typed SSA value in the rewritten function.
    fn next_typed_value(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        let function = self.tree.get_mut(function_id);
        function.next_typed_value(ty)
    }
}
