use destack_base::StringId;

use {destack_dir as dir, destack_mir as mir};

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
    pub source_index: u32,
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
    pub source_index: u32,
}

/// Convert a static key to a field name.
///
/// For name and number keys, returns the string directly.
/// For symbol keys, generates a synthetic name with `@` prefix to avoid conflicts.
pub(crate) fn static_key_to_field_name(
    key: &dir::StaticKey,
    builder: &mut mir::ModuleBuilder,
) -> StringId {
    match key {
        dir::StaticKey::Name(s) | dir::StaticKey::Number(s) => *s,
        dir::StaticKey::Symbol(symbol_key) => {
            let synthetic = match symbol_key {
                dir::SymbolKey::WellKnown(well_known) => {
                    format!("@{}", well_known.global_symbol_name())
                }
                dir::SymbolKey::Registry(s) => {
                    let key_str = builder.strings().get(*s);
                    format!("@Symbol.for:{}", &*key_str)
                }
                dir::SymbolKey::Unique(global_id) => format!("@Symbol#{global_id:?}"),
            };
            builder.intern(&synthetic)
        }
    }
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
            .position(|f| f.source_index == source_index)
            .map(|i| i as u32)
    }

    /// Get a field by its layout index.
    pub(crate) fn field(&self, index: u32) -> Option<&FieldLayout> {
        self.fields.get(index as usize)
    }
}

/// Compute struct layout from a list of field inputs.
///
/// This implements the layout policy:
/// - Optimized: sort by alignment (desc), size (desc), source order as tiebreaker
/// - Source/C: preserve declaration order
pub(crate) fn compute_struct_layout(
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
                b.alignment
                    .cmp(&a.alignment)
                    .then_with(|| b.size.cmp(&a.size))
                    .then_with(|| a.source_index.cmp(&b.source_index))
            });
        }
        LayoutPolicy::Source | LayoutPolicy::C => {
            // keep source order
            fields.sort_by_key(|f| f.source_index);
        }
    }

    // compute struct alignment (max of all field alignments)
    let struct_alignment = fields.iter().map(|f| f.alignment).max().unwrap_or(1);

    // assign offsets
    let mut current_offset: u32 = 0;
    let mut layout_fields = Vec::with_capacity(fields.len());

    for field in fields {
        // align current offset to field's alignment requirement
        let aligned_offset = align_up(current_offset, field.alignment);
        layout_fields.push(FieldLayout {
            name: field.name,
            ty: field.ty,
            offset: aligned_offset,
            size: field.size,
            alignment: field.alignment,
            source_index: field.source_index,
        });
        current_offset = aligned_offset + field.size;
    }

    // add trailing padding to align struct size
    let total_size = align_up(current_offset, struct_alignment);

    StructLayout {
        fields: layout_fields,
        size: total_size,
        alignment: struct_alignment,
    }
}

/// Align a value up to the given alignment.
/// Alignment must be a power of 2.
#[inline]
pub(crate) fn align_up(value: u32, alignment: u32) -> u32 {
    debug_assert!(alignment.is_power_of_two(), "alignment must be power of 2");
    (value + alignment - 1) & !(alignment - 1)
}

/// Get the size and alignment for a MIR type.
/// Returns (size, alignment) in bytes.
pub(crate) fn size_and_align_of_type(
    ty: &mir::Type,
    tree: &mir::NodeTree,
    pointer_bytes: u8,
) -> (u32, u32) {
    match ty {
        mir::Type::Void => (0, 1),
        mir::Type::Boolean => (1, 1),
        mir::Type::Int { width, .. } => {
            let bytes = u32::from(*width).div_ceil(8);
            // natural alignment: min(size, 8) for most ABIs
            let align = bytes.min(8);
            (bytes, align)
        }
        mir::Type::Float { width } => {
            let bytes = u32::from(*width).div_ceil(8);
            let align = bytes.min(8);
            (bytes, align)
        }
        mir::Type::Reference { .. } => {
            let bytes = pointer_bytes as u32;
            (bytes, bytes)
        }
        mir::Type::Array {
            element,
            length,
            copyability: _,
        } => {
            let element_ty = tree.get(*element);
            let (elem_size, elem_align) = size_and_align_of_type(element_ty, tree, pointer_bytes);
            (elem_size * (*length as u32), elem_align)
        }
        mir::Type::Tuple {
            elements,
            copyability: _,
        } => {
            // tuple layout is like a struct with anonymous fields
            let mut max_align: u32 = 1;
            let mut current_offset: u32 = 0;

            for elem_id in elements {
                let elem_ty = tree.get(*elem_id);
                let (elem_size, elem_align) = size_and_align_of_type(elem_ty, tree, pointer_bytes);
                max_align = max_align.max(elem_align);
                current_offset = align_up(current_offset, elem_align) + elem_size;
            }

            let total_size = align_up(current_offset, max_align);
            (total_size, max_align)
        }
        mir::Type::Struct {
            fields,
            copyability: _,
        } => {
            // for already laid out structs, compute from field info
            let mut max_align: u32 = 1;
            let mut max_end: u32 = 0;

            for field_id in fields {
                let field = tree.get(*field_id);
                let field_ty = tree.get(field.ty);
                let (field_size, field_align) =
                    size_and_align_of_type(field_ty, tree, pointer_bytes);
                max_align = max_align.max(field_align);
                max_end = max_end.max(field.offset + field_size);
            }

            let total_size = align_up(max_end, max_align);
            (total_size, max_align)
        }
        mir::Type::FunctionPointer { .. } => {
            // function pointers are pointer sized
            let bytes = pointer_bytes as u32;
            (bytes, bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_base::StringPool;

    /// Align value up to alignment boundary.
    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(7, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 8), 16);
    }

    /// Empty struct has zero size and alignment 1.
    #[test]
    fn test_empty_struct() {
        let layout = compute_struct_layout(vec![], LayoutPolicy::Optimized);
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

        // create a dummy type id (we only care about layout, not the actual type)
        let dummy_ty = mir::LocalNodeId::new(0);

        // fields: a (1 byte, align 1), b (4 bytes, align 4), c (2 bytes, align 2)
        let fields = vec![
            FieldInput {
                name: a,
                ty: dummy_ty,
                size: 1,
                alignment: 1,
                source_index: 0,
            },
            FieldInput {
                name: b,
                ty: dummy_ty,
                size: 4,
                alignment: 4,
                source_index: 1,
            },
            FieldInput {
                name: c,
                ty: dummy_ty,
                size: 2,
                alignment: 2,
                source_index: 2,
            },
        ];

        let layout = compute_struct_layout(fields, LayoutPolicy::Optimized);

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

        let dummy_ty = mir::LocalNodeId::new(0);

        // fields in source order: a (1 byte), b (4 bytes)
        let fields = vec![
            FieldInput {
                name: a,
                ty: dummy_ty,
                size: 1,
                alignment: 1,
                source_index: 0,
            },
            FieldInput {
                name: b,
                ty: dummy_ty,
                size: 4,
                alignment: 4,
                source_index: 1,
            },
        ];

        let layout = compute_struct_layout(fields, LayoutPolicy::Source);

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

    /// Name keys return the string directly.
    #[test]
    fn test_static_key_name() {
        let mut builder = mir::ModuleBuilder::new();
        let name = builder.intern("foo");
        let key = dir::StaticKey::Name(name);

        let result = static_key_to_field_name(&key, &mut builder);
        assert_eq!(result, name);
    }

    /// Number keys return the string directly.
    #[test]
    fn test_static_key_number() {
        let mut builder = mir::ModuleBuilder::new();
        let num = builder.intern("42");
        let key = dir::StaticKey::Number(num);

        let result = static_key_to_field_name(&key, &mut builder);
        assert_eq!(result, num);
    }

    /// Well-known symbol keys get synthetic names with @ prefix.
    #[test]
    fn test_static_key_well_known_symbol() {
        let mut builder = mir::ModuleBuilder::new();
        let key = dir::StaticKey::Symbol(dir::SymbolKey::WellKnown(
            dir::WellKnownSymbolKey::SymbolIterator,
        ));

        let result = static_key_to_field_name(&key, &mut builder);
        let result_str = builder.strings().get(result);
        assert_eq!(&*result_str, "@Symbol.iterator");
    }

    /// Registry symbol keys get synthetic names with @ prefix.
    #[test]
    fn test_static_key_registry_symbol() {
        let mut builder = mir::ModuleBuilder::new();
        let registry_key = builder.intern("myKey");
        let key = dir::StaticKey::Symbol(dir::SymbolKey::Registry(registry_key));

        let result = static_key_to_field_name(&key, &mut builder);
        let result_str = builder.strings().get(result);
        assert_eq!(&*result_str, "@Symbol.for:myKey");
    }
}
