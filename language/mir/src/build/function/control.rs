use crate::build::FunctionBuilder;
use crate::{
    Block, BlockTarget, Call, CallSite, CheckConstraint, DispatchSlot, Function, LocalNodeId,
    Terminator, TrapKind, Type, Value,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Return from the function.
    pub fn return_(&mut self, return_value: Option<Value>) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Return {
            value: return_value.map(Into::into),
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
            target: BlockTarget::new(target_block.into(), arguments),
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
            condition: condition_value.into(),
            then_target: BlockTarget::new(then_block.into(), then_arguments),
            else_target: BlockTarget::new(else_block.into(), else_arguments),
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
            success: BlockTarget::new(success_block.into(), success_arguments),
            failure: BlockTarget::new(failure_block.into(), failure_arguments),
        };
    }

    /// Abort execution immediately.
    pub fn trap_abort(&mut self) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Trap {
            kind: TrapKind::Abort,
            payload: None,
        };
    }

    /// Panic with a language payload.
    pub fn panic(&mut self, payload: Option<Value>) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Panic {
            payload: payload.map(Into::into),
        };
    }

    /// Continue the active unwind after cleanup.
    pub fn resume_unwind(&mut self) {
        let block = self.current_block();
        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::UnwindResume;
    }

    /// Call a function with an explicit continuation.
    pub fn call_branch(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        target_block: LocalNodeId<Block>,
        target_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, target_block);
        let arguments = self.tree.add_values(&argument_values);
        let target_arguments = self.tree.add_values(&target_arguments);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::Call {
            function: function.into(),
            call: Call::new(arguments, signature.into()),
            target: BlockTarget::new(target_block.into(), target_arguments),
            unwind: None,
        };
    }

    /// Call through a function pointer with an explicit continuation.
    pub fn call_indirect_branch(
        &mut self,
        callee: Value,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        target_block: LocalNodeId<Block>,
        target_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, target_block);
        let arguments = self.tree.add_values(&argument_values);
        let target_arguments = self.tree.add_values(&target_arguments);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::CallIndirect {
            callee: callee.into(),
            call: Call::new(arguments, signature.into()),
            target: BlockTarget::new(target_block.into(), target_arguments),
            unwind: None,
        };
    }

    /// Call a virtual method with an explicit continuation.
    pub fn call_virtual_branch(
        &mut self,
        receiver: Value,
        class: LocalNodeId<Type>,
        slot: DispatchSlot,
        target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        target_block: LocalNodeId<Block>,
        target_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, target_block);
        let arguments = self.tree.add_values(&argument_values);
        let target_arguments = self.tree.add_values(&target_arguments);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::CallVirtual {
            receiver: receiver.into(),
            class: class.into(),
            slot,
            call: Call::new(arguments, signature.into()),
            target: BlockTarget::new(target_block.into(), target_arguments),
            unwind: None,
        };
        if let Some(target) = target {
            self.tree
                .metadata
                .functions
                .call_mut(CallSite::Terminator(block_id))
                .target = Some(target);
        }
    }

    /// Call a dynamic method with an explicit continuation.
    pub fn call_dynamic_branch(
        &mut self,
        receiver: Value,
        constraint: LocalNodeId<Type>,
        slot: DispatchSlot,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        target_block: LocalNodeId<Block>,
        target_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, target_block);
        let arguments = self.tree.add_values(&argument_values);
        let target_arguments = self.tree.add_values(&target_arguments);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::CallDynamic {
            receiver: receiver.into(),
            constraint: constraint.into(),
            slot,
            call: Call::new(arguments, signature.into()),
            target: BlockTarget::new(target_block.into(), target_arguments),
            unwind: None,
        };
    }

    /// Tail call to a function (does not return to this function).
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block_id = self.current_block();
        let arguments = self.tree.add_values(&argument_values);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::TailCall {
            function: function.into(),
            call: Call::new(arguments, signature.into()),
        };
    }

    /// Tail call through a virtual dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_virtual(
        &mut self,
        receiver: Value,
        class: LocalNodeId<Type>,
        slot: DispatchSlot,
        target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block_id = self.current_block();
        let arguments = self.tree.add_values(&argument_values);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::TailCallVirtual {
            receiver: receiver.into(),
            class: class.into(),
            slot,
            call: Call::new(arguments, signature.into()),
        };
        if let Some(target) = target {
            self.tree
                .metadata
                .functions
                .call_mut(CallSite::Terminator(block_id))
                .target = Some(target);
        }
    }

    /// Tail call through a dynamic dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_dynamic(
        &mut self,
        receiver: Value,
        constraint: LocalNodeId<Type>,
        slot: DispatchSlot,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block_id = self.current_block();
        let arguments = self.tree.add_values(&argument_values);

        let terminator_id = self.tree.get(block_id).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::TailCallDynamic {
            receiver: receiver.into(),
            constraint: constraint.into(),
            slot,
            call: Call::new(arguments, signature.into()),
        };
    }

    /// Tail call through a function pointer (does not return to this function).
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_indirect(
        &mut self,
        callee: Value,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block = self.current_block();
        let arguments = self.tree.add_values(&argument_values);

        let terminator_id = self.tree.get(block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        *terminator = Terminator::TailCallIndirect {
            callee: callee.into(),
            call: Call::new(arguments, signature.into()),
        };
    }
}
