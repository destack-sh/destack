use std::mem::replace;

use crate::build::{BuildError, FunctionBuilder};
use crate::{
    Block, Call, Callee, Function, GenericArgument, Instruction, LocalNodeId, Substitution, Type,
    TypeId, Value,
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
        // invoke the target with the cleanup it unwinds into, continuing in a fresh block
        let result = Substitution::resolve(result_type, self.tree);
        let is_void = matches!(self.tree.get(result), Type::Void);
        if let Some(unwind) = self.unwind {
            let normal = self.block();
            let destination = (!is_void).then(|| self.add_block_parameter(normal, result_type));
            self.invoke(
                callee,
                signature,
                argument_values,
                normal,
                Vec::new(),
                unwind,
                Vec::new(),
            );
            self.switch_to_block(normal);

            return destination;
        }

        // omit SSA storage for void calls
        let destination = if is_void {
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

    /// Set the cleanup block calls and panics unwind into, returning the previous one.
    pub fn set_unwind(&mut self, unwind: Option<LocalNodeId<Block>>) -> Option<LocalNodeId<Block>> {
        replace(&mut self.unwind, unwind)
    }

    /// Call one declared function directly, deriving its signature from the header.
    pub fn call_function(
        &mut self,
        function: LocalNodeId<Function>,
        arguments: Vec<Value>,
    ) -> Option<Value> {
        // intern the declared signature
        let declared = self.tree.get(function);
        let result = declared.return_type;
        let signature = self.tree.intern_type(declared.signature());

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

    /// Return the result type one callable signature declares.
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
}
