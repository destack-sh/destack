use destack_mir as mir;

use crate::program::{Layout, Projection};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::projection::{
    array_element_count, element_projection, field_count_for_layout, field_projection,
};
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, raw_pointee_type_for_value,
    raw_pointee_type_for_value_layout, value_type_for_value as lookup_value_type_for_value,
};

impl<'a> BlockLowerer<'a> {
    /// Return one lowered layout by MIR type.
    pub(super) fn layout_for_type(
        &self,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<&Layout> {
        self.layouts()
            .get(&value_type)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing lowered layout for type: {value_type:?}"),
            })
    }

    /// Return one lowered field count for one value.
    pub(super) fn field_count_for_value(&self, value: mir::Value) -> Result<u32> {
        if let Some(count) = self
            .value_layout_map()
            .get(value)
            .and_then(|layout| field_count_for_layout(self.tree, layout))
        {
            return Ok(count);
        }

        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidInstruction);
        };

        self.field_count_for_type(value_type)
    }

    /// Return one lowered array length for one value.
    pub(super) fn array_length_for_value(&self, value: mir::Value) -> Result<u64> {
        if let Some(length) = self
            .value_layout_map()
            .get(value)
            .and_then(|layout| array_element_count(self.tree, layout))
        {
            return Ok(length);
        }

        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidInstruction);
        };

        self.array_length_for_type(value_type)
    }

    /// Return the type projected by one value.
    pub(super) fn projection_type_for_value(
        &self,
        value: mir::Value,
    ) -> Result<Option<mir::LocalNodeId<mir::Type>>> {
        let pointee_type = heap_pointee_type_for_value_layout(self.value_layout_map(), value)
            .or_else(|| raw_pointee_type_for_value_layout(self.value_layout_map(), value))
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
            return Err(Error::InvalidFieldAccess { index, field_count });
        };
        field_projection(self.tree, self.layouts(), value_type, index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })
    }

    /// Return one lowered element projection for a value.
    pub(super) fn element_projection_for_value(&self, value: mir::Value) -> Result<Projection> {
        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidInstruction);
        };
        element_projection(self.tree, self.layouts(), value_type).ok_or(Error::InvalidInstruction)
    }

    /// Return one field count from a concrete type.
    fn field_count_for_type(&self, value_type: mir::LocalNodeId<mir::Type>) -> Result<u32> {
        match self.tree.get(value_type) {
            mir::Type::Struct { fields, copy: _ } => {
                u32::try_from(fields.len()).map_err(|_| Error::InvalidInstruction)
            }
            mir::Type::Tuple { elements, copy: _ } => {
                u32::try_from(elements.len()).map_err(|_| Error::InvalidInstruction)
            }
            _ => Err(Error::InvalidInstruction),
        }
    }

    /// Return one array length from a concrete type.
    fn array_length_for_type(&self, value_type: mir::LocalNodeId<mir::Type>) -> Result<u64> {
        match self.tree.get(value_type) {
            mir::Type::Array { length, .. } => Ok(*length),
            _ => Err(Error::InvalidInstruction),
        }
    }

    /// Return the MIR type for one value.
    pub(super) fn value_type_for_value(
        &self,
        value: mir::Value,
    ) -> Result<mir::LocalNodeId<mir::Type>> {
        lookup_value_type_for_value(value, self.value_type()).ok_or_else(|| {
            Error::InvariantViolation {
                context: format!("missing value type for {value:?}"),
            }
        })
    }

    /// Return the lowered byte length for one type.
    pub(super) fn byte_len_for_type(&self, layout: mir::LocalNodeId<mir::Type>) -> Result<usize> {
        self.layouts()
            .get(&layout)
            .map(|layout| layout.byte_len)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing lowered layout for type: {layout:?}"),
            })
    }
}
