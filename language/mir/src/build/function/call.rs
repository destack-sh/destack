use crate::build::{BuildError, FunctionBuilder};
use crate::{
    Call, CallSite, DispatchSlot, Function, Instruction, LocalNodeId, Type, TypeId, Value,
};

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
        let result_type = self.expect_build(result_type);
        let arguments = self.tree.add_values(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: Some(destination),
            function,
            call: Call::new(arguments, TypeId::from(signature)),
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
        let arguments = self.tree.add_values(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: None,
            function,
            call: Call::new(arguments, TypeId::from(signature)),
        });
    }

    /// Call a virtual method through a virtual dispatch slot.
    pub fn call_virtual(
        &mut self,
        receiver: Value,
        class: LocalNodeId<Type>,
        slot: DispatchSlot,
        target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let result_type = self.expect_build(result_type);
        let arguments = self.tree.add_values(&argument_values);
        let instruction = self.insert_instruction(Instruction::CallVirtual {
            destination: Some(destination),
            receiver,
            class,
            slot,
            call: Call::new(arguments, TypeId::from(signature)),
        });
        if let Some(target) = target {
            self.effects
                .call_mut(CallSite::Instruction(instruction))
                .target = Some(target);
        }
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a virtual method with no return value.
    pub fn call_virtual_void(
        &mut self,
        receiver: Value,
        class: LocalNodeId<Type>,
        slot: DispatchSlot,
        target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_values(&argument_values);
        let instruction = self.insert_instruction(Instruction::CallVirtual {
            destination: None,
            receiver,
            class,
            slot,
            call: Call::new(arguments, TypeId::from(signature)),
        });
        if let Some(target) = target {
            self.effects
                .call_mut(CallSite::Instruction(instruction))
                .target = Some(target);
        }
    }

    /// Call a dynamic function through a dynamic table slot.
    pub fn call_dynamic(
        &mut self,
        receiver: Value,
        constraint: LocalNodeId<Type>,
        slot: DispatchSlot,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let result_type = self.expect_build(result_type);
        let arguments = self.tree.add_values(&argument_values);
        self.insert_instruction(Instruction::CallDynamic {
            destination: Some(destination),
            receiver,
            constraint,
            slot,
            call: Call::new(arguments, TypeId::from(signature)),
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a dynamic function with no return value.
    pub fn call_dynamic_void(
        &mut self,
        receiver: Value,
        constraint: LocalNodeId<Type>,
        slot: DispatchSlot,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_values(&argument_values);
        self.insert_instruction(Instruction::CallDynamic {
            destination: None,
            receiver,
            constraint,
            slot,
            call: Call::new(arguments, TypeId::from(signature)),
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

    /// Construct a function value for one function and environment.
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

    /// Project the function pointer from one function value.
    pub fn function_pointer(&mut self, function: Value, function_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionPointer {
            destination,
            function,
        });
        self.define_value(destination, function_type);
        destination
    }

    /// Project the environment from one function value.
    pub fn function_environment(
        &mut self,
        function: Value,
        environment_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionEnvironment {
            destination,
            function,
        });
        self.define_value(destination, environment_type);
        destination
    }

    /// Load the hidden environment pointer for the current function.
    pub fn function_environment_current(&mut self, environment_type: LocalNodeId<Type>) -> Value {
        // record the hidden environment type on the function tables
        let existing_environment = {
            let requested_environment = TypeId::from(environment_type);
            let function = self.tree.get_mut(self.function_id);
            match &function.environment {
                Some(existing) if existing != &requested_environment => Some(*existing),
                Some(_) => None,
                None => {
                    function.environment = Some(requested_environment);
                    None
                }
            }
        };
        if let Some(existing) = existing_environment {
            self.expect_build::<()>(Err(BuildError::MismatchedFunctionEnvironment {
                existing,
                requested: environment_type,
            }));
        }

        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionEnvironmentCurrent { destination });
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
        let result_type = self.expect_build(result_type);
        let arguments = self.tree.add_values(&args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: Some(destination),
            callee,
            call: Call::new(arguments, TypeId::from(signature)),
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
        let arguments = self.tree.add_values(&args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: None,
            callee,
            call: Call::new(arguments, TypeId::from(signature)),
        });
    }
}
