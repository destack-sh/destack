use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Emit one dynamic value binding.
    pub(super) fn emit_dynamic_bind(
        &mut self,
        destination: mir::Value,
        payload: mir::Value,
        concrete: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the dynamic representation and its dispatch table
        let dynamic_type = self
            .optimized
            .tree
            .storage_type(self.value_type(destination)?);
        let mir::Type::Dynamic { constraint, .. } = self.optimized.tree.get(dynamic_type) else {
            return Err(self.invalid("native dynamic binding result is not dynamic"));
        };
        let table = self
            .object
            .dynamic_index(concrete, *constraint)
            .ok_or_else(|| self.invalid("native dynamic table is absent"))?;
        let table = self.index_u32(native::Index::Dynamic { table }, builder)?;
        let payload = self.scalar(payload)?;
        self.set(destination, Value::ScalarPair([payload, table]))?;

        Ok(())
    }

    /// Emit one erased dynamic payload projection.
    pub(super) fn emit_dynamic_payload(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
    ) -> Result<(), EmitError> {
        let [payload, _] = self
            .value(dynamic)?
            .scalar_pair()
            .ok_or_else(|| self.invalid("native dynamic value is not a scalar pair"))?;
        self.set(destination, Value::Direct(payload))?;

        Ok(())
    }

    /// Emit one dynamic field read.
    pub(super) fn emit_dynamic_read(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
        slot: mir::DispatchSlot,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the dynamic entry and address it on the memory base
        let (payload, offset) = self.dynamic_entry(dynamic, slot, builder)?;
        let payload = self.rebase(payload, builder)?;
        let offset = builder.ins().uextend(self.types.pointer(), offset);
        let address = builder.ins().iadd(payload, offset);
        let value_type = self.types.value(result_type)?;
        let value = self.load(address, value_type, builder)?;
        self.set(destination, value)?;

        Ok(())
    }

    /// Emit one dynamic table identity projection.
    pub(super) fn emit_dynamic_type(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let (_, table) = self.dynamic_table(dynamic, builder)?;
        let concrete = builder.ins().load(
            cir::types::I32,
            cir::MemFlagsData::trusted(),
            table,
            std::mem::offset_of!(native::abi::DynamicTable, concrete) as i32,
        );
        self.set(destination, Value::Direct(concrete))?;

        Ok(())
    }

    /// Load one entry from an erased value's process-local dynamic table.
    pub(super) fn dynamic_entry(
        &self,
        dynamic: mir::Value,
        slot: mir::DispatchSlot,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(cir::Value, cir::Value), EmitError> {
        let (payload, table) = self.dynamic_table(dynamic, builder)?;
        let entry = builder.ins().load(
            cir::types::I32,
            cir::MemFlagsData::trusted(),
            table,
            native::abi::DynamicTable::entry_offset(slot.0) as i32,
        );

        Ok((payload, entry))
    }

    /// Resolve one erased value's process-local dynamic table.
    fn dynamic_table(
        &self,
        dynamic: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(cir::Value, cir::Value), EmitError> {
        let [payload, table] = self
            .value(dynamic)?
            .scalar_pair()
            .ok_or_else(|| self.invalid("native dynamic value is not a scalar pair"))?;
        let pointer = self.types.pointer();
        let tables = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, dynamics),
            builder,
        )?;
        let table = builder.ins().uextend(pointer, table);
        let table_offset = builder
            .ins()
            .ishl_imm_u(table, i64::from(pointer.bytes().trailing_zeros()));
        let table_address = builder.ins().iadd(tables, table_offset);
        let table = builder
            .ins()
            .load(pointer, cir::MemFlagsData::trusted(), table_address, 0);

        Ok((payload, table))
    }
}
