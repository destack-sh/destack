use crate::build::FunctionBuilder;
use crate::{Call, Function, Instruction, InterfaceSlotId, LocalNodeId, Type, Value, VtableSlotId};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Call a function.
    pub fn call(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: Some(destination),
            function,
            call: Call::new(arguments, signature),
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a function with no return value.
    pub fn call_void(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: None,
            function,
            call: Call::new(arguments, signature),
        });
    }

    /// Call a virtual method through a vtable slot.
    pub fn call_virtual(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: VtableSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallVirtual {
            destination: Some(destination),
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(arguments, signature),
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a virtual method with no return value.
    pub fn call_virtual_void(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: VtableSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallVirtual {
            destination: None,
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(arguments, signature),
        });
    }

    /// Call an interface method through an itab slot.
    pub fn call_interface(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: InterfaceSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallInterface {
            destination: Some(destination),
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(arguments, signature),
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call an interface method with no return value.
    pub fn call_interface_void(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: InterfaceSlotId,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallInterface {
            destination: None,
            receiver,
            declaring_type,
            slot_id,
            declared_target,
            call: Call::new(arguments, signature),
        });
    }

    /// Load a function pointer value for a function.
    pub fn function_addr(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionAddr {
            destination,
            function,
        });
        self.define_value(destination, signature);
        destination
    }

    /// Construct a callable value for one function and environment.
    pub fn function_bind(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        environment: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionBind {
            destination,
            function,
            environment,
        });
        self.define_value(destination, signature);
        destination
    }

    /// Load the hidden environment pointer for the current function.
    pub fn function_environment(&mut self, environment_type: LocalNodeId<Type>) -> Value {
        // record the hidden environment type on the function metadata
        {
            let function = self.tree.get_mut(self.function_id);
            match function.environment {
                Some(existing) if existing != environment_type => {
                    panic!("mismatched environment types for function.environment");
                }
                Some(_) => {}
                None => {
                    function.environment = Some(environment_type);
                }
            }
        }

        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionEnvironment { destination });
        self.define_value(destination, environment_type);
        destination
    }

    /// Call through a function pointer with an explicit signature type.
    pub fn call_indirect(
        &mut self,
        callee: Value,
        signature: LocalNodeId<Type>,
        args: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: Some(destination),
            callee,
            call: Call::new(arguments, signature),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Call through a function pointer with no return value.
    pub fn call_indirect_void(
        &mut self,
        callee: Value,
        signature: LocalNodeId<Type>,
        args: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: None,
            callee,
            call: Call::new(arguments, signature),
        });
    }
}
