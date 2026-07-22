use crate::build::{BuildError, FunctionBuilder};
use crate::{
    Call, Callee, Function, Instruction, LocalNodeId, SignatureParameter, Type, TypeId, Value,
};

impl<'a> FunctionBuilder<'a> {
    /// Call one callable target.
    pub fn call(
        &mut self,
        callee: Callee,
        signature: TypeId,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        // resolve the call result
        let result_type = self.signature_result_type(signature);
        let result_type = self.expect_build(result_type);

        // omit SSA storage for void calls
        let destination = if matches!(self.tree.get(result_type), Type::Void) {
            None
        } else {
            Some(self.allocate_value())
        };

        // insert the unified call operation
        let arguments = self.tree.add_values(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination,
            call: Call::new(callee, arguments, signature),
        });

        // record the result type when the call returns a value
        if let Some(destination) = destination {
            self.define_value(destination, result_type);
        }

        destination
    }

    /// Call one declared function directly, deriving its signature from the header.
    pub fn call_function(
        &mut self,
        function: LocalNodeId<Function>,
        arguments: Vec<Value>,
    ) -> Option<Value> {
        // rebuild the signature type from the declared parameters and result
        let declared = self.tree.get(function);
        let parameters = declared
            .parameters
            .iter()
            .map(|parameter| SignatureParameter {
                ty: parameter.ty,
                obligations: parameter.obligations.clone(),
            })
            .collect();
        let result = declared.return_type;
        let signature = self.tree.intern_type(Type::FunctionSignature {
            lifetimes: Vec::new(),
            parameters,
            result,
        });

        self.call(
            Callee::Direct { function },
            TypeId::from(signature),
            arguments,
        )
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

    /// Resolve the return type for one callable signature.
    fn signature_result_type(&self, signature: TypeId) -> Result<TypeId, BuildError> {
        let signature_type = self.tree.get(signature);
        match signature_type {
            Type::FunctionSignature { result, .. } => Ok(*result),
            Type::FunctionPointer { .. } | Type::Function { .. } => {
                let Some(signature) = signature_type.callable_signature() else {
                    return Err(BuildError::MissingFunctionSignature { ty: signature });
                };
                let signature_type = self.tree.get(signature);
                let Some((_, _, result)) = signature_type.function_signature_parts() else {
                    return Err(BuildError::MissingFunctionSignature { ty: signature });
                };

                Ok(result)
            }
            _ => Err(BuildError::MissingFunctionSignature { ty: signature }),
        }
    }
}
