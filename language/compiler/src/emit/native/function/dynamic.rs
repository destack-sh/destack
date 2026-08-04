use destack_mir as mir;
use destack_native as native;

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
        let dynamic_type = self.value_type(destination)?;
        let mir::Type::Dynamic { constraint, .. } = self.optimized.tree.get(dynamic_type) else {
            return Err(self.invalid("native dynamic binding result is not dynamic"));
        };
        let table = self
            .object
            .dynamic_index(concrete, *constraint)
            .ok_or_else(|| self.invalid("native dynamic table is absent"))?;
        let table = self.index_u32(native::Index::Dynamic { table }, builder)?;
        let payload = self.scalar(payload, builder)?;
        self.set(destination, Value::ScalarPair([payload, table]), builder)?;

        Ok(())
    }

    /// Emit one erased dynamic payload projection.
    pub(super) fn emit_dynamic_payload(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let [payload, _] = self
            .value(dynamic, builder)?
            .scalar_pair()
            .ok_or_else(|| self.invalid("native dynamic value is not a scalar pair"))?;
        self.set(destination, Value::Direct(payload), builder)?;

        Ok(())
    }

    /// Emit one dynamic table identity projection.
    pub(super) fn emit_dynamic_type(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let [_, ty] = self
            .value(dynamic, builder)?
            .scalar_pair()
            .ok_or_else(|| self.invalid("native dynamic value is not a scalar pair"))?;
        self.set(destination, Value::Direct(ty), builder)?;

        Ok(())
    }
}
