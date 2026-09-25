use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Emit one atomic load.
    pub(super) fn emit_atomic_load(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        result_type: mir::TypeId,
        access: mir::AtomicAccess,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        self.require_closed(access.ordering.closed())?;
        let pointer = self.place_pointer(place, builder)?;
        let ty = self.atomic_type(result_type)?;
        let value = builder
            .ins()
            .atomic_load(ty, cir::MemFlagsData::trusted(), pointer);
        let value = self.decode_atomic(value, result_type, builder)?;
        self.set(destination, Value::Direct(value))?;

        Ok(())
    }

    /// Emit one atomic store.
    pub(super) fn emit_atomic_store(
        &mut self,
        place: &mir::Place,
        value: mir::Value,
        access: mir::AtomicAccess,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        self.require_closed(access.ordering.closed())?;

        // materialize the atomic address and stored value
        let pointer = self.place_pointer(place, builder)?;
        let ty = self.optimized.tree.storage_type(self.value_type(value)?);
        let value = self.scalar(value)?;
        let value = self.encode_atomic(value, ty, builder)?;
        builder
            .ins()
            .atomic_store(cir::MemFlagsData::trusted(), value, pointer);

        Ok(())
    }

    /// Emit one atomic compare exchange.
    pub(super) fn emit_atomic_compare_exchange(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        expected: mir::Value,
        new_value: mir::Value,
        _is_weak: bool,
        access: mir::CompareExchangeAccess,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        self.require_closed(access.success.ordering.closed())?;
        self.require_closed(access.failure_ordering())?;

        // materialize the atomic address and comparison values
        let pointer = self.place_pointer(place, builder)?;
        let ty = self.optimized.tree.storage_type(self.value_type(expected)?);
        let expected = self.scalar(expected)?;
        let new_value = self.scalar(new_value)?;
        let expected_bits = self.encode_atomic(expected, ty, builder)?;
        let new_bits = self.encode_atomic(new_value, ty, builder)?;
        let old_bits = builder.ins().atomic_cas(
            cir::MemFlagsData::trusted(),
            pointer,
            expected_bits,
            new_bits,
        );
        let is_exchanged =
            builder
                .ins()
                .icmp(cir::condcodes::IntCC::Equal, old_bits, expected_bits);
        let old = self.decode_atomic(old_bits, ty, builder)?;
        self.set(destination, Value::ScalarPair([old, is_exchanged]))?;

        Ok(())
    }

    /// Emit one atomic read modify write operation.
    pub(super) fn emit_atomic_rmw(
        &mut self,
        destination: mir::Value,
        operator: mir::AtomicRmwOperator,
        place: &mir::Place,
        value: mir::Value,
        access: mir::AtomicAccess,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        self.require_closed(access.ordering.closed())?;

        // materialize the atomic address and operand
        let pointer = self.place_pointer(place, builder)?;
        let ty = self.optimized.tree.storage_type(self.value_type(value)?);
        let value = self.scalar(value)?;
        let is_float = matches!(self.optimized.tree.type_definition(ty), mir::Type::Float(_));
        let old = match (is_float, operator) {
            // exchange the exact floating point representation directly
            (true, mir::AtomicRmwOperator::Exchange) => {
                self.emit_atomic_rmw_bits(cir::AtomicRmwOp::Xchg, pointer, value, ty, builder)?
            }

            // apply floating point arithmetic through a compare exchange loop
            (
                true,
                mir::AtomicRmwOperator::Add
                | mir::AtomicRmwOperator::Subtract
                | mir::AtomicRmwOperator::Min
                | mir::AtomicRmwOperator::Max,
            ) => self.emit_float_atomic_rmw(operator, pointer, value, ty, builder)?,

            // reject bitwise operations over floating point values
            (true, _) => {
                return Err(self.invalid("native floating atomic operator requires arithmetic"));
            }

            // select the concrete integer operation from signedness
            (false, _) => {
                let is_signed = self.types.is_signed_integer(ty)?;
                let operator = Self::integer_atomic_operator(operator, is_signed);

                self.emit_atomic_rmw_bits(operator, pointer, value, ty, builder)?
            }
        };
        self.set(destination, Value::Direct(old))?;

        Ok(())
    }

    /// Emit one atomic fence.
    pub(super) fn emit_atomic_fence(
        &self,
        access: mir::FenceAccess,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        self.require_closed(access.ordering.closed())?;
        builder.ins().fence();

        Ok(())
    }

    /// Require one memory ordering closed by instantiation.
    fn require_closed(&self, ordering: Result<mir::MemoryOrdering, u32>) -> Result<(), EmitError> {
        ordering.map(|_| ()).map_err(|_| {
            self.invalid("an atomic ordering parameter reached native emit before instantiation")
        })
    }

    /// Emit one floating read modify write operation with a compare exchange loop.
    fn emit_float_atomic_rmw(
        &self,
        operator: mir::AtomicRmwOperator,
        pointer: cir::Value,
        value: cir::Value,
        ty: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let atomic_type = self.atomic_type(ty)?;
        let initial = builder
            .ins()
            .atomic_load(atomic_type, cir::MemFlagsData::trusted(), pointer);
        let retry = builder.create_block();
        let complete = builder.create_block();
        builder.append_block_param(retry, atomic_type);
        builder.append_block_param(complete, atomic_type);
        builder.ins().jump(retry, &[initial.into()]);

        // calculate and attempt the next value from the observed bits
        builder.switch_to_block(retry);
        let expected_bits = builder.block_params(retry)[0];
        let expected = self.decode_atomic(expected_bits, ty, builder)?;
        let next = match operator {
            mir::AtomicRmwOperator::Add => builder.ins().fadd(expected, value),
            mir::AtomicRmwOperator::Subtract => builder.ins().fsub(expected, value),
            mir::AtomicRmwOperator::Min => builder.ins().fmin(expected, value),
            mir::AtomicRmwOperator::Max => builder.ins().fmax(expected, value),
            _ => return Err(self.invalid("native floating atomic operator is not floating")),
        };
        let next = self.encode_atomic(next, ty, builder)?;
        let observed =
            builder
                .ins()
                .atomic_cas(cir::MemFlagsData::trusted(), pointer, expected_bits, next);
        let is_exchanged =
            builder
                .ins()
                .icmp(cir::condcodes::IntCC::Equal, observed, expected_bits);
        builder.ins().brif(
            is_exchanged,
            complete,
            &[observed.into()],
            retry,
            &[observed.into()],
        );
        builder.seal_block(retry);

        // return the value observed by the successful attempt
        builder.switch_to_block(complete);
        builder.seal_block(complete);
        let old = builder.block_params(complete)[0];

        self.decode_atomic(old, ty, builder)
    }

    /// Emit one native atomic operation over the exact stored bits.
    fn emit_atomic_rmw_bits(
        &self,
        operator: cir::AtomicRmwOp,
        pointer: cir::Value,
        value: cir::Value,
        ty: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let value = self.encode_atomic(value, ty, builder)?;
        let atomic_type = builder.func.dfg.value_type(value);
        let old = builder.ins().atomic_rmw(
            atomic_type,
            cir::MemFlagsData::trusted(),
            operator,
            pointer,
            value,
        );

        self.decode_atomic(old, ty, builder)
    }

    /// Return the integer operation supported directly by Cranelift.
    fn integer_atomic_operator(
        operator: mir::AtomicRmwOperator,
        is_signed: bool,
    ) -> cir::AtomicRmwOp {
        match operator {
            mir::AtomicRmwOperator::Exchange => cir::AtomicRmwOp::Xchg,
            mir::AtomicRmwOperator::Add => cir::AtomicRmwOp::Add,
            mir::AtomicRmwOperator::Subtract => cir::AtomicRmwOp::Sub,
            mir::AtomicRmwOperator::And => cir::AtomicRmwOp::And,
            mir::AtomicRmwOperator::Or => cir::AtomicRmwOp::Or,
            mir::AtomicRmwOperator::Xor => cir::AtomicRmwOp::Xor,
            mir::AtomicRmwOperator::Min if is_signed => cir::AtomicRmwOp::Smin,
            mir::AtomicRmwOperator::Min => cir::AtomicRmwOp::Umin,
            mir::AtomicRmwOperator::Max if is_signed => cir::AtomicRmwOp::Smax,
            mir::AtomicRmwOperator::Max => cir::AtomicRmwOp::Umax,
        }
    }

    /// Return the integer memory type used for one atomic value.
    fn atomic_type(&self, ty: mir::TypeId) -> Result<cir::Type, EmitError> {
        let value_type = self
            .types
            .value(ty)?
            .direct()
            .ok_or_else(|| self.invalid("native atomic value is not scalar"))?;
        let bits = u16::try_from(value_type.bits())
            .map_err(|_| self.invalid("native atomic value width is unsupported"))?;

        cir::Type::int(bits).ok_or_else(|| self.invalid("native atomic value width is unsupported"))
    }

    /// Convert one native scalar into its atomic integer representation.
    fn encode_atomic(
        &self,
        value: cir::Value,
        ty: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let atomic_type = self.atomic_type(ty)?;
        let value_type = builder.func.dfg.value_type(value);
        if value_type == atomic_type {
            return Ok(value);
        }
        let flags = cir::MemFlagsData::new();

        Ok(builder.ins().bitcast(atomic_type, flags, value))
    }

    /// Convert one atomic integer into its MIR scalar representation.
    fn decode_atomic(
        &self,
        value: cir::Value,
        ty: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let value_type = self
            .types
            .value(ty)?
            .direct()
            .ok_or_else(|| self.invalid("native atomic value is not scalar"))?;
        if value_type == builder.func.dfg.value_type(value) {
            return Ok(value);
        }
        let flags = cir::MemFlagsData::new();

        Ok(builder.ins().bitcast(value_type, flags, value))
    }
}
