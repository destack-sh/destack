use crate::build::FunctionBuilder;
use crate::{
    Block, Call, CheckConstraint, CheckTarget, Function, InterfaceSlotId, LocalNodeId, Terminator,
    TrapKind, Type, Value, VtableSlotId,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Return from the function.
    pub fn return_(&mut self, return_value: Option<Value>) {
        let block = self.current_block();
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Return {
            value: return_value,
        };
    }

    /// Unconditional jump to another block.
    pub fn jump(&mut self, target_block: LocalNodeId<Block>) {
        let block = self.current_block();
        self.add_predecessor(block, target_block);
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Jump {
            target: target_block,
            arguments: Vec::new(),
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
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Branch {
            condition: condition_value,
            then_target: then_block,
            then_arguments: Vec::new(),
            else_target: else_block,
            else_arguments: Vec::new(),
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
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Check {
            constraint,
            success: CheckTarget {
                target: success_block,
                arguments: Vec::new(),
            },
            failure: CheckTarget {
                target: failure_block,
                arguments: Vec::new(),
            },
        };
    }

    /// Throw a managed exception object.
    pub fn throw(&mut self, value: Value) {
        let block = self.current_block();
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Throw { value };
    }

    /// Abort execution immediately.
    pub fn trap_abort(&mut self) {
        let block = self.current_block();
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Trap {
            kind: TrapKind::Abort,
            payload: None,
        };
    }

    /// Panic with a runtime payload.
    pub fn trap_panic(&mut self, payload: Value) {
        let block = self.current_block();
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Trap {
            kind: TrapKind::Panic,
            payload: Some(payload),
        };
    }

    /// Call a function with explicit success and exception continuations.
    pub fn call_branch(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        normal_block: LocalNodeId<Block>,
        normal_arguments: Vec<Value>,
        unwind_block: LocalNodeId<Block>,
        unwind_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, normal_block);
        self.add_predecessor(block_id, unwind_block);

        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::Invoke {
            function,
            call: Call::new(argument_values, signature),
            normal_target: normal_block,
            normal_arguments,
            unwind_target: unwind_block,
            unwind_arguments,
        };
    }

    /// Call through a function pointer with explicit success and exception continuations.
    pub fn call_indirect_branch(
        &mut self,
        callee: Value,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        normal_block: LocalNodeId<Block>,
        normal_arguments: Vec<Value>,
        unwind_block: LocalNodeId<Block>,
        unwind_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, normal_block);
        self.add_predecessor(block_id, unwind_block);

        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::InvokeIndirect {
            callee,
            call: Call::new(argument_values, signature),
            normal_target: normal_block,
            normal_arguments,
            unwind_target: unwind_block,
            unwind_arguments,
        };
    }

    /// Call a virtual method with explicit success and exception continuations.
    pub fn call_virtual_branch(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: VtableSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        normal_block: LocalNodeId<Block>,
        normal_arguments: Vec<Value>,
        unwind_block: LocalNodeId<Block>,
        unwind_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, normal_block);
        self.add_predecessor(block_id, unwind_block);

        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::InvokeVirtual {
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(argument_values, signature),
            normal_target: normal_block,
            normal_arguments,
            unwind_target: unwind_block,
            unwind_arguments,
        };
    }

    /// Call an interface method with explicit success and exception continuations.
    pub fn call_interface_branch(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: InterfaceSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        normal_block: LocalNodeId<Block>,
        normal_arguments: Vec<Value>,
        unwind_block: LocalNodeId<Block>,
        unwind_arguments: Vec<Value>,
    ) {
        let block_id = self.current_block();
        self.add_predecessor(block_id, normal_block);
        self.add_predecessor(block_id, unwind_block);

        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::InvokeInterface {
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(argument_values, signature),
            normal_target: normal_block,
            normal_arguments,
            unwind_target: unwind_block,
            unwind_arguments,
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
        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::TailCall {
            function,
            call: Call::new(argument_values, signature),
        };
    }

    /// Tail call through a virtual dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_virtual(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: VtableSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block_id = self.current_block();
        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::TailCallVirtual {
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(argument_values, signature),
        };
    }

    /// Tail call through an interface dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_interface(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: InterfaceSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block_id = self.current_block();
        let block = self.tree.get_mut(block_id);
        block.terminator = Terminator::TailCallInterface {
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(argument_values, signature),
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
        let block = self.tree.get_mut(block);
        block.terminator = Terminator::TailCallIndirect {
            callee,
            call: Call::new(argument_values, signature),
        };
    }
}
