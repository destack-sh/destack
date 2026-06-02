use destack_mir as mir;

use crate::program::{Layout, Projection};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::projection::{
    array_element_count, element_projection, field_count_for_layout, field_projection,
};
use super::value::{
    heap_pointee_type_for_storage_id, heap_pointee_type_for_value, raw_pointee_type_for_storage_id,
    raw_pointee_type_for_value, value_type_for_value as lookup_value_type_for_value,
};

impl<'a> BlockLowerer<'a> {
    /// Return one lowered layout by MIR type.
    pub(super) fn layout_for_type(
        &self,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<&Layout> {
        self.layouts().get(&value_type).ok_or_else(|| {
            Error::internal(format!("missing lowered layout for type: {value_type:?}"))
        })
    }

    /// Return one lowered field count for one value.
    pub(super) fn field_count_for_value(&self, value: mir::Value) -> Result<u32> {
        if let Some(count) = self
            .value_shape_map()
            .get(value)
            .and_then(|layout| field_count_for_layout(self.tree, layout))
        {
            return Ok(count);
        }

        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::invalid_instruction());
        };

        self.field_count_for_type(value_type)
    }

    /// Return one lowered array length for one value.
    pub(super) fn array_length_for_value(&self, value: mir::Value) -> Result<u64> {
        if let Some(length) = self
            .value_shape_map()
            .get(value)
            .and_then(|layout| array_element_count(self.tree, layout))
        {
            return Ok(length);
        }

        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::invalid_instruction());
        };

        self.array_length_for_type(value_type)
    }

    /// Return the type projected by one value.
    pub(super) fn projection_type_for_value(
        &self,
        value: mir::Value,
    ) -> Result<Option<mir::LocalNodeId<mir::Type>>> {
        let pointee_type = heap_pointee_type_for_storage_id(self.value_shape_map(), value)
            .or_else(|| raw_pointee_type_for_storage_id(self.value_shape_map(), value))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), value))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), value));

        if pointee_type.is_some() {
            return Ok(pointee_type);
        }

        Ok(Some(self.value_type_for_value(value)?))
    }

    /// Return one lowered field projection for a value.
    pub(super) fn field_projection_for_value(
        &self,
        value: mir::Value,
        index: u32,
    ) -> Result<Projection> {
        let field_count = self.field_count_for_value(value)? as usize;
        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::invalid_field_access(index, field_count));
        };
        field_projection(self.tree, self.layouts(), value_type, index)
            .ok_or(Error::invalid_field_access(index, field_count))
    }

    /// Return one lowered element projection for a value.
    pub(super) fn element_projection_for_value(&self, value: mir::Value) -> Result<Projection> {
        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::invalid_instruction());
        };
        element_projection(self.tree, self.layouts(), value_type)
            .ok_or(Error::invalid_instruction())
    }

    /// Return one field count from a concrete type.
    fn field_count_for_type(&self, value_type: mir::LocalNodeId<mir::Type>) -> Result<u32> {
        match self.tree.get(value_type) {
            mir::Type::Struct { fields, copy: _ } => {
                u32::try_from(fields.len()).map_err(|_| Error::invalid_instruction())
            }
            mir::Type::Tuple { elements, copy: _ } => {
                u32::try_from(elements.len()).map_err(|_| Error::invalid_instruction())
            }
            _ => Err(Error::invalid_instruction()),
        }
    }

    /// Return one array length from a concrete type.
    fn array_length_for_type(&self, value_type: mir::LocalNodeId<mir::Type>) -> Result<u64> {
        match self.tree.get(value_type) {
            mir::Type::Array { length, .. } => Ok(*length),
            _ => Err(Error::invalid_instruction()),
        }
    }

    /// Return the MIR type for one value.
    pub(super) fn value_type_for_value(
        &self,
        value: mir::Value,
    ) -> Result<mir::LocalNodeId<mir::Type>> {
        lookup_value_type_for_value(value, self.value_type())
            .ok_or_else(|| Error::internal(format!("missing value type for {value:?}")))
    }
}
