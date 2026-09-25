use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use destack_mir as mir;
use destack_native as native;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{FunctionEmitter, Value};

impl<'a> FunctionEmitter<'a> {
    /// Emit one fixed or repeated heap allocation.
    pub(super) fn emit_new(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        result_type: mir::TypeId,
        space: mir::Space,
        initialization: native::abi::AllocationInitialization,
        length: Option<mir::Value>,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // locate the allocation site
        let point = self.object.instruction_point(instruction);
        let site = self
            .object
            .allocation_index(point)
            .ok_or_else(|| self.invalid("native allocation has no site"))?;
        let value_type = self.types.value(result_type)?;
        let space = builder
            .ins()
            .iconst(cir::types::I32, self.native_space(space)? as i64);
        let site = self.index_u32(native::Index::Allocation { site }, builder)?;
        let initialization = builder.ins().iconst(cir::types::I32, initialization as i64);
        let length = length.map(|length| self.scalar(length)).transpose()?;

        // select the fixed or repeated runtime operation
        let call = if let Some(length) = length {
            self.emit_runtime(
                native::abi::Operation::AllocateRepeated,
                &[space, site, length, initialization],
                builder,
            )?
        } else {
            self.emit_runtime(
                native::abi::Operation::Allocate,
                &[space, site, initialization],
                builder,
            )?
        };
        // retain allocation results in their native physical representation
        let reference = builder.inst_results(call)[0];
        match value_type {
            ValueType::Direct { .. } => {
                self.set(destination, Value::Direct(reference))?;
            }
            ValueType::ScalarPair { .. } => {
                let length = length.ok_or_else(|| {
                    self.invalid("native scalar-pair allocation has no repeated length")
                })?;
                self.set(destination, Value::ScalarPair([reference, length]))?;
            }
            ValueType::Indirect { .. } => {
                let output = self.allocate(value_type, builder);
                let flags = cir::MemFlagsData::trusted();
                builder.ins().store(flags, reference, output, 0);
                if let Some(length) = length {
                    let layout = self
                        .optimized
                        .layouts
                        .type_layout(result_type)
                        .ok_or_else(|| self.invalid("native slice result has no layout"))?;
                    let field = layout
                        .source_field(1)
                        .ok_or_else(|| self.invalid("native slice length has no placement"))?;
                    builder
                        .ins()
                        .store(flags, length, output, field.offset as i32);
                }
                self.set(destination, Value::Address(output))?;
            }
        }

        Ok(())
    }
}
