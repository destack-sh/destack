use destack_core::{StringId, StringPool};

use crate::tree::compute_type_layout;
use crate::{
    Field, Layout, LayoutField, LayoutKind, LayoutTrace, LocalNodeId, NodeTree, ReferenceKind,
    Type, TypeReference,
};

/// One layout synthesis result.
pub(crate) type LayoutBuildResult<T> = Result<T, LayoutBuildError>;

/// One layout synthesis failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LayoutBuildError {
    /// One array length did not fit in the metadata representation.
    ArrayLengthOverflow,
    /// One aggregate layout id was missing unexpectedly.
    MissingLayoutId { type_id: LocalNodeId<Type> },
    /// One layout entry was missing unexpectedly.
    MissingLayoutEntry { index: usize },
    /// One layout computation overflowed.
    Overflow { context: &'static str },
}

impl std::fmt::Display for LayoutBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArrayLengthOverflow => write!(formatter, "array length exceeds layout metadata"),
            Self::MissingLayoutId { type_id } => {
                write!(
                    formatter,
                    "missing layout id during layout synthesis: {type_id:?}"
                )
            }
            Self::MissingLayoutEntry { index } => {
                write!(
                    formatter,
                    "missing layout entry during layout synthesis: index={index}"
                )
            }
            Self::Overflow { context } => {
                write!(formatter, "layout synthesis overflow: {context}")
            }
        }
    }
}

impl std::error::Error for LayoutBuildError {}

/// Build canonical layout metadata for every parsed MIR type.
pub(crate) fn build_layout_metadata(
    tree: &mut NodeTree,
    strings: &mut StringPool,
) -> LayoutBuildResult<()> {
    let type_ids: Vec<_> = tree
        .iter_nodes::<Type>()
        .map(|(type_id, _)| type_id)
        .collect();
    let mut builder = LayoutBuilder { tree, strings };

    for type_id in type_ids {
        builder.record_layout_for_type(type_id)?;
    }

    Ok(())
}

/// One in-place layout metadata builder.
struct LayoutBuilder<'a> {
    /// The MIR tree being populated.
    tree: &'a mut NodeTree,
    /// The canonical string pool.
    strings: &'a mut StringPool,
}

impl LayoutBuilder<'_> {
    /// Record layout metadata for one concrete aggregate type.
    fn record_layout_for_type(&mut self, type_id: LocalNodeId<Type>) -> LayoutBuildResult<()> {
        if self.tree.metadata.layout.layout_id(type_id).is_some() {
            return Ok(());
        }

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

    /// Record layout metadata for one struct type.
    fn record_struct_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        fields: &[LocalNodeId<Field>],
    ) -> LayoutBuildResult<()> {
        let mut layout_fields = Vec::with_capacity(fields.len());
        let mut offset = 0u32;

        // field layouts
        for (index, field_id) in fields.iter().enumerate() {
            let (field_name, field_type) = {
                let field = self.tree.get(*field_id);
                (field.name, field.ty)
            };
            let Some(field_type) = concrete_type(field_type) else {
                return Ok(());
            };
            let field_layout =
                compute_type_layout(self.tree, field_type, self.tree.pointer_bytes());
            offset = field_layout.align_offset(offset);

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

        let layout = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes());
        let layout_entry = Layout {
            kind: LayoutKind::Struct,
            size: layout.size,
            alignment: layout.alignment,
            scan: LayoutTrace::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_layout_scan(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one tuple type.
    fn record_tuple_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        elements: &[TypeReference],
    ) -> LayoutBuildResult<()> {
        let mut layout_fields = Vec::with_capacity(elements.len());
        let mut offset = 0u32;
        let mut alignment = 1u32;

        // element layouts
        for (index, element_id) in elements.iter().copied().enumerate() {
            let Some(element_id) = concrete_type(element_id) else {
                return Ok(());
            };

            let element_layout =
                compute_type_layout(self.tree, element_id, self.tree.pointer_bytes());
            offset = element_layout.align_offset(offset);

            layout_fields.push(LayoutField {
                name: self.synthetic_tuple_name(index),
                ty: element_id,
                offset,
                size: element_layout.size,
                alignment: element_layout.alignment,
                source_index: Some(index as u32),
            });

            offset += element_layout.size;
            alignment = alignment.max(element_layout.alignment);
        }

        let size = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes()).size;
        let layout_entry = Layout {
            kind: LayoutKind::Tuple,
            size,
            alignment,
            scan: LayoutTrace::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_layout_scan(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one array type.
    fn record_array_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        element: TypeReference,
        length: u64,
    ) -> LayoutBuildResult<()> {
        let Some(element) = concrete_type(element) else {
            return Ok(());
        };

        let element_layout = compute_type_layout(self.tree, element, self.tree.pointer_bytes());
        let stride = align_up(element_layout.size, element_layout.alignment);
        let count = u32::try_from(length).map_err(|_| LayoutBuildError::ArrayLengthOverflow)?;
        let size = stride
            .checked_mul(count)
            .ok_or(LayoutBuildError::Overflow {
                context: "array layout size",
            })?;

        let layout_entry = Layout {
            kind: LayoutKind::Array {
                element_type: element,
                element_stride: stride,
                element_count: Some(count),
            },
            size,
            alignment: element_layout.alignment,
            scan: LayoutTrace::empty(),
            fields: Vec::new(),
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_layout_scan(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one function value type.
    fn record_function_value_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        signature: TypeReference,
    ) -> LayoutBuildResult<()> {
        let Some(signature) = concrete_type(signature) else {
            return Ok(());
        };

        let environment = self.tree.ensure_function_value_environment_type();
        let components = [signature, environment];
        let mut layout_fields = Vec::with_capacity(components.len());
        let mut offset = 0u32;
        let mut alignment = 1u32;

        // component layouts
        for (index, component_type) in components.into_iter().enumerate() {
            let component_layout =
                compute_type_layout(self.tree, component_type, self.tree.pointer_bytes());
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
            alignment = alignment.max(component_layout.alignment);
        }

        let layout = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes());
        let layout_entry = Layout {
            kind: LayoutKind::Closure,
            size: layout.size,
            alignment,
            scan: LayoutTrace::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_layout_scan(type_id)?;

        Ok(())
    }

    /// Insert one layout entry and attach it to the type table.
    fn insert_layout_entry(&mut self, type_id: LocalNodeId<Type>, layout: Layout) {
        let layout_id = self.tree.metadata.layout.layout_table.insert(layout);
        self.tree.metadata.layout.set_layout_id(type_id, layout_id);
    }

    /// Compute and record the scan metadata for one aggregate layout.
    fn record_layout_scan(&mut self, type_id: LocalNodeId<Type>) -> LayoutBuildResult<()> {
        let scan = self.build_layout_scan(type_id)?;
        let layout_id = self
            .tree
            .metadata
            .layout
            .layout_id(type_id)
            .ok_or(LayoutBuildError::MissingLayoutId { type_id })?;
        let Some(layout) = self
            .tree
            .metadata
            .layout
            .layout_table
            .layouts
            .get_mut(layout_id.index())
        else {
            return Err(LayoutBuildError::MissingLayoutEntry {
                index: layout_id.index(),
            });
        };

        layout.scan = scan;

        Ok(())
    }

    /// Build the scan metadata for one concrete type.
    fn build_layout_scan(&mut self, type_id: LocalNodeId<Type>) -> LayoutBuildResult<LayoutTrace> {
        if let Type::Array {
            element, length, ..
        } = self.tree.get(type_id)
        {
            let Some(element) = concrete_type(*element) else {
                return Ok(LayoutTrace::empty());
            };

            let element_layout = compute_type_layout(self.tree, element, self.tree.pointer_bytes());
            let stride = align_up(element_layout.size, element_layout.alignment);
            let count =
                u32::try_from(*length).map_err(|_| LayoutBuildError::ArrayLengthOverflow)?;
            let mut offsets = Vec::new();
            self.append_layout_scan_offsets(element, 0, &mut offsets)?;

            if offsets.is_empty() {
                return Ok(LayoutTrace::empty());
            }

            return Ok(LayoutTrace::RepeatedReference {
                count,
                stride,
                offsets: offsets.into_boxed_slice(),
            });
        }

        let mut offsets = Vec::new();
        self.append_layout_scan_offsets(type_id, 0, &mut offsets)?;

        if offsets.is_empty() {
            Ok(LayoutTrace::empty())
        } else {
            Ok(LayoutTrace::Reference {
                offsets: offsets.into_boxed_slice(),
            })
        }
    }

    /// Append managed-reference offsets for one concrete type.
    fn append_layout_scan_offsets(
        &mut self,
        type_id: LocalNodeId<Type>,
        base_offset: u32,
        offsets: &mut Vec<u32>,
    ) -> LayoutBuildResult<()> {
        match self.tree.get(type_id) {
            Type::Reference {
                kind: ReferenceKind::Managed,
                ..
            } => {
                offsets.push(base_offset);

                Ok(())
            }
            Type::Struct { .. } | Type::Tuple { .. } | Type::Closure { .. } => {
                self.record_layout_for_type(type_id)?;
                let layout_id = self
                    .tree
                    .metadata
                    .layout
                    .layout_id(type_id)
                    .ok_or(LayoutBuildError::MissingLayoutId { type_id })?;
                let layout = self
                    .tree
                    .metadata
                    .layout
                    .layout_table
                    .layout(layout_id)
                    .clone();

                for field in layout.fields {
                    let field_offset = base_offset.checked_add(field.offset).ok_or(
                        LayoutBuildError::Overflow {
                            context: "layout trace field offset",
                        },
                    )?;

                    self.append_layout_scan_offsets(field.ty, field_offset, offsets)?;
                }

                Ok(())
            }
            Type::Array {
                element, length, ..
            } => {
                let Some(element) = concrete_type(*element) else {
                    return Ok(());
                };
                let element_layout =
                    compute_type_layout(self.tree, element, self.tree.pointer_bytes());
                let stride = align_up(element_layout.size, element_layout.alignment);
                let count =
                    u32::try_from(*length).map_err(|_| LayoutBuildError::ArrayLengthOverflow)?;

                let mut element_offsets = Vec::new();
                self.append_layout_scan_offsets(element, 0, &mut element_offsets)?;

                if element_offsets.is_empty() {
                    return Ok(());
                }

                for index in 0..count {
                    let delta = stride
                        .checked_mul(index)
                        .ok_or(LayoutBuildError::Overflow {
                            context: "layout trace array stride",
                        })?;

                    for element_offset in &element_offsets {
                        let offset = base_offset
                            .checked_add(delta)
                            .and_then(|value| value.checked_add(*element_offset))
                            .ok_or(LayoutBuildError::Overflow {
                                context: "layout trace array offset",
                            })?;
                        offsets.push(offset);
                    }
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Build one synthetic field name for one unnamed struct field.
    fn synthetic_field_name(&mut self, index: usize) -> StringId {
        let name = format!("@field{index}");
        self.strings.intern(&name)
    }

    /// Build one synthetic tuple field name.
    fn synthetic_tuple_name(&mut self, index: usize) -> StringId {
        let name = index.to_string();
        self.strings.intern(&name)
    }
}

/// Return one concrete type when present.
fn concrete_type(reference: TypeReference) -> Option<LocalNodeId<Type>> {
    match reference {
        TypeReference::Type(ty) => Some(ty),
        TypeReference::Missing | TypeReference::Error => None,
    }
}

/// Align one size up to the requested alignment.
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
