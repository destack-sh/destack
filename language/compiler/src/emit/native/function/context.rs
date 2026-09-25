use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program::ContextNode;

use crate::EmitError;

use super::memory::AliasRegion;
use super::{FunctionEmitter, Value};

impl<'a> FunctionEmitter<'a> {
    /// Emit one current execution context load.
    pub(super) fn emit_context_current(
        &mut self,
        destination: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let offset = std::mem::offset_of!(native::abi::Activation, context);
        let context = self.activation_pointer(offset, builder)?;
        self.set(destination, Value::Direct(context))?;

        Ok(())
    }

    /// Emit one current execution context replacement.
    pub(super) fn emit_context_replace(
        &mut self,
        destination: mir::Value,
        context: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let activation = self.activation()?;
        let flags = self.memory_flags(AliasRegion::Activation);
        let offset = std::mem::offset_of!(native::abi::Activation, context);
        let previous = self.activation_pointer(offset, builder)?;
        let context = self.reference(context)?;
        builder
            .ins()
            .store(flags, context, activation, offset as i32);
        self.set(destination, Value::Direct(previous))?;

        Ok(())
    }

    /// Emit one immutable execution context extension.
    pub(super) fn emit_context_bind(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        context: mir::Value,
        variable: mir::Value,
        value: mir::Value,
        node_type: mir::TypeId,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let (value_offset, byte_len) = self.context_layout(node_type, value)?;
        let space = self.heap_space(result_type)?;
        self.emit_new(
            instruction,
            destination,
            result_type,
            space,
            native::abi::AllocationInitialization::Zeroed,
            None,
            builder,
        )?;

        // write the fixed header and concrete inline value before publication
        let address = self.materialize_pointer(destination, builder)?;
        let flags = self.memory_flags(AliasRegion::World);
        let context = self.reference(context)?;
        let variable = self.reference(variable)?;
        builder
            .ins()
            .store(flags, context, address, ContextNode::PARENT_OFFSET as i32);
        builder.ins().store(
            flags,
            variable,
            address,
            ContextNode::VARIABLE_OFFSET as i32,
        );
        let value_address = builder.ins().iadd_imm_u(address, i64::from(value_offset));
        let value_type = self.types.value(self.value_type(value)?)?;
        let value = self.value(value)?;
        self.store(value_address, value, value_type, builder)?;

        // publish managed references through the ordinary heap barrier
        let offset = builder.ins().iconst(self.types.pointer(), 0);
        let byte_len = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(byte_len));
        self.emit_runtime(
            native::abi::Operation::WriteBarrier,
            &[address, offset, byte_len],
            builder,
        )?;

        Ok(())
    }

    /// Emit one execution context value lookup.
    pub(super) fn emit_context_get(
        &mut self,
        destination: mir::Value,
        context: mir::Value,
        variable: mir::Value,
        default: mir::Value,
        node_type: mir::TypeId,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let (value_offset, _) = self.context_layout(node_type, default)?;
        if self.value_type(destination)? != result_type {
            return Err(self.invalid("native context result type does not match its destination"));
        }
        let value_type = self.types.value(result_type)?;
        let pointer = self.types.pointer();
        let context = self.reference(context)?;
        let variable = self.reference(variable)?;
        let default = self.value(default)?;
        let memory = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, memory_base),
            builder,
        )?;

        // build one header-only search loop over the immutable context chain
        let search = builder.create_block();
        let inspect = builder.create_block();
        let fallback = builder.create_block();
        let found = builder.create_block();
        let next = builder.create_block();
        let complete = builder.create_block();
        let current = builder.append_block_param(search, pointer);
        let result = Value::block_parameters(value_type, pointer, complete, builder);
        builder.ins().jump(search, &[context.into()]);

        builder.switch_to_block(search);
        let is_empty = builder
            .ins()
            .icmp_imm_u(cir::condcodes::IntCC::Equal, current, 0);
        builder.ins().brif(is_empty, fallback, &[], inspect, &[]);

        // compare the nearest context variable identity
        builder.switch_to_block(inspect);
        builder.seal_block(inspect);
        let address = builder.ins().iadd(memory, current);
        let flags = self.memory_flags(AliasRegion::World);
        let candidate =
            builder
                .ins()
                .load(pointer, flags, address, ContextNode::VARIABLE_OFFSET as i32);
        let is_match = builder
            .ins()
            .icmp(cir::condcodes::IntCC::Equal, candidate, variable);
        builder.ins().brif(is_match, found, &[], next, &[]);

        // return the concrete value stored in the matching node
        builder.switch_to_block(found);
        builder.seal_block(found);
        let value_address = builder.ins().iadd_imm_u(address, i64::from(value_offset));
        let value = self.load(value_address, value_type, builder)?;
        let mut arguments = Vec::with_capacity(value_type.abi_parameter_count());
        value.append_block_arguments(&mut arguments);
        builder.ins().jump(complete, &arguments);

        // continue with the immutable parent reference
        builder.switch_to_block(next);
        builder.seal_block(next);
        let parent = builder
            .ins()
            .load(pointer, flags, address, ContextNode::PARENT_OFFSET as i32);
        builder.ins().jump(search, &[parent.into()]);
        builder.seal_block(search);

        // use the explicit ContextVar default at the empty sentinel
        builder.switch_to_block(fallback);
        builder.seal_block(fallback);
        let mut arguments = Vec::with_capacity(value_type.abi_parameter_count());
        default.append_block_arguments(&mut arguments);
        builder.ins().jump(complete, &arguments);

        builder.switch_to_block(complete);
        builder.seal_block(complete);
        self.set(destination, result)?;

        Ok(())
    }

    /// Return and verify one concrete context node layout.
    fn context_layout(
        &self,
        node_type: mir::TypeId,
        value: mir::Value,
    ) -> Result<(u32, u32), EmitError> {
        // read the physical context layout
        let layout = self
            .optimized
            .layouts
            .type_layout(node_type)
            .ok_or_else(|| self.invalid("native context node has no layout"))?;
        let parent = layout
            .source_field(0)
            .ok_or_else(|| self.invalid("native context node has no parent field"))?;
        let variable = layout
            .source_field(1)
            .ok_or_else(|| self.invalid("native context node has no variable field"))?;
        let stored = layout
            .source_field(2)
            .ok_or_else(|| self.invalid("native context node has no value field"))?;

        // require the fixed Program header and exact concrete value type
        if parent.offset as usize != ContextNode::PARENT_OFFSET
            || variable.offset as usize != ContextNode::VARIABLE_OFFSET
            || stored.offset < ContextNode::BYTE_LEN as u32
            || stored.ty != self.value_type(value)?
        {
            return Err(self.invalid("native context node does not match the Program ABI"));
        }

        Ok((stored.offset, layout.size))
    }
}
