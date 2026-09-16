use crate::build::{BuildError, FunctionBuilder};
use crate::{
    Copy,
    Call, Callee, Function, FunctionBehavior, GenericArgument, Instruction, LocalNodeId, Point,
    SignatureParameter, Substitution, Type, TypeId, Value,
};

impl<'a> FunctionBuilder<'a> {
    /// Call one callable target.
    pub fn call(
        &mut self,
        callee: Callee,
        signature: TypeId,
        argument_values: Vec<Value>,
        result_type: TypeId,
    ) -> Option<Value> {
        // omit SSA storage for void calls
        let result = Substitution::resolve(result_type, self.tree);
        let destination = if matches!(self.tree.get(result), Type::Void) {
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
            .map(|parameter| SignatureParameter { ty: parameter.ty })
            .collect();
        let result = declared.return_type;
        let lifetimes = declared.lifetimes.clone();
        let signature = self.tree.intern_type(
            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            },
            Copy::Yes,
        );

        self.call(
            Callee::Direct {
                function,
                arguments: Vec::new(),
            },
            signature,
            arguments,
            result,
        )
    }

    /// Load a function pointer value for a function.
    pub fn function_addr(
        &mut self,
        function: LocalNodeId<Function>,
        arguments: Vec<GenericArgument>,
        signature: TypeId,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionAddr {
            destination,
            function,
            arguments,
        });
        self.define_value(destination, signature);
        destination
    }

    /// Construct a function value for one function and environment.
    pub fn function_bind(
        &mut self,
        function: LocalNodeId<Function>,
        arguments: Vec<GenericArgument>,
        signature: TypeId,
        environment: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionBind {
            destination,
            function,
            arguments,
            environment,
        });
        self.define_value(destination, signature);
        destination
    }

    /// Load the hidden environment pointer for the current function.
    pub fn function_environment_current(&mut self, environment_type: TypeId) -> Value {
        // record the hidden environment type on the function tables
        let existing_environment = {
            let requested_environment = environment_type;
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

    /// Return the result type one callable signature declares, the destination a call at that
    /// signature defines without a caller-side type.
    pub fn signature_result(&mut self, signature: TypeId) -> TypeId {
        let result = self.signature_result_type(signature);

        self.expect_build(result)
    }

    /// Return the result type one callable signature declares.
    fn signature_result_type(&mut self, signature: TypeId) -> Result<TypeId, BuildError> {
        let signature = Substitution::resolve(signature, self.tree);
        let signature_type = self.tree.get(signature);
        match signature_type {
            Type::FunctionSignature { result, .. } => Ok(*result),
            Type::FunctionPointer { .. } | Type::Function { .. } => {
                let Some(signature) = signature_type.callable_signature() else {
                    return Err(BuildError::MissingFunctionSignature { ty: signature });
                };
                let signature = Substitution::resolve(signature, self.tree);
                let signature_type = self.tree.get(signature);
                let Some((_, _, result)) = signature_type.function_signature_parts() else {
                    return Err(BuildError::MissingFunctionSignature { ty: signature });
                };

                Ok(result)
            }
            _ => Err(BuildError::MissingFunctionSignature { ty: signature }),
        }
    }

    /// Mark the call inserted last as parking the current fiber.
    pub fn mark_park(&mut self) {
        let block = self.current_block();
        let instruction = *self
            .tree
            .get(block)
            .instructions
            .last()
            .unwrap_or_else(|| unreachable!("a park mark before any instruction"));
        let call = self.effects.upsert_call(Point::Instruction(instruction));
        call.behavior = Some(FunctionBehavior::none().with_park());
    }
}
