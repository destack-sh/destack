use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use destack_mir as mir;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Emit one contiguous subslice descriptor.
    pub(super) fn emit_slice_view(
        &mut self,
        destination: mir::Value,
        source: mir::Value,
        start: mir::Value,
        length: mir::Value,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let Value::ScalarPair([reference, _]) = self.value(source, builder)? else {
            return Err(self.invalid("native slice is not a scalar pair"));
        };
        let start = self.scalar(start, builder)?;
        let length = self.scalar(length, builder)?;
        let element = match self.optimized.tree.get(result_type) {
            mir::Type::Slice { element, .. } => *element,
            _ => return Err(self.invalid("native slice view result is not a slice")),
        };
        let stride = self
            .optimized
            .layouts
            .type_layout(element)
            .map(|layout| layout.stride())
            .ok_or_else(|| self.invalid("native slice element has no layout"))?;
        let start = self.slice_index(start, builder)?;
        let byte_offset = builder.ins().imul_imm_u(start, stride as i64);
        let reference = builder.ins().iadd(reference, byte_offset);
        self.set(destination, Value::ScalarPair([reference, length]), builder)?;

        Ok(())
    }

    /// Emit one slice descriptor length.
    pub(super) fn emit_slice_length(
        &mut self,
        destination: mir::Value,
        slice: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let Value::ScalarPair([_, length]) = self.value(slice, builder)? else {
            return Err(self.invalid("native slice is not a scalar pair"));
        };
        self.set(destination, Value::Direct(length), builder)?;

        Ok(())
    }

    /// Convert one integer into the native pointer width.
    fn slice_index(
        &self,
        value: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let source = builder.func.dfg.value_type(value);
        let target = self.types.pointer();
        if !source.is_int() {
            return Err(self.invalid("native slice index is not an integer"));
        }

        let value = if source.bits() < target.bits() {
            builder.ins().uextend(target, value)
        } else if source.bits() > target.bits() {
            builder.ins().ireduce(target, value)
        } else {
            value
        };

        Ok(value)
    }
}
