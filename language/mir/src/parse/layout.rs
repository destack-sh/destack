use destack_core::StringId;

use crate::metadata::{Layout, LayoutField, LayoutKind};
use crate::parse::{ParseError, ParseResult, Parser};
use crate::tree::compute_type_layout;
use crate::{Field, LocalNodeId, Type, TypeReference};

impl Parser {
    /// Record layout metadata for aggregate types parsed from MIR text.
    pub(super) fn record_layout_for_type(&mut self, type_id: LocalNodeId<Type>) -> ParseResult<()> {
        // skip if metadata already exists
        if self.tree.metadata.layout.layout_id(type_id).is_some() {
            return Ok(());
        }

        // dispatch on the parsed aggregate type
        match self.tree.get(type_id) {
            Type::Struct { fields, .. } => {
                let fields = fields.clone();
                self.record_struct_layout(type_id, &fields)
            }
            Type::Tuple { elements, .. } => {
                let elements = elements.clone();
                self.record_tuple_layout(type_id, &elements)
            }
            Type::Array {
                element, length, ..
            } => self.record_array_layout(type_id, *element, *length),
            Type::Closure { signature } => self.record_function_value_layout(type_id, *signature),
            _ => Ok(()),
        }
    }

    /// Record layout metadata for a struct type.
    fn record_struct_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        fields: &[LocalNodeId<Field>],
    ) -> ParseResult<()> {
        // compute field layouts in declaration order
        let mut layout_fields = Vec::with_capacity(fields.len());
        let mut offset = 0u32;
        for (index, field_id) in fields.iter().enumerate() {
            // compute field size and alignment
            let (field_name, field_type) = {
                let field = self.tree.get(*field_id);
                (field.name, field.ty)
            };
            let Some(field_type) = concrete_type(field_type) else {
                return Ok(());
            };
            let field_layout =
                compute_type_layout(&self.tree, field_type, self.tree.pointer_bytes());
            offset = field_layout.align_offset(offset);

            // resolve a stable field name
            let name = field_name.unwrap_or_else(|| self.synthetic_field_name(index));
            layout_fields.push(LayoutField {
                name,
                ty: field_type,
                offset,
                size: field_layout.size,
                alignment: field_layout.alignment,
                source_index: Some(index as u32),
            });
            offset += field_layout.size;
        }

        // compute the final struct size and alignment
        let layout = compute_type_layout(&self.tree, type_id, self.tree.pointer_bytes());

        // assemble the layout table entry
        let layout_entry = Layout {
            kind: LayoutKind::Struct,
            size: layout.size,
            alignment: layout.alignment,
            fields: layout_fields,
        };

        // record the layout entry on metadata
        self.insert_layout_entry(type_id, layout_entry);

        Ok(())
    }

    /// Record layout metadata for a tuple type.
    fn record_tuple_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        elements: &[TypeReference],
    ) -> ParseResult<()> {
        // compute element layouts in order
        let mut layout_fields = Vec::with_capacity(elements.len());
        let mut offset = 0u32;
        let mut max_alignment = 1u32;

        for (index, element_id) in elements.iter().copied().enumerate() {
            let Some(element_id) = concrete_type(element_id) else {
                return Ok(());
            };

            // compute element size and alignment
            let element_layout =
                compute_type_layout(&self.tree, element_id, self.tree.pointer_bytes());

            // align the current offset
            offset = element_layout.align_offset(offset);

            // record the element field
            layout_fields.push(LayoutField {
                name: self.synthetic_tuple_name(index),
                ty: element_id,
                offset,
                size: element_layout.size,
                alignment: element_layout.alignment,
                source_index: Some(index as u32),
            });

            // advance past the element
            offset += element_layout.size;
            max_alignment = max_alignment.max(element_layout.alignment);
        }

        // pad to the final alignment
        let total_size = compute_type_layout(&self.tree, type_id, self.tree.pointer_bytes()).size;

        // assemble the layout table entry
        let layout_entry = Layout {
            kind: LayoutKind::Tuple,
            size: total_size,
            alignment: max_alignment,
            fields: layout_fields,
        };

        // record the layout entry on metadata
        self.insert_layout_entry(type_id, layout_entry);

        Ok(())
    }

    /// Record layout metadata for an array type.
    fn record_array_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        element: TypeReference,
        length: u64,
    ) -> ParseResult<()> {
        let Some(element) = concrete_type(element) else {
            return Ok(());
        };

        // compute element layout
        let element_layout = compute_type_layout(&self.tree, element, self.tree.pointer_bytes());
        let element_stride = self.align_up(element_layout.size, element_layout.alignment);

        // validate the array length
        let length_u32 = u32::try_from(length)
            .map_err(|_| ParseError::invalid("array element count", self.pos()))?;

        // compute array size and alignment
        let size = element_stride
            .checked_mul(length_u32)
            .ok_or_else(|| ParseError::invalid("array layout size overflow", self.pos()))?;
        let alignment = element_layout.alignment;

        // assemble the layout table entry
        let layout_entry = Layout {
            kind: LayoutKind::Array {
                element_type: element,
                element_stride,
                element_count: Some(length_u32),
            },
            size,
            alignment,
            fields: Vec::new(),
        };

        // record the layout entry on metadata
        self.insert_layout_entry(type_id, layout_entry);

        Ok(())
    }

    /// Record layout metadata for a function value type.
    fn record_function_value_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        signature: TypeReference,
    ) -> ParseResult<()> {
        let Some(signature) = concrete_type(signature) else {
            return Ok(());
        };

        let environment = self.tree.ensure_function_value_environment_type();
        let components = [signature, environment];
        let mut layout_fields = Vec::with_capacity(components.len());
        let mut offset = 0u32;
        let mut max_alignment = 1u32;

        // compute component layouts in semantic order
        for (index, component_type) in components.into_iter().enumerate() {
            let component_layout =
                compute_type_layout(&self.tree, component_type, self.tree.pointer_bytes());
            offset = component_layout.align_offset(offset);

            layout_fields.push(LayoutField {
                name: self.synthetic_tuple_name(index),
                ty: component_type,
                offset,
                size: component_layout.size,
                alignment: component_layout.alignment,
                source_index: Some(index as u32),
            });

            offset += component_layout.size;
            max_alignment = max_alignment.max(component_layout.alignment);
        }

        let layout = compute_type_layout(&self.tree, type_id, self.tree.pointer_bytes());
        let layout_entry = Layout {
            kind: LayoutKind::Closure,
            size: layout.size,
            alignment: max_alignment,
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);

        Ok(())
    }

    /// Insert a layout entry and attach it to the type table.
    fn insert_layout_entry(&mut self, type_id: LocalNodeId<Type>, layout: Layout) {
        let layout_id = self.tree.metadata.layout.layout_table.insert(layout);
        self.tree.metadata.layout.set_layout_id(type_id, layout_id);
    }

    /// Build a synthetic field name for unnamed struct fields.
    fn synthetic_field_name(&mut self, index: usize) -> StringId {
        let name = format!("@field{index}");
        self.strings.intern(&name)
    }

    /// Build a synthetic tuple element name.
    fn synthetic_tuple_name(&mut self, index: usize) -> StringId {
        let name = index.to_string();
        self.strings.intern(&name)
    }

    /// Align a size up to a given alignment.
    fn align_up(&self, value: u32, alignment: u32) -> u32 {
        // zero alignment leaves the value unchanged
        if alignment == 0 {
            return value;
        }

        // already aligned values stay put
        let misalignment = value % alignment;
        if misalignment == 0 {
            value
        } else {
            value + (alignment - misalignment)
        }
    }
}

fn concrete_type(reference: TypeReference) -> Option<LocalNodeId<Type>> {
    match reference {
        TypeReference::Type(ty) => Some(ty),
        TypeReference::Missing | TypeReference::Error => None,
    }
}
