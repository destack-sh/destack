use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one generated frame destructor call.
    pub(super) fn emit_drop(
        &mut self,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let destructor = self
            .optimized
            .drops
            .destructor(ty, mir::Storage::Frame)
            .ok_or_else(|| self.invalid("native drop has no frame destructor"))?;
        let function = self
            .functions
            .get(&destructor)
            .copied()
            .ok_or_else(|| self.invalid("native frame destructor was not declared"))?;
        let function = self.output.declare_func_in_func(function, builder.func);
        let value_type = self.types.value(ty)?;
        let value = self.value(value)?;
        let address = self.materialize(value, value_type, builder)?;

        // generated destructors take the activation and one exclusive frame reference
        let activation = self.activation()?;
        builder.ins().call(function, &[activation, address]);

        Ok(())
    }
}
