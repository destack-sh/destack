use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::{LowerError, LowerResult};

/// Layout metadata for erased Any values.
#[derive(Debug, Clone)]
pub(crate) struct AnyValueLayout {
    /// Field index for the value pointer.
    pub(crate) value_field_index: u32,
    /// Field index for the table pointer.
    pub(crate) table_field_index: u32,
    /// The MIR type of the value pointer field.
    pub(crate) value_type: mir::LocalNodeId<mir::Type>,
    /// The MIR type of the table pointer field.
    pub(crate) table_type: mir::LocalNodeId<mir::Type>,
}

impl TypeLowerer<'_> {
    /// Return cached Any value layout metadata.
    pub(crate) fn any_value_layout(&self, type_id: dir::LocalTypeId) -> Option<&AnyValueLayout> {
        self.any_value_layout_cache.get(&type_id)
    }

    /// Lower one erased Any value layout.
    pub(crate) fn lower_any_value_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // return cached types when available
        if let Some(mir_type) = self.cached_type(type_id) {
            return Ok(mir_type);
        }

        // load the dir type for validation
        let dir::Type::Reference(reference) = types.get_type(type_id) else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "expected Any interface type".to_string(),
            }
            .into());
        };

        // lower the erased interface shape
        let interface_type_id = types
            .get_instance_type_id(reference.symbol)
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "Any interface type missing instance shape".to_string(),
            })?;
        let interface_type = self.lower_type(types, interface_type_id, module_id, node, builder)?;

        // define value and table field names and types
        let value_name = builder.intern("value");
        let table_name = builder.intern("table");
        let value_type = builder.type_managed_reference(self.ty_void);
        let table_type = builder.type_reference(
            mir::ReferenceKind::Raw,
            self.ty_void,
            mir::Access::Readonly,
            mir::Space::Static,
            mir::Nullability::None,
        );

        // compute field sizes and alignments
        let (value_size, value_alignment) = self
            .size_and_align_of_type(builder.tree().get(value_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "Any layout requires concrete nested types".to_string(),
            })?;
        let (table_size, table_alignment) = self
            .size_and_align_of_type(builder.tree().get(table_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "Any layout requires concrete nested types".to_string(),
            })?;

        // assemble field inputs
        let fields = vec![
            FieldInput {
                name: value_name,
                ty: value_type,
                size: value_size,
                alignment: value_alignment,
                source_index: Some(0),
                kind: FieldLayoutKind::Synthetic,
            },
            FieldInput {
                name: table_name,
                ty: table_type,
                size: table_size,
                alignment: table_alignment,
                source_index: Some(1),
                kind: FieldLayoutKind::Synthetic,
            },
        ];

        // compute layout and create the MIR Any type
        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Source);
        let mir_type = builder.type_any(interface_type);
        self.layout_cache.insert(mir_type, layout.clone());

        // resolve field indices for Any metadata
        let value_field_index =
            layout
                .field_index(value_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing Any value field".to_string(),
                })?;
        let table_field_index =
            layout
                .field_index(table_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing interface table field".to_string(),
                })?;

        // cache Any value metadata
        self.any_value_layout_cache.insert(
            type_id,
            AnyValueLayout {
                value_field_index,
                table_field_index,
                value_type,
                table_type,
            },
        );

        Ok(mir_type)
    }
}
