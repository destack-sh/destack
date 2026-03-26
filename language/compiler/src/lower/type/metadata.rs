use std::collections::HashMap;

use destack_core::StringId;
use destack_dir::AnchoredGlobalNodeId;
use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{FieldLayoutKind, LowerError, LowerResult, StructLayout};

impl ModuleLowerer<'_> {
    /// Return layout metadata for a cached aggregate layout.
    pub(crate) fn layout_metadata_for_type(
        &mut self,
        type_id: dir::LocalTypeId,
        ty: mir::LocalNodeId<mir::Type>,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LayoutId>> {
        // skip if metadata already exists
        if let Some(layout_id) = self.builder.tree().type_table.layout_id(ty) {
            return Ok(Some(layout_id));
        }

        // resolve the cached struct layout when available
        if let Some(layout) = self.type_lowerer.layout_for_type(ty).cloned() {
            let layout_type = self.layout_type_for_type(type_id, ty, anchor, &layout)?;
            let layout_id = self.insert_layout_entry(ty, layout_type, &layout);

            return Ok(Some(layout_id));
        }

        // resolve the tuple or array shape for layout
        let target = match self.builder.tree().get(ty) {
            mir::Type::Tuple {
                elements,
                copyability: _,
            } => Some(LayoutTarget::Tuple(elements.clone())),
            mir::Type::Array {
                element,
                length,
                copyability: _,
            } => Some(LayoutTarget::Array(*element, *length)),
            _ => None,
        };
        let Some(target) = target else {
            return Ok(None);
        };

        // materialize the concrete layout metadata
        match target {
            LayoutTarget::Tuple(elements) => {
                let (fields, size, alignment) = self.tuple_layout_fields(&elements);
                let layout_id = self.insert_layout_metadata(
                    ty,
                    mir::LayoutType::Tuple,
                    size,
                    alignment,
                    fields,
                );

                Ok(Some(layout_id))
            }
            LayoutTarget::Array(element, length) => {
                let (layout_type, size, alignment) =
                    self.array_layout_info(type_id, element, length, anchor)?;
                let layout_id =
                    self.insert_layout_metadata(ty, layout_type, size, alignment, Vec::new());

                Ok(Some(layout_id))
            }
        }
    }

    /// Insert a concrete layout entry and attach it to the type metadata.
    pub(crate) fn insert_layout_entry(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        layout_type: mir::LayoutType,
        layout: &StructLayout,
    ) -> mir::LayoutId {
        // build the layout fields in order
        let mut fields = Vec::with_capacity(layout.fields.len());
        for field in &layout.fields {
            fields.push(mir::LayoutField {
                name: field.name,
                ty: field.ty,
                offset: field.offset,
                size: field.size,
                alignment: field.alignment,
                source_index: field.source_index,
            });
        }

        self.insert_layout_metadata(ty, layout_type, layout.size, layout.alignment, fields)
    }

    /// Insert a layout entry and attach it to the type metadata.
    fn insert_layout_metadata(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        layout_type: mir::LayoutType,
        size: u32,
        alignment: u32,
        fields: Vec<mir::LayoutField>,
    ) -> mir::LayoutId {
        // build the mir layout entry
        let layout_entry = mir::Layout {
            layout_type,
            size,
            alignment,
            fields,
        };

        // attach layout metadata to the type table
        let type_table = &mut self.builder.tree_mut().type_table;
        let layout_id = type_table.layout_table.insert(layout_entry);
        type_table.set_layout_id(ty, layout_id);

        layout_id
    }

    /// Compute layout fields for a tuple type.
    fn tuple_layout_fields(
        &mut self,
        elements: &[mir::LocalNodeId<mir::Type>],
    ) -> (Vec<mir::LayoutField>, u32, u32) {
        // compute element layouts in order
        let mut fields = Vec::with_capacity(elements.len());
        let mut offset = 0u32;
        let mut max_alignment = 1u32;

        for (index, element) in elements.iter().enumerate() {
            // compute element size and alignment
            let element_type = self.builder.tree().get(*element);
            let (size, alignment) = self
                .type_lowerer
                .size_and_align_of_type(element_type, self.builder.tree());

            // align the current offset
            offset = align_up(offset, alignment);

            // record the element layout
            let name = self.builder.intern(&index.to_string());
            fields.push(mir::LayoutField {
                name,
                ty: *element,
                offset,
                size,
                alignment,
                source_index: Some(index as u32),
            });

            // advance past the element
            offset = offset.saturating_add(size);
            max_alignment = max_alignment.max(alignment);
        }

        // pad to alignment
        let size = align_up(offset, max_alignment);

        (fields, size, max_alignment)
    }

    /// Compute layout info for an array type.
    fn array_layout_info(
        &self,
        type_id: dir::LocalTypeId,
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<(mir::LayoutType, u32, u32)> {
        // compute element size and alignment
        let element_type = self.builder.tree().get(element);
        let (size, alignment) = self
            .type_lowerer
            .size_and_align_of_type(element_type, self.builder.tree());
        let stride = align_up(size, alignment);

        // validate the array length
        let length_u32 = u32::try_from(length).map_err(|_| LowerError::UnsupportedType {
            node: anchor,
            ty: type_id.into_global(self.module_id),
            message: "array length exceeds layout limits".to_string(),
        })?;
        let total_size =
            stride
                .checked_mul(length_u32)
                .ok_or_else(|| LowerError::UnsupportedType {
                    node: anchor,
                    ty: type_id.into_global(self.module_id),
                    message: "array layout size overflow".to_string(),
                })?;

        Ok((
            mir::LayoutType::Array {
                element_type: element,
                element_stride: stride,
                element_count: Some(length_u32),
            },
            total_size,
            alignment,
        ))
    }

    /// Resolve the layout kind for a lowered type.
    fn layout_type_for_type(
        &self,
        type_id: dir::LocalTypeId,
        mir_type: mir::LocalNodeId<mir::Type>,
        anchor: AnchoredGlobalNodeId,
        layout: &StructLayout,
    ) -> LowerResult<mir::LayoutType> {
        // prefer union layouts when present
        let layout_type = if let Some(union_layout) =
            self.builder.tree().type_table.union_layout(mir_type)
        {
            let tag_index = layout
                .field_index(union_layout.tag_field_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "missing union tag field".to_string(),
                })?;
            let tag_field =
                layout
                    .field(tag_index)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: anchor,
                        message: "missing union tag field".to_string(),
                    })?;
            let payload_index = layout
                .field_index(union_layout.payload_field_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "missing union payload field".to_string(),
                })?;
            let payload_field =
                layout
                    .field(payload_index)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: anchor,
                        message: "missing union payload field".to_string(),
                    })?;

            mir::LayoutType::Union {
                tag_type: union_layout.tag_type,
                tag_offset: tag_field.offset,
                payload_offset: payload_field.offset,
            }
        }
        // prefer interface layouts when present
        else if let Some(interface_layout) = self.type_lowerer.interface_ref_layout(type_id) {
            let object_field = layout
                .field(interface_layout.object_field_index)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "missing interface object field".to_string(),
                })?;
            let itab_field = layout
                .field(interface_layout.itab_field_index)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "missing interface table field".to_string(),
                })?;

            mir::LayoutType::Interface {
                object_offset: object_field.offset,
                table_offset: itab_field.offset,
            }
        }
        // use function value layouts for function types
        else if matches!(self.types.get_type(type_id), dir::Type::Function { .. }) {
            mir::LayoutType::FunctionValue
        }
        // mark function environments explicitly when present
        else if self
            .function_environment_layouts
            .values()
            .any(|env_layout| env_layout.env_type == mir_type)
        {
            mir::LayoutType::FunctionEnvironment
        }
        // default to plain struct layout
        else {
            mir::LayoutType::Struct
        };

        Ok(layout_type)
    }

    /// Return field map metadata for nominal struct and class layouts.
    pub(crate) fn field_map_metadata_for_type(
        &mut self,
        type_id: dir::LocalTypeId,
        mir_type: mir::LocalNodeId<mir::Type>,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<Option<HashMap<StringId, mir::LocalNodeId<mir::Field>>>> {
        // restrict field maps to struct and class payloads
        let Some(symbol) = self.types.symbol_for_instance_type(type_id) else {
            return Ok(None);
        };
        if !matches!(
            symbol.ty(),
            dir::SymbolType::Struct | dir::SymbolType::Class
        ) {
            return Ok(None);
        }

        // skip if metadata already exists
        if let Some(field_map) = self.builder.tree().type_table.field_map(mir_type)
            && !field_map.is_empty()
        {
            return Ok(Some(field_map.clone()));
        }

        // require a struct layout for field map generation
        let mir_type_node = self.builder.tree().get(mir_type);
        let mir::Type::Struct { fields, .. } = mir_type_node else {
            return Ok(None);
        };

        // resolve the cached layout for the struct
        let layout = self
            .type_lowerer
            .layout_for_type_or_error(mir_type, anchor)?;

        // reject layout size mismatches
        if fields.len() != layout.fields.len() {
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "field map layout size mismatch".to_string(),
            });
        }

        // build the field map from source field indices
        let mut field_map = HashMap::new();
        for (index, field) in layout.fields.iter().enumerate() {
            // skip synthetic fields
            if field.kind != FieldLayoutKind::Source {
                continue;
            }

            let field_id = fields[index];
            field_map.insert(field.name, field_id);
        }

        // record the field map metadata
        let type_table = &mut self.builder.tree_mut().type_table;
        type_table.set_field_map(mir_type, field_map.clone());

        Ok(Some(field_map))
    }
}

/// Aggregate type variants that need layout metadata.
enum LayoutTarget {
    /// Tuple element types in order.
    Tuple(Vec<mir::LocalNodeId<mir::Type>>),
    /// Array element type and count.
    Array(mir::LocalNodeId<mir::Type>, u64),
}

/// Align a value up to the given alignment.
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
