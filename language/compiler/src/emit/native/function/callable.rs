use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program as program;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl<'a> FunctionEmitter<'a> {
    /// Emit one function identity.
    pub(super) fn emit_function_address(
        &mut self,
        destination: mir::Value,
        function: mir::FunctionId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let function = self.function_value(function, builder)?;
        self.set(destination, Value::Direct(function))?;

        Ok(())
    }

    /// Emit one function value from its code and environment fields.
    pub(super) fn emit_function_bind(
        &mut self,
        destination: mir::Value,
        function: mir::FunctionId,
        environment: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let function = self.function_value(function, builder)?;
        let environment = self.scalar(environment)?;
        self.set(destination, Value::ScalarPair([function, environment]))?;

        Ok(())
    }

    /// Emit the environment carried by one function value.
    pub(super) fn emit_function_environment(
        &mut self,
        destination: mir::Value,
        function: mir::Value,
    ) -> Result<(), EmitError> {
        let [_, environment] = self
            .value(function)?
            .scalar_pair()
            .ok_or_else(|| self.invalid("native function value is not a scalar pair"))?;
        self.set(destination, Value::Direct(environment))?;

        Ok(())
    }

    /// Emit the current function's hidden environment.
    pub(super) fn emit_function_environment_current(
        &mut self,
        destination: mir::Value,
    ) -> Result<(), EmitError> {
        let environment = self
            .environment
            .ok_or_else(|| self.invalid("native function has no current environment"))?;
        self.set(destination, Value::Direct(environment))?;

        Ok(())
    }

    /// Return one stable Program function value.
    fn function_value(
        &mut self,
        function: mir::FunctionId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let function = self.index_u32(
            native::Index::Function {
                function: function.id,
            },
            builder,
        )?;
        let function = builder.ins().uextend(self.types.pointer(), function);

        Ok(builder
            .ins()
            .iadd_imm_u(function, program::FunctionId::WORD_BIAS as i64))
    }
}
