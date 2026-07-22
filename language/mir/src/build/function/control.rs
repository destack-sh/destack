use crate::build::FunctionBuilder;
use crate::{
    Block, BlockTarget, Call, Callee, CheckConstraint, LocalNodeId, SwitchCase, Terminator, TypeId,
    Value,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Return from the function.
    pub fn return_(&mut self, return_value: Option<Value>) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Return {
            value: return_value,
        };
    }

    /// Unconditional jump to another block.
    pub fn jump(&mut self, target_block: LocalNodeId<Block>) {
        let block = self.current_block();
        self.add_predecessor(block, target_block);
        let arguments = self.tree.add_values(&[]);

        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Jump {
            target: BlockTarget::new(target_block, arguments),
        };
    }

    /// Conditional branch.
    pub fn branch(
        &mut self,
        condition_value: Value,
        then_block: LocalNodeId<Block>,
        else_block: LocalNodeId<Block>,
    ) {
        let block = self.current_block();
        self.add_predecessor(block, then_block);
        self.add_predecessor(block, else_block);
        let then_arguments = self.tree.add_values(&[]);
        let else_arguments = self.tree.add_values(&[]);

        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Branch {
            condition: condition_value,
            then_target: BlockTarget::new(then_block, then_arguments),
            else_target: BlockTarget::new(else_block, else_arguments),
        };
    }

    /// Switch on an integer value.
    pub fn switch(
        &mut self,
        value: Value,
        default_block: LocalNodeId<Block>,
        cases: Vec<(i128, LocalNodeId<Block>)>,
    ) {
        let block = self.current_block();
        self.add_predecessor(block, default_block);
        let default_arguments = self.tree.add_values(&[]);
        let cases = cases
            .into_iter()
            .map(|(value, target_block)| {
                self.add_predecessor(block, target_block);
                let target_arguments = self.tree.add_values(&[]);

                SwitchCase {
                    value,
                    target: BlockTarget::new(target_block, target_arguments),
                }
            })
            .collect::<Vec<_>>();
        let cases = self.tree.add_switch_cases(&cases);

        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Switch {
            value,
            default: BlockTarget::new(default_block, default_arguments),
            cases,
        };
    }

    /// Switch on the logical case of a variant value.
    pub fn variant_switch(
        &mut self,
        value: Value,
        default_block: Option<LocalNodeId<Block>>,
        cases: Vec<(u32, LocalNodeId<Block>)>,
    ) {
        let block = self.current_block();
        let default = default_block.map(|default_block| {
            self.add_predecessor(block, default_block);
            let default_arguments = self.tree.add_values(&[]);

            BlockTarget::new(default_block, default_arguments)
        });
        let cases = cases
            .into_iter()
            .map(|(case, target_block)| {
                self.add_predecessor(block, target_block);
                let target_arguments = self.tree.add_values(&[]);

                SwitchCase {
                    value: case as i128,
                    target: BlockTarget::new(target_block, target_arguments),
                }
            })
            .collect::<Vec<_>>();
        let cases = self.tree.add_switch_cases(&cases);

        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::VariantSwitch {
            value,
            default,
            cases,
        };
    }

    /// Conditional check with explicit success and failure edges.
    pub fn check(
        &mut self,
        constraint: CheckConstraint,
        success_block: LocalNodeId<Block>,
        failure_block: LocalNodeId<Block>,
    ) {
        let block = self.current_block();
        self.add_predecessor(block, success_block);
        self.add_predecessor(block, failure_block);
        let success_arguments = self.tree.add_values(&[]);
        let failure_arguments = self.tree.add_values(&[]);

        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Check {
            constraint,
            success: BlockTarget::new(success_block, success_arguments),
            failure: BlockTarget::new(failure_block, failure_arguments),
        };
    }

    /// Abort execution immediately.
    pub fn abort(&mut self, payload: Option<Value>) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Abort { payload };
    }

    /// Panic with a language payload.
    pub fn panic(&mut self, payload: Option<Value>) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Panic { payload };
    }

    /// Mark the current block's end as unreachable.
    pub fn unreachable(&mut self) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Unreachable;
    }

    /// Continue the active unwind after cleanup.
    pub fn resume_unwind(&mut self) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::UnwindResume;
    }

    /// Invoke one call with normal and unwind continuations.
    pub fn invoke(
        &mut self,
        callee: Callee,
        signature: TypeId,
        arguments: Vec<Value>,
        target_block: LocalNodeId<Block>,
        target_arguments: Vec<Value>,
        unwind_block: LocalNodeId<Block>,
        unwind_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, target_block);
        self.add_predecessor(block_id, unwind_block);
        let arguments = self.tree.add_values(&arguments);
        let target_arguments = self.tree.add_values(&target_arguments);
        let unwind_arguments = self.tree.add_values(&unwind_arguments);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Invoke {
            call: Call::new(callee, arguments, signature),
            target: BlockTarget::new(target_block, target_arguments),
            unwind: BlockTarget::new(unwind_block, unwind_arguments),
        };
    }

    /// Tail call one callable target.
    pub fn tail_call(&mut self, callee: Callee, signature: TypeId, arguments: Vec<Value>) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let arguments = self.tree.add_values(&arguments);

        *self.tree.get_mut(terminator_id) = Terminator::TailCall {
            call: Call::new(callee, arguments, signature),
        };
    }
}
