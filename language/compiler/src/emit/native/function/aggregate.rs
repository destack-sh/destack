use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{FunctionEmitter, Value};

impl<'a> FunctionEmitter<'a> {
    /// Emit one aggregate into canonical stack storage.
    pub(super) fn emit_aggregate(
        &mut self,
        destination: mir::Value,
        values: mir::ValueSlice,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the destination type and aggregate layout
        let ty = self.value_type(destination)?;
        let layout = self
            .optimized
            .layouts
            .type_layout(ty)
            .ok_or_else(|| self.invalid("native aggregate has no layout"))?;
        let value_type = self.types.value(ty)?;
        let source = self.optimized.tree.get_values(values);

        // retain direct aggregates entirely in SSA
        if matches!(value_type, ValueType::Direct { .. }) {
            let [source] = source else {
                return Err(self.invalid("native direct aggregate does not have one value"));
            };
            self.aggregate_field(layout, 0)?;
            let source = self.value(*source)?;
            self.set(destination, source)?;

            return Ok(());
        }

        // place scalar-pair sources by their canonical offsets
        if let Some(fields) = value_type.scalar_pair() {
            let mut values = [None, None];
            for (source_index, source) in source.iter().enumerate() {
                let (_, offset) = self.aggregate_field(layout, source_index as u32)?;
                let field_index = fields
                    .iter()
                    .position(|field| field.offset == offset)
                    .ok_or_else(|| self.invalid("native scalar pair has no source field"))?;
                values[field_index] = Some(self.scalar(*source)?);
            }
            let [Some(first), Some(second)] = values else {
                return Err(self.invalid("native scalar pair is missing a source value"));
            };
            self.set(destination, Value::ScalarPair([first, second]))?;

            return Ok(());
        }

        // materialize larger aggregates in canonical memory
        let address = self.allocate(value_type, builder);
        self.zero_padding(address, layout, builder)?;

        // place source values into canonical aggregate offsets
        for (index, value) in source.iter().enumerate() {
            let (ty, offset) = self.aggregate_field(layout, index as u32)?;
            let target = builder.ins().iadd_imm_u(address, i64::from(offset));
            let value_type = self.types.value(ty)?;
            let value = self.value(*value)?;
            self.store(target, value, value_type, builder)?;
        }
        self.set(destination, Value::Address(address))?;

        Ok(())
    }

    /// Emit one aggregate field or fixed element projection.
    pub(super) fn emit_projection(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        index: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the aggregate layout and value
        let aggregate_type = self.value_type(aggregate)?;
        let layout = self
            .optimized
            .layouts
            .type_layout(aggregate_type)
            .ok_or_else(|| self.invalid("native aggregate has no layout"))?;
        let aggregate_value = self.value(aggregate)?;

        // project the sole field from one direct aggregate
        if let Value::Direct(value) = aggregate_value {
            self.aggregate_field(layout, index)?;
            self.set(destination, Value::Direct(value))?;

            return Ok(());
        }

        // project scalar-pair fields without materializing canonical memory
        if let Value::ScalarPair(values) = aggregate_value {
            let value_type = self.types.value(aggregate_type)?;
            let fields = value_type
                .scalar_pair()
                .ok_or_else(|| self.invalid("native scalar pair has no field layout"))?;
            let (_, offset) = self.aggregate_field(layout, index)?;
            let field = fields
                .iter()
                .position(|field| field.offset == offset)
                .ok_or_else(|| self.invalid("native scalar pair has no projected field"))?;
            self.set(destination, Value::Direct(values[field]))?;

            return Ok(());
        }

        // load indirect projections from canonical memory
        let offset = if let Some(field) = layout.source_field(index) {
            field.offset
        } else if let Some(elements) = layout.element() {
            elements.stride * index
        } else {
            return Err(self.invalid("native projection has no physical placement"));
        };
        let address = builder.ins().iadd_imm_u(
            aggregate_value
                .address()
                .ok_or_else(|| self.invalid("native aggregate is not indirect"))?,
            i64::from(offset),
        );
        let value_type = self.types.value(self.value_type(destination)?)?;
        let value = self.load(address, value_type, builder)?;
        self.set(destination, value)?;

        Ok(())
    }

    /// Emit one structural field replacement.
    pub(super) fn emit_field_set(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        field: u32,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // resolve the field offset in the aggregate layout
        let aggregate_type = self.value_type(aggregate)?;
        let offset = self
            .optimized
            .layouts
            .type_layout(aggregate_type)
            .and_then(|layout| layout.source_field(field))
            .map(|field| field.offset)
            .ok_or_else(|| self.invalid("native field has no layout"))?;

        self.emit_insert(destination, aggregate, offset, value, builder)
    }

    /// Emit one fixed-array element replacement.
    pub(super) fn emit_element_set(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the indexed aggregate layout
        let aggregate_type = self.value_type(aggregate)?;
        let offset = self
            .optimized
            .layouts
            .type_layout(aggregate_type)
            .and_then(|layout| layout.element())
            .map(|element| element.stride * index)
            .ok_or_else(|| self.invalid("native fixed array has no element layout"))?;

        self.emit_insert(destination, aggregate, offset, value, builder)
    }

    /// Emit one immutable aggregate update.
    fn emit_insert(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        offset: u32,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let value_type = self.types.value(aggregate_type)?;

        // replace the sole field of one direct aggregate
        if matches!(self.value(aggregate)?, Value::Direct(_)) {
            if offset != 0 {
                return Err(self.invalid("native direct aggregate field is not at byte zero"));
            }
            let value = self.value(value)?;
            self.set(destination, value)?;

            return Ok(());
        }

        // update two-scalar aggregates entirely in SSA
        if let Value::ScalarPair(mut values) = self.value(aggregate)? {
            let fields = value_type
                .scalar_pair()
                .ok_or_else(|| self.invalid("native scalar pair has no field layout"))?;
            let field = fields
                .iter()
                .position(|field| field.offset == offset)
                .ok_or_else(|| self.invalid("native scalar pair has no inserted field"))?;
            values[field] = self.scalar(value)?;
            self.set(destination, Value::ScalarPair(values))?;

            return Ok(());
        }

        // copy indirect aggregates before replacing one field
        let address = self.allocate(value_type, builder);
        let aggregate = self.value(aggregate)?;
        self.store(address, aggregate, value_type, builder)?;
        let target = builder.ins().iadd_imm_u(address, i64::from(offset));
        let field_type = self.types.value(self.value_type(value)?)?;
        let value = self.value(value)?;
        self.store(target, value, field_type, builder)?;
        self.set(destination, Value::Address(address))?;

        Ok(())
    }

    /// Return one aggregate source's type and canonical byte offset.
    fn aggregate_field(
        &self,
        layout: &mir::Layout,
        index: u32,
    ) -> Result<(mir::TypeId, u32), EmitError> {
        // select the named or tuple field by its source index
        if let Some(field) = layout.source_field(index) {
            return Ok((field.ty, field.offset));
        }

        // multiply the array index by the element stride
        if let Some(element) = layout.element().filter(|element| index < element.count) {
            return Ok((element.element, element.stride * index));
        }

        // place the newtype's backing value at byte zero
        if let mir::LayoutShape::Newtype(newtype) = layout.shape
            && index == 0
        {
            return Ok((newtype.backing_type, 0));
        }

        Err(self.invalid("native aggregate source has no physical field"))
    }

    /// Initialize only the padding bytes in one aggregate layout.
    fn zero_padding(
        &self,
        address: cranelift_codegen::ir::Value,
        layout: &mir::Layout,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // collect the occupied field ranges
        let mut fields = layout
            .shape
            .fields()
            .iter()
            .map(|field| (field.offset, field.size))
            .collect::<Vec<_>>();

        // expand fixed arrays and transparent newtypes into occupied byte ranges
        if let Some(elements) = layout.element() {
            let element_size = self
                .optimized
                .layouts
                .type_layout(elements.element)
                .map(|layout| layout.size)
                .ok_or_else(|| self.invalid("native array element has no layout"))?;
            fields.extend((0..elements.count).map(|index| (index * elements.stride, element_size)));
        } else if matches!(layout.shape, mir::LayoutShape::Newtype(_)) {
            fields.push((0, layout.size));
        }
        fields.sort_unstable_by_key(|(offset, _)| *offset);

        // clear each gap and the trailing padding
        let mut end = 0;
        for (offset, size) in fields {
            if end < offset {
                self.zero_range(address, end, offset - end, builder);
            }
            end = end.max(offset + size);
        }
        if end < layout.size {
            self.zero_range(address, end, layout.size - end, builder);
        }

        Ok(())
    }
}
