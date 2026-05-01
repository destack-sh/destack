use destack_mir as mir;

use crate::program::{ElementAccess, FieldAccess, Layout};
use crate::{Error, Result};

use super::access::{array_length, element_access, field_access, field_count};
use super::lower::BlockLowerer;
use super::repr::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_repr, pointer_class_for_value,
    raw_pointee_type_for_value, raw_pointee_type_for_value_repr,
    value_type_for_value as lookup_value_type_for_value,
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
            .value_repr_map()
            .get(value)
            .and_then(|repr| field_count(self.tree, repr))
        {
            return Ok(count);
        }

        let value_type = self.indexed_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidInstruction);
        };

        self.field_count_for_type(value_type)
    }

    /// Return one lowered array length for one value.
    pub(super) fn array_length_for_value(&self, value: mir::Value) -> Result<u64> {
        if let Some(length) = self
            .value_repr_map()
            .get(value)
            .and_then(|repr| array_length(self.tree, repr))
        {
            return Ok(length);
        }

        let value_type = self.indexed_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidInstruction);
        };

        self.array_length_for_type(value_type)
    }

    /// Return the indexed type behind one value.
    pub(super) fn indexed_type_for_value(
        &self,
        value: mir::Value,
    ) -> Result<Option<mir::LocalNodeId<mir::Type>>> {
        let pointee_type = heap_pointee_type_for_value_repr(self.value_repr_map(), value)
            .or_else(|| raw_pointee_type_for_value_repr(self.value_repr_map(), value))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), value))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), value));

        if pointee_type.is_some() {
            return Ok(pointee_type);
        }

        Ok(Some(self.value_type_for_value(value)?))
    }

    /// Return one lowered field access for a value.
    pub(super) fn field_access_for_value(
        &self,
        value: mir::Value,
        index: u32,
    ) -> Result<FieldAccess> {
        let field_count = self.field_count_for_value(value)? as usize;
        let value_type = self.indexed_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidFieldAccess { index, field_count });
        };
        let pointer_class = pointer_class_for_value(self.value_repr_map(), value);

        field_access(self.tree, self.layouts(), value_type, pointer_class, index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })
    }

    /// Return one lowered element access for a value.
    pub(super) fn element_access_for_value(&self, value: mir::Value) -> Result<ElementAccess> {
        let value_type = self.indexed_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(Error::InvalidInstruction);
        };
        let pointer_class = pointer_class_for_value(self.value_repr_map(), value);

        element_access(self.tree, self.layouts(), value_type, pointer_class)
            .ok_or(Error::InvalidInstruction)
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

    /// Return one lowered byte length for one layout.
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

    /// Return one lowered byte length for one layout.
    pub(super) fn layout_byte_len(&self, layout: mir::LocalNodeId<mir::Type>) -> Result<usize> {
        self.layouts()
            .get(&layout)
            .map(|layout| layout.byte_len)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing lowered layout for type: {layout:?}"),
            })
    }
}
