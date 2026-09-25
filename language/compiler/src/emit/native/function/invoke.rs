use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program::object::{FramePoint, Point};

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{FunctionEmitter, Return, Value};

impl FunctionEmitter<'_> {
    /// Emit one call with explicit normal and unwind continuations.
    pub(in crate::emit::native::function) fn emit_invoke(
        &mut self,
        call: &mir::Call,
        target: &mir::BlockTarget,
        unwind: &mir::BlockTarget,
        point: Point,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // build the call frame map
        let frame = self.stack_map(FramePoint::operation(point), builder)?;
        let call = self.call(call, &frame, builder)?;
        let returned = builder.create_block();
        let landing = builder.create_block();
        let skipped = builder.create_block();
        let pointer = self.types.pointer();
        let mut normal_arguments = Vec::new();
        let mut normal_types = Vec::new();

        // pass the physical call result into one normal-return block
        match call.result {
            Return::Void => {}
            Return::Registers(ValueType::Direct { ty, .. }) => {
                normal_arguments.push(cir::BlockArg::TryCallRet(0));
                normal_types.push(ty);
            }
            Return::Registers(ValueType::ScalarPair { fields, .. }) => {
                for (index, field) in fields.into_iter().enumerate() {
                    normal_arguments.push(cir::BlockArg::TryCallRet(index as u32));
                    normal_types.push(field.ty);
                }
            }
            Return::Registers(ValueType::Indirect { .. }) => {
                return Err(self.invalid("native call returns an indirect value in registers"));
            }
            Return::Address { address } | Return::Buffer { address, .. } => {
                normal_arguments.push(address.into());
                normal_types.push(pointer);
            }
        }

        // preserve explicit MIR edge arguments across the call
        for argument in self.optimized.tree.get_values(target.arguments) {
            let value = self.value(*argument)?;
            let mut values = Vec::new();
            value.append_values(&mut values);
            normal_types.extend(
                values
                    .iter()
                    .map(|value| builder.func.dfg.value_type(*value)),
            );
            normal_arguments.extend(values.into_iter().map(cir::BlockArg::from));
        }
        for ty in normal_types {
            builder.append_block_param(returned, ty);
        }

        // pass the platform unwind object and cleanup arguments into the landing block
        let exception_types = builder.func.dfg.signatures[call.signature]
            .call_conv
            .exception_payload_types(pointer)
            .to_vec();
        if exception_types.is_empty() {
            return Err(self.invalid("native calling convention cannot unwind"));
        }
        let mut exception_arguments = Vec::new();
        for (index, ty) in exception_types.iter().copied().enumerate() {
            builder.append_block_param(landing, ty);
            exception_arguments.push(cir::BlockArg::TryCallExn(index as u32));
        }
        for argument in self.optimized.tree.get_values(unwind.arguments) {
            let value = self.value(*argument)?;
            let mut values = Vec::new();
            value.append_values(&mut values);
            for value in values {
                let ty = builder.func.dfg.value_type(value);
                builder.append_block_param(landing, ty);
                exception_arguments.push(value.into());
            }
        }

        // terminate the call block with one catch-all native invocation
        let normal = cir::BlockCall::new(
            returned,
            normal_arguments,
            &mut builder.func.dfg.value_lists,
        );
        let exceptional = cir::BlockCall::new(
            landing,
            exception_arguments,
            &mut builder.func.dfg.value_lists,
        );
        let exceptions = builder
            .func
            .dfg
            .exception_tables
            .push(cir::ExceptionTableData::new(
                call.signature,
                normal,
                [cir::ExceptionTableItem::Default(exceptional)],
            ));
        let instruction = call.invoke(exceptions, builder);
        Self::attach_stack_map(frame.entries, instruction, builder);

        // translate physical returns into the MIR result parameter
        builder.switch_to_block(returned);
        builder.seal_block(returned);
        let parameters = builder.block_params(returned).to_vec();
        let mut index = 0;
        let result = match call.result {
            Return::Void => None,
            Return::Registers(value_type) => {
                Value::from_parameters(value_type, &parameters, &mut index)
            }
            Return::Address { .. } => {
                let address = parameters[index];
                index += 1;

                Some(Value::Address(address))
            }
            Return::Buffer { value_type, .. } => {
                let address = parameters[index];
                index += 1;

                Some(self.load(address, value_type, builder)?)
            }
        };
        let mut arguments = Vec::new();
        if let Some(result) = result {
            result.append_block_arguments(&mut arguments);
        }
        arguments.extend(parameters[index..].iter().copied().map(cir::BlockArg::from));
        builder.ins().jump(self.blocks[&target.block], &arguments);

        // classify the unwind before entering language cleanup
        builder.switch_to_block(landing);
        builder.seal_block(landing);
        let parameters = builder.block_params(landing).to_vec();
        let unwind_object = parameters[0];
        let unwind_slot = self.unwind_slot(builder);
        let unwind_address = builder.ins().stack_addr(pointer, unwind_slot, 0);
        let flags = cir::MemFlagsData::trusted();
        builder.ins().store(flags, unwind_object, unwind_address, 0);
        let classify = self.emit_runtime(native::abi::Operation::UnwindClassify, &[], builder)?;
        let action = builder.inst_results(classify)[0];
        let cleanup_arguments = parameters[exception_types.len()..]
            .iter()
            .copied()
            .map(cir::BlockArg::from)
            .collect::<Vec<_>>();
        builder.ins().brif(
            action,
            skipped,
            &[],
            self.blocks[&unwind.block],
            &cleanup_arguments,
        );

        // skip language cleanup for a retained activation or a trap
        builder.switch_to_block(skipped);
        builder.seal_block(skipped);
        self.emit_runtime(
            native::abi::Operation::UnwindResume,
            &[unwind_object],
            builder,
        )?;
        Self::terminate_runtime(builder);

        Ok(())
    }
}
