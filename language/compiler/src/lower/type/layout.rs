use destack_core::StringId;

use destack_mir as mir;

use super::TypeLowerer;

/// Layout policy for struct fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub(crate) enum LayoutPolicy {
    /// Minimizes padding while maintaining deterministic layout.
    #[default]
    Optimized,
    /// Preserve source declaration order (for `@layout("source")`).
    Source,
    /// C ABI compatible layout (for `@layout("C")` / FFI).
    C,
}

/// Policy values for type layout decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeLayoutPolicy {
    /// Maximum size in bytes for inline union payloads.
    pub inline_union_budget_bytes: u32,
    /// Maximum alignment in bytes for inline union payloads.
    pub inline_union_max_alignment: u32,
    /// Require trivial copyability for inline union payloads.
    pub inline_union_requires_trivial_copyability: bool,
}

impl TypeLayoutPolicy {
    /// Build a layout policy using target pointer size.
    pub(crate) fn for_target(pointer_bytes: u8) -> Self {
        let pointer_size = u32::from(pointer_bytes).max(1);
        let inline_union_budget_bytes = pointer_size.saturating_mul(2);

        Self {
            inline_union_budget_bytes,
            inline_union_max_alignment: pointer_size,
            inline_union_requires_trivial_copyability: true,
        }
    }
}

/// Classification for layout fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldLayoutKind {
    /// Field derived from source declarations.
    Source,
    /// Synthetic vtable header field for class layouts.
    VtableHeader,
    /// Synthetic field derived from lowering.
    Synthetic,
}

/// A computed struct layout with field offsets and total size.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct StructLayout {
    /// The fields in layout order (may differ from source order).
    pub fields: Vec<FieldLayout>,
    /// Total size in bytes (including trailing padding for alignment).
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
}

/// Layout information for a single field.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct FieldLayout {
    /// Field name (for lookup and debugging).
    pub name: StringId,
    /// The MIR type of this field.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// Byte offset within the struct.
    pub offset: u32,
    /// Size of this field in bytes.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Original source index (for mapping back to source order).
    pub source_index: Option<u32>,
    /// Field classification for layout logic.
    pub kind: FieldLayoutKind,
}

/// Input field for layout computation (before offsets are assigned).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct FieldInput {
    /// Field name.
    pub name: StringId,
    /// The MIR type of this field.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// Size of this field in bytes.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Original source index.
    pub source_index: Option<u32>,
    /// Field classification for layout logic.
    pub kind: FieldLayoutKind,
}

#[allow(dead_code, unused)]
impl StructLayout {
    /// Create an empty struct layout (zero-sized).
    pub(crate) fn empty() -> Self {
        Self {
            fields: Vec::new(),
            size: 0,
            alignment: 1,
        }
    }

    /// Find a field by name and return its index in layout order.
    pub(crate) fn field_index(&self, name: StringId) -> Option<u32> {
        self.fields
            .iter()
            .position(|f| f.name == name)
            .map(|i| i as u32)
    }

    /// Find a field by source index and return its index in layout order.
    pub(crate) fn field_index_by_source(&self, source_index: u32) -> Option<u32> {
        self.fields
            .iter()
            .position(|f| f.source_index == Some(source_index))
            .map(|i| i as u32)
    }

    /// Get a field by its layout index.
    pub(crate) fn field(&self, index: u32) -> Option<&FieldLayout> {
        self.fields.get(index as usize)
    }
}

impl TypeLowerer {
    /// Compute struct layout from a list of field inputs:
    /// - Optimized: sort by alignment (desc), size (desc), source order as tiebreaker
    /// - Source/C: preserve declaration order
    pub(crate) fn compute_struct_layout(
        &self,
        mut fields: Vec<FieldInput>,
        policy: LayoutPolicy,
    ) -> StructLayout {
        if fields.is_empty() {
            return StructLayout::empty();
        }

        // sort fields according to policy
        match policy {
            LayoutPolicy::Optimized => {
                // sort by alignment desc, then size desc, then source order asc
                fields.sort_by(|a, b| {
                    let a_source = (a.source_index.is_none(), a.source_index.unwrap_or_default());
                    let b_source = (b.source_index.is_none(), b.source_index.unwrap_or_default());
                    b.alignment
                        .cmp(&a.alignment)
                        .then_with(|| b.size.cmp(&a.size))
                        .then_with(|| a_source.cmp(&b_source))
                });
            }
            LayoutPolicy::Source | LayoutPolicy::C => {
                // keep source order
                fields.sort_by_key(|f| {
                    (f.source_index.is_none(), f.source_index.unwrap_or_default())
                });
            }
        }

        // compute struct alignment (max of all field alignments)
        let struct_alignment = fields.iter().map(|f| f.alignment).max().unwrap_or(1);

        // assign offsets
        let mut current_offset: u32 = 0;
        let mut layout_fields = Vec::with_capacity(fields.len());

        for field in fields {
            // align current offset to field's alignment requirement
            let aligned_offset = self.align_up(current_offset, field.alignment);
            layout_fields.push(FieldLayout {
                name: field.name,
                ty: field.ty,
                offset: aligned_offset,
                size: field.size,
                alignment: field.alignment,
                source_index: field.source_index,
                kind: field.kind,
            });
            current_offset = aligned_offset + field.size;
        }

        // add trailing padding to align struct size
        let total_size = self.align_up(current_offset, struct_alignment);

        StructLayout {
            fields: layout_fields,
            size: total_size,
            alignment: struct_alignment,
        }
    }

    /// Compute struct layout with a fixed prefix field at offset zero.
    pub(crate) fn compute_struct_layout_with_prefix(
        &self,
        prefix: FieldInput,
        fields: Vec<FieldInput>,
        policy: LayoutPolicy,
    ) -> StructLayout {
        if fields.is_empty() {
            let size = self.align_up(prefix.size, prefix.alignment);
            return StructLayout {
                fields: vec![FieldLayout {
                    name: prefix.name,
                    ty: prefix.ty,
                    offset: 0,
                    size: prefix.size,
                    alignment: prefix.alignment,
                    source_index: prefix.source_index,
                    kind: prefix.kind,
                }],
                size,
                alignment: prefix.alignment,
            };
        }

        let rest_layout = self.compute_struct_layout(fields, policy);
        let base_offset = self.align_up(prefix.size, rest_layout.alignment);
        let alignment = prefix.alignment.max(rest_layout.alignment);

        let mut merged_fields = Vec::with_capacity(rest_layout.fields.len() + 1);
        merged_fields.push(FieldLayout {
            name: prefix.name,
            ty: prefix.ty,
            offset: 0,
            size: prefix.size,
            alignment: prefix.alignment,
            source_index: prefix.source_index,
            kind: prefix.kind,
        });

        for field in rest_layout.fields {
            merged_fields.push(FieldLayout {
                name: field.name,
                ty: field.ty,
                offset: base_offset + field.offset,
                size: field.size,
                alignment: field.alignment,
                source_index: field.source_index,
                kind: field.kind,
            });
        }

        let size = self.align_up(base_offset + rest_layout.size, alignment);

        StructLayout {
            fields: merged_fields,
            size,
            alignment,
        }
    }

    /// Compute struct layout with an existing layout prefix.
    pub(crate) fn compute_struct_layout_with_base(
        &self,
        base: StructLayout,
        fields: Vec<FieldInput>,
        policy: LayoutPolicy,
    ) -> StructLayout {
        // return the base layout when there are no new fields
        if fields.is_empty() {
            return base;
        }

        // compute the layout for the new fields
        let rest_layout = self.compute_struct_layout(fields, policy);

        // align the derived fields after the base layout
        let base_offset = self.align_up(base.size, rest_layout.alignment);
        let alignment = base.alignment.max(rest_layout.alignment);

        // merge the base and derived layouts
        let mut merged_fields = Vec::with_capacity(base.fields.len() + rest_layout.fields.len());
        merged_fields.extend(base.fields);
        for field in rest_layout.fields {
            merged_fields.push(FieldLayout {
                name: field.name,
                ty: field.ty,
                offset: base_offset + field.offset,
                size: field.size,
                alignment: field.alignment,
                source_index: field.source_index,
                kind: field.kind,
            });
        }

        // compute the combined size with trailing padding
        let size = self.align_up(base_offset + rest_layout.size, alignment);

        // return the merged layout
        StructLayout {
            fields: merged_fields,
            size,
            alignment,
        }
    }

    /// Align a value up to the given alignment.
    /// Alignment must be a power of 2.
    #[inline]
    fn align_up(&self, value: u32, alignment: u32) -> u32 {
        debug_assert!(alignment.is_power_of_two(), "alignment must be power of 2");
        (value + alignment - 1) & !(alignment - 1)
    }

    /// Get the size and alignment for a MIR type.
    /// Returns (size, alignment) in bytes.
    pub(crate) fn size_and_align_of_type(
        &self,
        ty: &mir::Type,
        tree: &mir::NodeTree,
    ) -> Option<(u32, u32)> {
        let pointer_bytes = self.pointer_bytes();
        match ty {
            mir::Type::Void => Some((0, 1)),
            mir::Type::Boolean => Some((1, 1)),
            mir::Type::Int { width, .. } => {
                let bytes = u32::from(*width).div_ceil(8);
                let align = bytes.min(8);
                Some((bytes, align))
            }
            mir::Type::Isize | mir::Type::Usize => {
                let bytes = pointer_bytes as u32;
                let align = bytes.min(8);
                Some((bytes, align))
            }
            mir::Type::Float { width } => {
                let bytes = u32::from(*width).div_ceil(8);
                let align = bytes.min(8);
                Some((bytes, align))
            }
            mir::Type::TypeDescriptor | mir::Type::TypeId | mir::Type::Reference { .. } => {
                let bytes = pointer_bytes as u32;
                Some((bytes, bytes))
            }
            mir::Type::TensorReference { .. } => {
                let bytes = pointer_bytes as u32;
                Some((bytes, bytes))
            }
            mir::Type::Array {
                element,
                length,
                copyability: _,
            } => {
                let element_ty = tree.get(element.ty()?);
                let (elem_size, elem_align) = self.size_and_align_of_type(element_ty, tree)?;
                Some((elem_size * (*length as u32), elem_align))
            }
            mir::Type::Tuple {
                elements,
                copyability: _,
            } => {
                let mut max_align: u32 = 1;
                let mut current_offset: u32 = 0;

                for elem_id in elements {
                    let elem_ty = tree.get(elem_id.ty()?);
                    let (elem_size, elem_align) = self.size_and_align_of_type(elem_ty, tree)?;
                    max_align = max_align.max(elem_align);
                    current_offset = self.align_up(current_offset, elem_align) + elem_size;
                }

                let total_size = self.align_up(current_offset, max_align);
                Some((total_size, max_align))
            }
            mir::Type::Struct {
                fields,
                copyability: _,
            } => {
                let mut max_align: u32 = 1;
                let mut current_offset: u32 = 0;

                for field_id in fields {
                    let field = tree.get(*field_id);
                    let field_ty = tree.get(field.ty.ty()?);
                    let (field_size, field_align) = self.size_and_align_of_type(field_ty, tree)?;
                    max_align = max_align.max(field_align);
                    current_offset = self.align_up(current_offset, field_align) + field_size;
                }

                let total_size = self.align_up(current_offset, max_align);
                Some((total_size, max_align))
            }
            mir::Type::Closure { signature } => {
                let mut max_align: u32 = 1;
                let mut current_offset: u32 = 0;
                let environment = tree.function_value_environment_type();

                let signature_ty = tree.get(signature.ty()?);
                let (signature_size, signature_align) =
                    self.size_and_align_of_type(signature_ty, tree)?;
                max_align = max_align.max(signature_align);
                current_offset = self.align_up(current_offset, signature_align) + signature_size;

                let environment_ty = tree.get(environment);
                let (environment_size, environment_align) =
                    self.size_and_align_of_type(environment_ty, tree)?;
                max_align = max_align.max(environment_align);
                current_offset =
                    self.align_up(current_offset, environment_align) + environment_size;

                let total_size = self.align_up(current_offset, max_align);
                Some((total_size, max_align))
            }
            mir::Type::Newtype { inner, .. } => {
                let inner_ty = tree.get(inner.ty()?);
                self.size_and_align_of_type(inner_ty, tree)
            }
            mir::Type::FunctionPointer { .. } => {
                let bytes = pointer_bytes as u32;
                Some((bytes, bytes))
            }
            mir::Type::Vector {
                element,
                lanes,
                copyability: _,
            } => {
                let element_ty = tree.get(element.ty()?);
                let (elem_size, elem_align) = self.size_and_align_of_type(element_ty, tree)?;
                let size = elem_size * *lanes;
                Some((size, elem_align))
            }
            mir::Type::Tensor {
                element,
                shape,
                layout,
                copyability: _,
            } => {
                let element_ty = tree.get(element.ty()?);
                let (elem_size, elem_align) = self.size_and_align_of_type(element_ty, tree)?;
                let element_count = self.tensor_element_count(shape, layout);
                let size = elem_size * element_count;
                Some((size, elem_align))
            }
        }
    }

    fn tensor_element_count(
        &self,
        shape: &[mir::TensorDimension],
        layout: &mir::TensorLayout,
    ) -> u32 {
        // map dynamic dimensions to zero size
        let shape: Vec<u64> = shape
            .iter()
            .map(|dim| match dim {
                mir::TensorDimension::Static(value) => *value,
                mir::TensorDimension::Dynamic => 0,
            })
            .collect();
        match layout {
            mir::TensorLayout::RowMajor | mir::TensorLayout::ColumnMajor => shape
                .iter()
                .copied()
                .product::<u64>()
                .min(u64::from(u32::MAX))
                as u32,
            mir::TensorLayout::Strided { strides } => {
                let strides: Vec<u64> = strides
                    .iter()
                    .map(|dim| match dim {
                        mir::TensorDimension::Static(value) => *value,
                        mir::TensorDimension::Dynamic => 0,
                    })
                    .collect();
                let mut max_index = 0u64;
                for (dim, stride) in shape.iter().copied().zip(strides.iter().copied()) {
                    if dim == 0 {
                        continue;
                    }
                    let last_index = dim - 1;
                    let offset = last_index.saturating_mul(stride);
                    max_index = max_index.max(offset);
                }
                max_index.saturating_add(1).min(u64::from(u32::MAX)) as u32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_core::StringPool;
    use destack_workspace::{AmbientSnapshot, Repository};
    use std::sync::Arc;

    /// Create a type lowerer for layout tests.
    fn test_lowerer() -> TypeLowerer {
        let mut builder = mir::ModuleBuilder::unchecked();
        let repository = Arc::new(Repository::open_root(
            std::env::current_dir().unwrap_or_default(),
            AmbientSnapshot::default(),
        ));
        TypeLowerer::new(&mut builder, 8, repository, None)
    }

    /// Align value up to alignment boundary.
    #[test]
    fn test_align_up() {
        let lowerer = test_lowerer();
        assert_eq!(lowerer.align_up(0, 4), 0);
        assert_eq!(lowerer.align_up(1, 4), 4);
        assert_eq!(lowerer.align_up(4, 4), 4);
        assert_eq!(lowerer.align_up(5, 4), 8);
        assert_eq!(lowerer.align_up(7, 8), 8);
        assert_eq!(lowerer.align_up(8, 8), 8);
        assert_eq!(lowerer.align_up(9, 8), 16);
    }

    /// Empty struct has zero size and alignment 1.
    #[test]
    fn test_empty_struct() {
        let lowerer = test_lowerer();
        let layout = lowerer.compute_struct_layout(vec![], LayoutPolicy::Optimized);
        assert_eq!(layout.size, 0);
        assert_eq!(layout.alignment, 1);
        assert!(layout.fields.is_empty());
    }

    /// Optimized layout sorts by alignment then size descending.
    #[test]
    fn test_optimized_layout_sorting() {
        let strings = StringPool::new();
        let a = strings.intern("a");
        let b = strings.intern("b");
        let c = strings.intern("c");

        let lowerer = test_lowerer();
        let dummy_ty = lowerer.ty_i32;

        // fields: a (1 byte, align 1), b (4 bytes, align 4), c (2 bytes, align 2)
        let fields = vec![
            FieldInput {
                name: a,
                ty: dummy_ty,
                size: 1,
                alignment: 1,
                source_index: Some(0),
                kind: FieldLayoutKind::Source,
            },
            FieldInput {
                name: b,
                ty: dummy_ty,
                size: 4,
                alignment: 4,
                source_index: Some(1),
                kind: FieldLayoutKind::Source,
            },
            FieldInput {
                name: c,
                ty: dummy_ty,
                size: 2,
                alignment: 2,
                source_index: Some(2),
                kind: FieldLayoutKind::Source,
            },
        ];

        let layout = lowerer.compute_struct_layout(fields, LayoutPolicy::Optimized);

        // should be sorted: b (align 4), c (align 2), a (align 1)
        assert_eq!(layout.fields.len(), 3);
        assert_eq!(layout.fields[0].name, b);
        assert_eq!(layout.fields[1].name, c);
        assert_eq!(layout.fields[2].name, a);

        // offsets: b at 0, c at 4, a at 6
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 4);
        assert_eq!(layout.fields[2].offset, 6);

        // total size: 7 bytes, padded to 8 for struct alignment (4)
        assert_eq!(layout.size, 8);
        assert_eq!(layout.alignment, 4);
    }

    /// Source layout preserves declaration order.
    #[test]
    fn test_source_layout_preserves_order() {
        let strings = StringPool::new();
        let a = strings.intern("a");
        let b = strings.intern("b");

        let lowerer = test_lowerer();
        let dummy_ty = lowerer.ty_i32;

        // fields in source order: a (1 byte), b (4 bytes)
        let fields = vec![
            FieldInput {
                name: a,
                ty: dummy_ty,
                size: 1,
                alignment: 1,
                source_index: Some(0),
                kind: FieldLayoutKind::Source,
            },
            FieldInput {
                name: b,
                ty: dummy_ty,
                size: 4,
                alignment: 4,
                source_index: Some(1),
                kind: FieldLayoutKind::Source,
            },
        ];

        let layout = lowerer.compute_struct_layout(fields, LayoutPolicy::Source);

        // should preserve order: a, b
        assert_eq!(layout.fields[0].name, a);
        assert_eq!(layout.fields[1].name, b);

        // offsets: a at 0, b at 4 (padded for alignment)
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 4);

        // total size: 8 bytes
        assert_eq!(layout.size, 8);
        assert_eq!(layout.alignment, 4);
    }
}
