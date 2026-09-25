use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use destack_mir as mir;
use destack_native as native;

use crate::EmitError;

use super::memory::MemoryRegion;
use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Broadcast one scalar across a fixed-width vector.
    pub(super) fn emit_vector_splat(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let ty = self.vector_type(destination)?;
        let value = self.scalar(value)?;
        let vector = builder.ins().splat(ty, value);
        self.set(destination, Value::Direct(vector))
    }

    /// Extract one dynamically selected vector lane.
    pub(super) fn emit_vector_extract(
        &mut self,
        destination: mir::Value,
        vector: mir::Value,
        index: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let vector_type = self.vector_type(vector)?;
        let vector = self.scalar(vector)?;
        let index = self.scalar(index)?;
        let lane = self.dynamic_lane(vector, index, vector_type, None, builder)?;
        self.set(destination, Value::Direct(lane))
    }

    /// Replace one dynamically selected vector lane.
    pub(super) fn emit_vector_insert(
        &mut self,
        destination: mir::Value,
        vector: mir::Value,
        index: mir::Value,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let vector_type = self.vector_type(vector)?;
        let vector = self.scalar(vector)?;
        let index = self.scalar(index)?;
        let value = self.scalar(value)?;
        let vector = self.dynamic_lane(vector, index, vector_type, Some(value), builder)?;
        self.set(destination, Value::Direct(vector))
    }

    /// Shuffle two vectors through one constant lane mask.
    pub(super) fn emit_vector_shuffle(
        &mut self,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
        mask: mir::IndexSlice,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the vector type and shuffle indices
        let ty = self.vector_type(left)?;
        let lane_count = ty.lane_count();
        let mask = self.optimized.tree.get_indices(mask);
        if mask.len() != lane_count as usize {
            return Err(self.invalid("native vector shuffle mask has the wrong lane count"));
        }
        let left = self.scalar(left)?;
        let right = self.scalar(right)?;

        // use the canonical 128-bit byte shuffle whenever the vector fits it exactly
        if ty.bytes() == 16 {
            let lane_bytes = ty.lane_type().bytes() as usize;
            let mut bytes = [0u8; 16];
            for (target, source) in mask.iter().copied().enumerate() {
                if source >= lane_count * 2 {
                    return Err(self.invalid("native vector shuffle lane is out of range"));
                }
                for byte in 0..lane_bytes {
                    bytes[target * lane_bytes + byte] = (source as usize * lane_bytes + byte) as u8;
                }
            }
            let flags = cir::MemFlagsData::new().with_endianness(cir::Endianness::Little);
            let bytes_type = cir::types::I8X16;
            let left = builder.ins().bitcast(bytes_type, flags, left);
            let right = builder.ins().bitcast(bytes_type, flags, right);
            let mask = builder
                .func
                .dfg
                .immediates
                .push(cir::immediates::V128Imm(bytes).into());
            let shuffled = builder.ins().shuffle(left, right, mask);
            let result = builder.ins().bitcast(ty, flags, shuffled);

            return self.set(destination, Value::Direct(result));
        }
        let mut result = None;

        // compose each logical lane without constraining the vector to 128 bits
        for (target, source) in mask.iter().copied().enumerate() {
            if source >= lane_count * 2 {
                return Err(self.invalid("native vector shuffle lane is out of range"));
            }
            let (vector, lane) = if source < lane_count {
                (left, source)
            } else {
                (right, source - lane_count)
            };
            let value = builder.ins().extractlane(vector, lane as u8);
            result = Some(match result {
                Some(result) => builder.ins().insertlane(result, value, target as u8),
                None => builder.ins().splat(ty, value),
            });
        }
        let result = result.ok_or_else(|| self.invalid("native vector shuffle is empty"))?;
        self.set(destination, Value::Direct(result))
    }

    /// Select each vector lane through one boolean mask lane.
    pub(super) fn emit_vector_select(
        &mut self,
        destination: mir::Value,
        mask: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let ty = self.vector_type(then_value)?;
        let mask = self.scalar(mask)?;
        let then_value = self.scalar(then_value)?;
        let else_value = self.scalar(else_value)?;
        let lane_bits = u16::try_from(ty.lane_type().bits())
            .map_err(|_| self.invalid("native vector select lane width is unsupported"))?;
        let lane_type = cir::Type::int(lane_bits)
            .ok_or_else(|| self.invalid("native vector select lane width is unsupported"))?;
        let mask_type = lane_type
            .by(ty.lane_count())
            .ok_or_else(|| self.invalid("native vector select mask shape is unsupported"))?;
        let mut expanded = None;

        // expand canonical byte booleans into full-width machine mask lanes
        for lane in 0..ty.lane_count() {
            let condition = builder.ins().extractlane(mask, lane as u8);
            let condition = if builder.func.dfg.value_type(condition) == lane_type {
                condition
            } else {
                builder.ins().uextend(lane_type, condition)
            };
            let condition = builder.ins().ineg(condition);
            expanded = Some(match expanded {
                Some(expanded) => builder.ins().insertlane(expanded, condition, lane as u8),
                None => builder.ins().splat(mask_type, condition),
            });
        }
        let expanded = expanded.ok_or_else(|| self.invalid("native vector select is empty"))?;
        let expanded = if mask_type == ty {
            expanded
        } else {
            builder
                .ins()
                .bitcast(ty, cir::MemFlagsData::new(), expanded)
        };
        let result = builder.ins().bitselect(expanded, then_value, else_value);
        self.set(destination, Value::Direct(result))
    }

    /// Reduce one vector into a scalar value.
    pub(super) fn emit_vector_reduce(
        &mut self,
        destination: mir::Value,
        operator: mir::VectorReduceOperator,
        vector: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let ty = self.vector_type(vector)?;
        let element = self.vector_element(vector)?;
        let vector = self.scalar(vector)?;
        let mut result = builder.ins().extractlane(vector, 0);
        for lane in 1..ty.lane_count() {
            let right = builder.ins().extractlane(vector, lane as u8);
            result = self.reduce(operator, element, result, right, builder)?;
        }
        self.set(destination, Value::Direct(result))
    }

    /// Compare two vectors into byte-wide boolean lanes.
    pub(super) fn emit_vector_compare(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let source_type = self.vector_type(left)?;
        let target_type = self.vector_type(destination)?;
        let element = self.vector_element(left)?;
        let left = self.scalar(left)?;
        let right = self.scalar(right)?;
        let comparison = self.emit_binary(operator, left, right, element, builder)?;
        let one = builder.ins().iconst(source_type.lane_type(), 1);
        let one = builder.ins().splat(source_type, one);
        let comparison = builder.ins().band(comparison, one);
        let mut result = None;

        // compact full-width machine comparisons into canonical byte booleans
        for lane in 0..source_type.lane_count() {
            let value = builder.ins().extractlane(comparison, lane as u8);
            let value = if source_type.lane_type() == target_type.lane_type() {
                value
            } else {
                builder.ins().ireduce(target_type.lane_type(), value)
            };
            result = Some(match result {
                Some(result) => builder.ins().insertlane(result, value, lane as u8),
                None => builder.ins().splat(target_type, value),
            });
        }
        let result = result.ok_or_else(|| self.invalid("native vector comparison is empty"))?;
        self.set(destination, Value::Direct(result))
    }

    /// Convert each vector lane into one target scalar representation.
    pub(super) fn emit_vector_convert(
        &mut self,
        destination: mir::Value,
        mode: mir::ConvertMode,
        vector: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let source_type = self.vector_type(vector)?;
        let target_type = self.vector_type(destination)?;
        if source_type.lane_count() != target_type.lane_count() {
            return Err(self.invalid("native vector conversion changes the lane count"));
        }
        let source_element = self.vector_element(vector)?;
        let target_element = self.vector_element(destination)?;
        let vector = self.scalar(vector)?;

        // preserve identity conversions as the original SIMD value
        if source_element == target_element {
            return self.set(destination, Value::Direct(vector));
        }
        let mut result = None;

        // convert scalar lanes to preserve the exact language conversion semantics
        for lane in 0..source_type.lane_count() {
            let value = builder.ins().extractlane(vector, lane as u8);
            let value = self.emit_convert(value, source_element, target_element, mode, builder)?;
            result = Some(match result {
                Some(result) => builder.ins().insertlane(result, value, lane as u8),
                None => builder.ins().splat(target_type, value),
            });
        }
        let result = result.ok_or_else(|| self.invalid("native vector conversion is empty"))?;
        self.set(destination, Value::Direct(result))
    }

    /// Access one dynamic vector lane through canonical stack storage.
    fn dynamic_lane(
        &self,
        vector: cir::Value,
        index: cir::Value,
        ty: cir::Type,
        value: Option<cir::Value>,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let out_of_bounds = builder.ins().icmp_imm_u(
            cir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            index,
            i64::from(ty.lane_count()),
        );
        let trap = cir::TrapCode::unwrap_user(native::abi::Trap::Bounds.code() as u8);
        builder.ins().trapnz(out_of_bounds, trap);
        let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
            cir::StackSlotKind::ExplicitSlot,
            ty.bytes(),
            ty.bytes().next_power_of_two().trailing_zeros() as u8,
        ));
        let base = builder.ins().stack_addr(self.types.pointer(), slot, 0);
        let flags = self.memory_flags(MemoryRegion::World);
        builder.ins().store(flags, vector, base, 0);
        let lane_bytes = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(ty.lane_type().bytes()));
        let index = if builder.func.dfg.value_type(index) == self.types.pointer() {
            index
        } else {
            builder.ins().uextend(self.types.pointer(), index)
        };
        let offset = builder.ins().imul(index, lane_bytes);
        let address = builder.ins().iadd(base, offset);
        if let Some(value) = value {
            builder.ins().store(flags, value, address, 0);

            Ok(builder.ins().load(ty, flags, base, 0))
        } else {
            Ok(builder.ins().load(ty.lane_type(), flags, address, 0))
        }
    }

    /// Return one MIR vector's direct Cranelift type.
    fn vector_type(&self, value: mir::Value) -> Result<cir::Type, EmitError> {
        self.types
            .value(self.value_type(value)?)?
            .direct()
            .filter(|ty| ty.is_vector())
            .ok_or_else(|| self.invalid("native value is not one fixed vector"))
    }

    /// Return one MIR vector's element type.
    fn vector_element(&self, value: mir::Value) -> Result<mir::TypeId, EmitError> {
        // resolve the vector storage type
        let ty = self.optimized.tree.storage_type(self.value_type(value)?);
        match self.optimized.tree.type_definition(ty) {
            mir::Type::Vector { element, .. } => Ok(*element),
            _ => Err(self.invalid("native value has no vector element type")),
        }
    }

    /// Reduce two scalar vector lanes.
    fn reduce(
        &self,
        operator: mir::VectorReduceOperator,
        element: mir::TypeId,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let value = match operator {
            mir::VectorReduceOperator::Add if ty.is_float() => builder.ins().fadd(left, right),
            mir::VectorReduceOperator::Add => builder.ins().iadd(left, right),
            mir::VectorReduceOperator::Multiply if ty.is_float() => builder.ins().fmul(left, right),
            mir::VectorReduceOperator::Multiply => builder.ins().imul(left, right),
            mir::VectorReduceOperator::Min if ty.is_float() => builder.ins().fmin(left, right),
            mir::VectorReduceOperator::Max if ty.is_float() => builder.ins().fmax(left, right),
            mir::VectorReduceOperator::Min if self.types.is_signed_integer(element)? => {
                builder.ins().smin(left, right)
            }
            mir::VectorReduceOperator::Min => builder.ins().umin(left, right),
            mir::VectorReduceOperator::Max if self.types.is_signed_integer(element)? => {
                builder.ins().smax(left, right)
            }
            mir::VectorReduceOperator::Max => builder.ins().umax(left, right),
            mir::VectorReduceOperator::And => builder.ins().band(left, right),
            mir::VectorReduceOperator::Or => builder.ins().bor(left, right),
            mir::VectorReduceOperator::Xor => builder.ins().bxor(left, right),
        };

        Ok(value)
    }
}
