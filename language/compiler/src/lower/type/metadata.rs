use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{FieldLayoutKind, LowerError, LowerResult, StructLayout};

impl ModuleLowerer<'_> {
    /// Return layout metadata for a cached aggregate layout.
    pub(crate) fn layout_metadata_for_type(
        &mut self,
        type_id: dir::LocalTypeId,
        ty: mir::LocalNodeId<mir::Type>,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LayoutId>> {
        self.layout_metadata_for_mir_type_with_source(Some(type_id), ty, anchor)
    }

    /// Return layout metadata for one MIR type regardless of DIR source.
    pub(crate) fn layout_metadata_for_mir_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LayoutId>> {
        self.layout_metadata_for_mir_type_with_source(None, ty, anchor)
    }

    /// Return layout metadata for one MIR type with an optional DIR source.
    fn layout_metadata_for_mir_type_with_source(
        &mut self,
        type_id: Option<dir::LocalTypeId>,
        ty: mir::LocalNodeId<mir::Type>,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LayoutId>> {
        // skip if metadata already exists
        if let Some(layout_id) = self.builder.tree().metadata.layout.layout_id(ty) {
            return Ok(Some(layout_id));
        }

        // resolve the cached struct layout when available
        if let Some(layout) = self.type_lowerer.layout_for_type(ty).cloned() {
            let layout_shape = self.layout_shape_for_mir_type(type_id, ty, anchor, &layout)?;
            let layout_id = self.insert_layout_entry(ty, layout_shape, &layout);

            return Ok(Some(layout_id));
        }

        // resolve the tuple or array shape for layout
        let target = match self.builder.tree().get(ty) {
            mir::Type::Tuple { elements, copy: _ } => {
                let Some(elements) = elements
                    .iter()
                    .map(|element| element.ty())
                    .collect::<Option<Vec<_>>>()
                else {
                    return Ok(None);
                };

                Some(LayoutTarget::Tuple(elements))
            }
            mir::Type::Array {
                element,
                length,
                copy: _,
            } => {
                let Some(element) = element.ty() else {
                    return Ok(None);
                };

                Some(LayoutTarget::Array(element, *length))
            }
            _ => None,
        };
        let Some(target) = target else {
            return Ok(None);
        };

        // materialize the concrete layout metadata
        match target {
            LayoutTarget::Tuple(elements) => {
                let Some((fields, size, alignment)) = self.tuple_layout_fields(&elements) else {
                    return Ok(None);
                };
                let layout_id = self.insert_layout_metadata(
                    ty,
                    mir::LayoutShape::Tuple,
                    size,
                    alignment,
                    fields,
                );

                Ok(Some(layout_id))
            }
            LayoutTarget::Array(element, length) => {
                let (layout_shape, size, alignment) =
                    self.array_layout_info(type_id, element, length, anchor)?;
                let layout_id =
                    self.insert_layout_metadata(ty, layout_shape, size, alignment, Vec::new());

                Ok(Some(layout_id))
            }
        }
    }

    /// Insert a concrete layout entry and attach it to the type metadata.
    pub(crate) fn insert_layout_entry(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        layout_shape: mir::LayoutShape,
        layout: &StructLayout,
    ) -> mir::LayoutId {
        // build the layout fields in order
        let mut fields = Vec::with_capacity(layout.fields.len());
        for field in &layout.fields {
            fields.push(mir::LayoutField {
                name: Some(field.name),
                ty: field.ty,
                offset: field.offset,
                size: field.size,
                alignment: field.alignment,
                source_index: field.source_index,
            });
        }

        self.insert_layout_metadata(ty, layout_shape, layout.size, layout.alignment, fields)
    }

    /// Insert a layout entry and attach it to the type metadata.
    fn insert_layout_metadata(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        layout_shape: mir::LayoutShape,
        size: u32,
        alignment: u32,
        fields: Vec<mir::LayoutField>,
    ) -> mir::LayoutId {
        // build the mir layout entry
        let layout_entry = mir::Layout {
            shape: layout_shape,
            size,
            alignment,
            reference_map: mir::ReferenceMap::empty(),
            fields,
        };

        // attach layout metadata to the type table
        let type_table = &mut self.builder.tree_mut().metadata.layout;
        let layout_id = type_table.layout_table.insert(layout_entry);
        type_table.set_layout_id(ty, layout_id);

        layout_id
    }

    /// Compute layout fields for a tuple type.
    fn tuple_layout_fields(
        &mut self,
        elements: &[mir::LocalNodeId<mir::Type>],
    ) -> Option<(Vec<mir::LayoutField>, u32, u32)> {
        // compute element layouts in order
        let mut fields = Vec::with_capacity(elements.len());
        let mut offset = 0u32;
        let mut max_alignment = 1u32;

        for (index, element) in elements.iter().enumerate() {
            // compute element size and alignment
            let element_type = self.builder.tree().get(*element);
            let (size, alignment) = self
                .type_lowerer
                .size_and_align_of_type(element_type, self.builder.tree())?;

            // align the current offset
            offset = align_up(offset, alignment);

            // record the element layout
            fields.push(mir::LayoutField {
                name: None,
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

        Some((fields, size, max_alignment))
    }

    /// Compute layout info for an array type.
    fn array_layout_info(
        &self,
        type_id: Option<dir::LocalTypeId>,
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<(mir::LayoutShape, u32, u32)> {
        // compute element size and alignment
        let element_type = self.builder.tree().get(element);
        let (size, alignment) = self
            .type_lowerer
            .size_and_align_of_type(element_type, self.builder.tree())
            .ok_or_else(|| {
                self.array_layout_error(
                    type_id,
                    anchor,
                    "array layout requires concrete nested types",
                )
            })?;
        let stride = align_up(size, alignment);

        // validate the array length
        let length_u32 = u32::try_from(length).map_err(|_| {
            self.array_layout_error(type_id, anchor, "array length exceeds layout limits")
        })?;
        let total_size = stride.checked_mul(length_u32).ok_or_else(|| {
            self.array_layout_error(type_id, anchor, "array layout size overflow")
        })?;

        Ok((
            mir::LayoutShape::Array {
                element_type: element,
                element_stride: stride,
                element_count: Some(length_u32),
            },
            total_size,
            alignment,
        ))
    }

    /// Resolve the layout kind for a lowered type.
    fn layout_shape_for_mir_type(
        &self,
        type_id: Option<dir::LocalTypeId>,
        mir_type: mir::LocalNodeId<mir::Type>,
        anchor: dir::AnchoredGlobalNodeId,
        layout: &StructLayout,
    ) -> LowerResult<mir::LayoutShape> {
        // prefer union layouts when present
        let layout_shape = if let Some(type_id) = type_id
            && let Some(union_layout) = self.type_lowerer.union_layout(type_id)
        {
            let tag_index = layout.field(union_layout.tag_field_index).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(anchor),
                    message: "missing union tag field".to_string(),
                }
            })?;
            let payload_index =
                layout
                    .field(union_layout.payload_field_index)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(anchor),
                        message: "missing union payload field".to_string(),
                    })?;

            mir::LayoutShape::Variant {
                tag_offset: tag_index.offset,
                payload_type: union_layout.payload_type,
                payload_offset: payload_index.offset,
                payload: match union_layout.payload {
                    crate::lower::r#type::VariantPayload::Inline => mir::VariantPayload::Inline,
                    crate::lower::r#type::VariantPayload::Boxed => mir::VariantPayload::Boxed,
                },
            }
        }
        // prefer Any layouts when present
        else if let Some(type_id) = type_id
            && let Some(any_layout) = self.type_lowerer.any_value_layout(type_id)
        {
            let value_field = layout.field(any_layout.value_field_index).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(anchor),
                    message: "missing Any value field".to_string(),
                }
            })?;
            let table_field = layout.field(any_layout.table_field_index).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(anchor),
                    message: "missing interface table field".to_string(),
                }
            })?;

            mir::LayoutShape::Any {
                value_offset: value_field.offset,
                table_offset: table_field.offset,
            }
        }
        // preserve object dispatch headers as first-class layout metadata
        else if let Some(vtable_field) = layout
            .fields
            .iter()
            .find(|field| field.kind == FieldLayoutKind::VtableHeader)
        {
            mir::LayoutShape::Object {
                table_offset: vtable_field.offset,
            }
        }
        // use function value layouts for function types
        else if type_id.is_some_and(|type_id| {
            matches!(self.types.get_type(type_id), dir::Type::Function { .. })
        }) {
            mir::LayoutShape::Callable
        }
        // mark function environments explicitly when present
        else if self
            .function_environment_layouts
            .values()
            .any(|env_layout| env_layout.env_type == mir_type)
        {
            mir::LayoutShape::CallableEnvironment
        }
        // default to plain struct layout
        else {
            mir::LayoutShape::Struct
        };

        Ok(layout_shape)
    }

    /// Build one consistent array layout error.
    fn array_layout_error(
        &self,
        type_id: Option<dir::LocalTypeId>,
        anchor: dir::AnchoredGlobalNodeId,
        message: &str,
    ) -> LowerError {
        if let Some(type_id) = type_id {
            return LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(anchor),
                ty: type_id.into_global(self.module_id),
                message: message.to_string(),
            };
        }

        LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(anchor),
            message: message.to_string(),
        }
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
