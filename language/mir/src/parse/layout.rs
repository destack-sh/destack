use destack_base::StringId;

use crate::metadata::{Layout, LayoutField, LayoutType};
use crate::parse::{ParseError, ParseResult, Parser};
use crate::tree::compute_type_layout;
use crate::{Field, LocalNodeId, Type};

impl Parser<'_> {
    /// Record layout metadata for aggregate types parsed from MIR text.
    pub(super) fn record_layout_for_type(&mut self, type_id: LocalNodeId<Type>) -> ParseResult<()> {
        // skip if metadata already exists
        if let Some(metadata) = self.tree.type_table.type_metadata_by_id.get(&type_id)
            && metadata.layout_id.is_some()
        {
            return Ok(());
        }

        // only aggregate types get layout metadata
        enum LayoutTarget {
            Struct(Vec<LocalNodeId<Field>>),
            Tuple(Vec<LocalNodeId<Type>>),
            Array(LocalNodeId<Type>, u64),
        }

        let target = match self.tree.get(type_id) {
            Type::Struct { fields, .. } => LayoutTarget::Struct(fields.clone()),
            Type::Tuple { elements, .. } => LayoutTarget::Tuple(elements.clone()),
            Type::Array {
                element, length, ..
            } => LayoutTarget::Array(*element, *length),
            _ => return Ok(()),
        };

        match target {
            LayoutTarget::Struct(fields) => self.record_struct_layout(type_id, &fields),
            LayoutTarget::Tuple(elements) => self.record_tuple_layout(type_id, &elements),
            LayoutTarget::Array(element, length) => {
                self.record_array_layout(type_id, element, length)
            }
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
        for (index, field_id) in fields.iter().enumerate() {
            // compute field size and alignment
            let (field_name, field_type, field_offset) = {
                let field = self.tree.get(*field_id);
                (field.name, field.ty, field.offset)
            };
            let field_layout =
                compute_type_layout(&self.tree, field_type, self.options.pointer_bytes);

            // resolve a stable field name
            let name = field_name.unwrap_or_else(|| self.synthetic_field_name(index));
            layout_fields.push(LayoutField {
                name,
                ty: field_type,
                offset: field_offset,
                size: field_layout.size,
                alignment: field_layout.alignment,
                source_index: Some(index as u32),
            });
        }

        // compute the final struct size and alignment
        let layout = compute_type_layout(&self.tree, type_id, self.options.pointer_bytes);

        // assemble the layout table entry
        let layout_entry = Layout {
            layout_type: LayoutType::Struct,
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
        elements: &[LocalNodeId<Type>],
    ) -> ParseResult<()> {
        // compute element layouts in order
        let mut layout_fields = Vec::with_capacity(elements.len());
        let mut offset = 0u32;
        let mut max_alignment = 1u32;

        for (index, element_id) in elements.iter().enumerate() {
            // compute element size and alignment
            let element_layout =
                compute_type_layout(&self.tree, *element_id, self.options.pointer_bytes);

            // align the current offset
            offset = element_layout.align_offset(offset);

            // record the element field
            layout_fields.push(LayoutField {
                name: self.synthetic_tuple_name(index),
                ty: *element_id,
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
        let total_size = compute_type_layout(&self.tree, type_id, self.options.pointer_bytes).size;

        // assemble the layout table entry
        let layout_entry = Layout {
            layout_type: LayoutType::Tuple,
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
        element: LocalNodeId<Type>,
        length: u64,
    ) -> ParseResult<()> {
        // compute element layout
        let element_layout = compute_type_layout(&self.tree, element, self.options.pointer_bytes);
        let element_stride = align_up(element_layout.size, element_layout.alignment);

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
            layout_type: LayoutType::Array {
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

    /// Insert a layout entry and attach it to the type metadata.
    fn insert_layout_entry(&mut self, type_id: LocalNodeId<Type>, layout: Layout) {
        let layout_id = self.tree.type_table.layout_table.insert(layout);
        let metadata = self
            .tree
            .type_table
            .type_metadata_by_id
            .entry(type_id)
            .or_default();
        metadata.layout_id = Some(layout_id);
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
}

/// Align a size up to a given alignment.
fn align_up(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return value;
    }
    let misalignment = value % alignment;
    if misalignment == 0 {
        value
    } else {
        value + (alignment - misalignment)
    }
}
