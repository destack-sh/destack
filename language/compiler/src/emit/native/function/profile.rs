use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one explicit profile counter increment.
    pub(super) fn emit_profile_increment(
        &mut self,
        counter: mir::CounterId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let counter = self.index_u32(
            native::Index::Counter {
                function: self.function_index,
                counter: counter.0,
            },
            builder,
        )?;
        self.emit_runtime(
            native::abi::Operation::ProfileIncrement,
            &[counter],
            builder,
        )?;

        Ok(())
    }

    /// Emit one explicit scalar profile sample.
    pub(super) fn emit_profile_sample(
        &mut self,
        sampler: mir::SamplerId,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let sampler = self.index_u32(
            native::Index::Sampler {
                function: self.function_index,
                sampler: sampler.0,
            },
            builder,
        )?;
        let value = self.scalar(value)?;
        let value = self.profile_word(value, builder)?;
        self.emit_runtime(
            native::abi::Operation::ProfileSample,
            &[sampler, value],
            builder,
        )?;

        Ok(())
    }

    /// Convert one scalar observation into canonical Word bits.
    fn profile_word(
        &self,
        value: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(value);
        let bits = u16::try_from(ty.bits())
            .map_err(|_| self.invalid("native profile sample width is unsupported"))?;
        let integer = cir::Type::int(bits)
            .ok_or_else(|| self.invalid("native profile sample is not scalar"))?;
        let value = if ty == integer {
            value
        } else {
            builder
                .ins()
                .bitcast(integer, cir::MemFlagsData::new(), value)
        };
        let value = if integer == cir::types::I64 {
            value
        } else if integer.bits() < 64 {
            builder.ins().uextend(cir::types::I64, value)
        } else {
            return Err(self.invalid("native profile sample exceeds one Word"));
        };

        Ok(value)
    }
}
