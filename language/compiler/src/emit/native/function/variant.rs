use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use destack_mir as mir;

use crate::EmitError;

use super::memory::AliasRegion;
use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Emit one variant case in its canonical physical representation.
    pub(super) fn emit_variant_new(
        &mut self,
        destination: mir::Value,
        case: u32,
        payload: Option<mir::Value>,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // resolve the variant storage type
        let result_type = self.optimized.tree.storage_type(result_type);
        let layout = self
            .optimized
            .layouts
            .type_layout(result_type)
            .ok_or_else(|| self.invalid("native variant has no layout"))?;
        let mir::LayoutShape::Variant(variant) = &layout.shape else {
            return Err(self.invalid("native variant result has no variant layout"));
        };
        let selected = variant
            .cases
            .get(case as usize)
            .ok_or_else(|| self.invalid("native variant case is absent"))?;
        let value_type = self.types.value(result_type)?;
        let address = self.allocate(value_type, builder);
        self.zero(address, value_type.byte_len(), builder);

        // place the selected payload before encoding possible payload niches
        if let Some(payload) = payload {
            let target = builder
                .ins()
                .iadd_imm_u(address, i64::from(selected.payload_offset));
            let payload_type = self.types.value(self.value_type(payload)?)?;
            let payload = self.value(payload)?;
            self.store(target, payload, payload_type, builder)?;
        }

        self.encode_variant(address, variant, case, builder)?;
        let value = if value_type.is_indirect() {
            Value::Address(address)
        } else {
            self.load(address, value_type, builder)?
        };
        self.set(destination, value)?;

        Ok(())
    }

    /// Emit one variant's logical discriminant.
    pub(super) fn emit_variant_tag(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // resolve the variant storage type
        let variant_type = self.optimized.tree.storage_type(self.value_type(variant)?);
        let layout = self
            .optimized
            .layouts
            .type_layout(variant_type)
            .ok_or_else(|| self.invalid("native variant has no layout"))?;
        let mir::LayoutShape::Variant(layout) = &layout.shape else {
            return Err(self.invalid("native variant value has no variant layout"));
        };
        let value_type = self.types.value(variant_type)?;
        let value = self.value(variant)?;
        let address = self.materialize(value, value_type, builder)?;
        let scalar = self.load_discriminant(address, layout.encoding.field(), builder)?;
        let destination_type = self
            .types
            .value(self.value_type(destination)?)?
            .direct()
            .ok_or_else(|| self.invalid("native variant tag result is not scalar"))?;
        let tag = self.decode_variant(scalar, layout, destination_type, builder)?;
        self.set(destination, Value::Direct(tag))?;

        Ok(())
    }

    /// Emit one stored variant's logical discriminant.
    pub(super) fn emit_variant_tag_load(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the selected variant layout
        let Some(mir::PlaceType::Value(variant_type)) =
            place.ty(self.function_id, &self.optimized.tree)
        else {
            return Err(self.invalid("native variant tag load does not select a value"));
        };
        let variant_type = self.optimized.tree.storage_type(variant_type);
        let layout = self
            .optimized
            .layouts
            .type_layout(variant_type)
            .ok_or_else(|| self.invalid("stored native variant has no layout"))?;
        let mir::LayoutShape::Variant(layout) = &layout.shape else {
            return Err(self.invalid("stored native value has no variant layout"));
        };
        let address = self.place_pointer(place, builder)?;
        let scalar = self.load_discriminant(address, layout.encoding.field(), builder)?;
        let destination_type = self
            .types
            .value(self.value_type(destination)?)?
            .direct()
            .ok_or_else(|| self.invalid("native variant tag result is not scalar"))?;
        let tag = self.decode_variant(scalar, layout, destination_type, builder)?;
        self.set(destination, Value::Direct(tag))?;

        Ok(())
    }

    /// Emit one statically selected variant payload.
    pub(super) fn emit_variant_payload(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
        case: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // resolve the variant storage type
        let variant_type = self.optimized.tree.storage_type(self.value_type(variant)?);
        let layout = self
            .optimized
            .layouts
            .type_layout(variant_type)
            .ok_or_else(|| self.invalid("native variant has no layout"))?;
        let mir::LayoutShape::Variant(layout) = &layout.shape else {
            return Err(self.invalid("native variant value has no variant layout"));
        };
        let selected = layout
            .cases
            .get(case as usize)
            .ok_or_else(|| self.invalid("native variant case is absent"))?;
        let variant_value_type = self.types.value(variant_type)?;
        let variant = self.value(variant)?;
        let address = self.materialize(variant, variant_value_type, builder)?;
        let address = builder
            .ins()
            .iadd_imm_u(address, i64::from(selected.payload_offset));
        let result_type = self.types.value(self.value_type(destination)?)?;
        let result = self.load(address, result_type, builder)?;
        self.set(destination, result)?;

        Ok(())
    }

    /// Encode one logical case into its physical discriminant field.
    fn encode_variant(
        &self,
        address: cir::Value,
        layout: &mir::VariantLayout,
        case: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let field = layout.encoding.field();
        let ty = self.discriminant_type(field)?;
        let scalar = self.load_discriminant(address, field, builder)?;
        let encoded = match layout.encoding {
            mir::VariantEncoding::Direct { .. } => layout.cases[case as usize].discriminant.bits(),
            mir::VariantEncoding::Niche { untagged_case, .. } if case == untagged_case => {
                return Ok(());
            }
            mir::VariantEncoding::Niche {
                untagged_case,
                niche_start,
                ..
            } => {
                let relative = case - u32::from(case > untagged_case);

                niche_start.bits() + u128::from(relative)
            }
        };
        let mask = self.emit_integer_constant(ty, field.mask(), builder)?;
        let mask = builder.ins().bnot(mask);
        let preserved = builder.ins().band(scalar, mask);
        let encoded =
            self.emit_integer_constant(ty, (encoded << field.bit_offset) & field.mask(), builder)?;
        let scalar = builder.ins().bor(preserved, encoded);
        let target = builder.ins().iadd_imm_u(address, i64::from(field.offset));
        builder
            .ins()
            .store(cir::MemFlagsData::trusted(), scalar, target, 0);

        Ok(())
    }

    /// Decode one physical discriminant into its logical case value.
    fn decode_variant(
        &self,
        scalar: cir::Value,
        layout: &mir::VariantLayout,
        destination: cir::Type,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let field = layout.encoding.field();
        let source = self.discriminant_type(field)?;
        let mask = self.emit_integer_constant(source, field.mask(), builder)?;
        let value = builder.ins().band(scalar, mask);
        let value = builder.ins().ushr_imm_u(value, i64::from(field.bit_offset));
        let mir::VariantEncoding::Niche {
            untagged_case,
            niche_start,
            ..
        } = layout.encoding
        else {
            return Ok(self.convert_discriminant(value, destination, builder));
        };
        let encoded_case_count = (layout.cases.len() - 1) as u32;
        let start = self.emit_integer_constant(source, niche_start.bits(), builder)?;
        let relative = builder.ins().isub(value, start);
        let mask = self.emit_integer_constant(source, field.value_mask(), builder)?;
        let relative = builder.ins().band(relative, mask);
        let in_niche = builder.ins().icmp_imm_u(
            IntCC::UnsignedLessThan,
            relative,
            i64::from(encoded_case_count),
        );
        let untagged = self.emit_integer_constant(source, u128::from(untagged_case), builder)?;
        let follows_untagged =
            builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, relative, untagged);
        let one = self.emit_integer_constant(source, 1, builder)?;
        let zero = self.emit_integer_constant(source, 0, builder)?;
        let adjustment = builder.ins().select(follows_untagged, one, zero);
        let niche_case = builder.ins().iadd(relative, adjustment);
        let untagged = self.emit_integer_constant(source, u128::from(untagged_case), builder)?;
        let case = builder.ins().select(in_niche, niche_case, untagged);

        // map physical case indices to source discriminants
        let mut result = self.emit_integer_constant(
            destination,
            layout.cases[untagged_case as usize].discriminant.bits(),
            builder,
        )?;
        for (index, variant_case) in layout.cases.iter().enumerate() {
            let condition = builder.ins().icmp_imm_u(IntCC::Equal, case, index as i64);
            let discriminant =
                self.emit_integer_constant(destination, variant_case.discriminant.bits(), builder)?;
            result = builder.ins().select(condition, discriminant, result);
        }

        Ok(result)
    }

    /// Load one physical discriminant scalar.
    fn load_discriminant(
        &self,
        address: cir::Value,
        field: mir::DiscriminantField,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = self.discriminant_type(field)?;
        let address = builder.ins().iadd_imm_u(address, i64::from(field.offset));

        let flags = self.memory_flags(AliasRegion::World);

        Ok(builder.ins().load(ty, flags, address, 0))
    }

    /// Return the Cranelift scalar storing one discriminant field.
    fn discriminant_type(&self, field: mir::DiscriminantField) -> Result<cir::Type, EmitError> {
        cir::Type::int(u16::from(field.byte_len) * 8)
            .ok_or_else(|| self.invalid("native discriminant width is unsupported"))
    }

    /// Convert one discriminant scalar into its MIR result width.
    fn convert_discriminant(
        &self,
        value: cir::Value,
        destination: cir::Type,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::Value {
        let source = builder.func.dfg.value_type(value);
        if source.bits() < destination.bits() {
            builder.ins().uextend(destination, value)
        } else if source.bits() > destination.bits() {
            builder.ins().ireduce(destination, value)
        } else {
            value
        }
    }
}
