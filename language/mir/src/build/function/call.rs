use crate::build::FunctionBuilder;
use crate::{
    Call, CallSite, DispatchSlot, Function, FunctionReference, Instruction, LocalNodeId, Type,
    TypeReference, Value, ValueReference,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Store one call argument list in the external argument buffer.
    fn add_call_arguments(&mut self, values: Vec<Value>) -> crate::ArgumentSlice {
        let values = values
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();

        self.tree.add_arguments(&values)
    }

    /// Call a function.
    pub fn call(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.add_call_arguments(argument_values);
        self.insert_instruction(Instruction::Call {
            destination: Some(destination.into()),
            function: FunctionReference::Function(function),
            call: Call::new(arguments, TypeReference::Type(signature)),
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
        let arguments = self.add_call_arguments(argument_values);
        self.insert_instruction(Instruction::Call {
            destination: None,
            function: FunctionReference::Function(function),
            call: Call::new(arguments, TypeReference::Type(signature)),
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
        let arguments = self.add_call_arguments(argument_values);
        let instruction = self.insert_instruction(Instruction::CallVirtual {
            destination: Some(destination.into()),
            receiver: receiver.into(),
            class: class.into(),
            slot,
            call: Call::new(arguments, TypeReference::Type(signature)),
        });
        if let Some(target) = target {
            self.tree
                .metadata
                .functions
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
        let arguments = self.add_call_arguments(argument_values);
        let instruction = self.insert_instruction(Instruction::CallVirtual {
            destination: None,
            receiver: receiver.into(),
            class: class.into(),
            slot,
            call: Call::new(arguments, TypeReference::Type(signature)),
        });
        if let Some(target) = target {
            self.tree
                .metadata
                .functions
                .call_mut(CallSite::Instruction(instruction))
                .target = Some(target);
        }
    }

    /// Call a dynamic method through a dynamic table slot.
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
        let arguments = self.add_call_arguments(argument_values);
        self.insert_instruction(Instruction::CallDynamic {
            destination: Some(destination.into()),
            receiver: receiver.into(),
            constraint: constraint.into(),
            slot,
            call: Call::new(arguments, TypeReference::Type(signature)),
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a dynamic method with no return value.
    pub fn call_dynamic_void(
        &mut self,
        receiver: Value,
        constraint: LocalNodeId<Type>,
        slot: DispatchSlot,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.add_call_arguments(argument_values);
        self.insert_instruction(Instruction::CallDynamic {
            destination: None,
            receiver: receiver.into(),
            constraint: constraint.into(),
            slot,
            call: Call::new(arguments, TypeReference::Type(signature)),
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
            destination: destination.into(),
            function: function.into(),
        });
        self.define_value(destination, signature);
        destination
    }

    /// Construct a closure value for one function and environment.
    pub fn closure_bind(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        environment: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ClosureBind {
            destination: destination.into(),
            function: function.into(),
            environment: environment.into(),
        });
        self.define_value(destination, signature);
        destination
    }

    /// Load the hidden environment pointer for the current function.
    pub fn closure_environment(&mut self, environment_type: LocalNodeId<Type>) -> Value {
        // record the hidden environment type on the function metadata
        {
            let function = self.tree.get_mut(self.function_id);
            match function.environment {
                Some(existing) if existing != environment_type.into() => {
                    panic!("mismatched environment types for closure.environment");
                }
                Some(_) => {}
                None => {
                    function.environment = Some(environment_type.into());
                }
            }
        }

        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ClosureEnvironment {
            destination: destination.into(),
        });
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
        let arguments = self.add_call_arguments(args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: Some(destination.into()),
            callee: callee.into(),
            call: Call::new(arguments, TypeReference::Type(signature)),
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
        let arguments = self.add_call_arguments(args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: None,
            callee: callee.into(),
            call: Call::new(arguments, TypeReference::Type(signature)),
        });
    }
}
