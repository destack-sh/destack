use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::{Linkage, Module};
use tspp_native as native;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Call one linked platform floating-point function.
    pub(super) fn emit_float_import(
        &mut self,
        import: native::Import,
        arguments: &[cir::Value],
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // reuse one Cranelift reference for each platform function
        if let Some(function) = self.platform_functions.get(&import).copied() {
            let call = builder.ins().call(function, arguments);

            return Ok(builder.func.dfg.first_result(call));
        }

        // build the platform calling signature from the physical values
        let ty = arguments
            .first()
            .map(|value| builder.func.dfg.value_type(*value))
            .ok_or_else(|| self.invalid("native float import has no arguments"))?;
        let mut signature = cir::Signature::new(self.types.call_conv());
        for argument in arguments {
            let argument_type = builder.func.dfg.value_type(*argument);
            signature.params.push(cir::AbiParam::new(argument_type));
        }
        signature.returns.push(cir::AbiParam::new(ty));

        // declare one object import and one function-local reference
        let name = format!("__destack_import_{:02x}", import as u32);
        let function = self
            .output
            .declare_function(&name, Linkage::Import, &signature)
            .map_err(|error| Self::internal(self.module, error.to_string()))?;
        self.imports.insert(function, import);
        let function = self.output.declare_func_in_func(function, builder.func);
        builder.func.dfg.ext_funcs[function].colocated = true;
        self.platform_functions.insert(import, function);

        // route the call through its linked image-local trampoline
        let call = builder.ins().call(function, arguments);

        Ok(builder.func.dfg.first_result(call))
    }
}
