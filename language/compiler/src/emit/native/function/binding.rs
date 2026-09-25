use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{Call, FunctionEmitter, Return};

impl FunctionEmitter<'_> {
    /// Emit one imported binding through the runtime operation table.
    pub(super) fn emit_binding_values(
        &mut self,
        call: &mir::Call,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(cir::Inst, Option<(cir::Value, ValueType)>), EmitError> {
        let call = self.binding_call(call, builder)?;
        let result = match call.result {
            Return::Buffer {
                address,
                value_type,
            } => Some((address, value_type)),
            Return::Void => None,
            Return::Registers(_) | Return::Address { .. } => {
                return Err(self.invalid("native binding has an invalid result representation"));
            }
        };
        let instruction = call.emit(builder);

        Ok((instruction, result))
    }

    /// Build one imported binding call through the runtime table.
    pub(super) fn binding_call(
        &mut self,
        call: &mir::Call,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<Call, EmitError> {
        // require a direct callee for the imported binding
        let mir::Callee::Direct { function, .. } = call.callee else {
            return Err(self.invalid("native indirect binding dispatch is not emitted yet"));
        };
        let callee = self.optimized.tree.get(function);
        let arguments = self.optimized.tree.get_values(call.arguments);
        let argument_types = arguments
            .iter()
            .map(|argument| self.types.value(self.value_type(*argument)?))
            .collect::<Result<Vec<_>, _>>()?;
        let argument_count = argument_types
            .iter()
            .map(|value| value.word_count())
            .sum::<u32>();
        let argument_buffer = self.allocate_words(argument_count, builder);

        // write every argument into one canonical Word array
        let mut argument_offset = 0u32;
        for (argument, value_type) in arguments.iter().zip(argument_types.iter().copied()) {
            let address = builder
                .ins()
                .iadd_imm_u(argument_buffer, i64::from(argument_offset) * 8);
            let ty = self.value_type(*argument)?;
            let argument = self.value(*argument)?;
            self.store_words(address, argument, ty, value_type, builder)?;
            argument_offset += value_type.word_count();
        }

        // reserve canonical result storage selected by the Program signature
        let result_type = self.types.result(callee.return_type)?;
        let result_count = result_type.map_or(0, |value| value.word_count());
        let result_buffer = self.allocate_words(result_count, builder);
        self.zero(result_buffer, result_count * 8, builder);
        let function = self.index_u32(
            native::Index::Function {
                function: function.id,
            },
            builder,
        )?;
        let argument_count = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(argument_count));
        let result_count = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(result_count));
        let result = result_type.map_or(Return::Void, |value_type| Return::Buffer {
            address: result_buffer,
            value_type,
        });
        let call = self.runtime_call(
            native::abi::Operation::BindingCall,
            &[
                function,
                argument_buffer,
                argument_count,
                result_buffer,
                result_count,
            ],
            builder,
        )?;

        Ok(Call { result, ..call })
    }

    /// Allocate one canonical Word buffer.
    pub(in crate::emit::native::function) fn allocate_words(
        &self,
        word_count: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::Value {
        if word_count == 0 {
            return builder.ins().iconst(self.types.pointer(), 0);
        }

        self.allocate_bytes(word_count * 8, 8, builder)
    }
}
