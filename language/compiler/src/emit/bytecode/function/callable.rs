use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one function pointer.
    pub(super) fn emit_function_address(
        &mut self,
        destination: mir::Value,
        function: mir::FunctionId,
    ) -> Result<(), EmitError> {
        let function = self.types.function_id(function)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::FUNCTION_ADDRESS);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, function.0);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one function value from its code and environment fields.
    pub(super) fn emit_function_bind(
        &mut self,
        destination: mir::Value,
        function: mir::FunctionId,
        environment: mir::Value,
    ) -> Result<(), EmitError> {
        let function = self.types.function_id(function)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::FUNCTION_BIND);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, function.0);
        instruction.register(self.word(environment)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit the environment carried by one function value.
    pub(super) fn emit_function_environment(
        &mut self,
        destination: mir::Value,
        function: mir::Value,
    ) -> Result<(), EmitError> {
        let function = self.register(function)?;
        let environment =
            bytecode::RegisterSpan::new(bytecode::RegisterId(function.start.0 + 1), 1);
        let destination_type = self.register_type(destination)?;
        let destination = self.register(destination)?;

        self.emit_move(environment, destination, destination_type)
    }

    /// Emit the current function's hidden environment.
    pub(super) fn emit_function_environment_current(
        &mut self,
        destination: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self
            .function
            .environment
            .ok_or_else(|| self.internal("function has no current environment"))?;
        let ty = self.types.register_type(ty)?;
        let environment = bytecode::RegisterSpan::new(bytecode::RegisterId(0), ty.word_count());
        let destination = self.register(destination)?;

        self.emit_move(environment, destination, ty)
    }
}
