use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::{LowerError, LowerResult};

/// Layout metadata for erased dynamic values.
#[derive(Debug, Clone)]
pub(crate) struct DynamicValueLayout {
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
    /// Return cached dynamic value layout metadata.
    pub(crate) fn dynamic_value_layout(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<&DynamicValueLayout> {
        self.dynamic_value_layout_cache.get(&type_id)
    }

    /// Lower one erased dynamic value layout.
    pub(crate) fn lower_dynamic_value_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        constraint_type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // return cached types when available
        if let Some(mir_type) = self.cached_type(type_id) {
            return Ok(mir_type);
        }

        // load the constraint type for validation
        let dir::Type::Reference(reference) = types.get_type(constraint_type_id) else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: constraint_type_id.into_global(module_id),
                message: "expected dynamic constraint type".to_string(),
            }
            .into());
        };

        // require an interface-shaped constraint
        if !self.symbol_kind_matches(reference.symbol, dir::SymbolKind::Interface) {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: constraint_type_id.into_global(module_id),
                message: "Dynamic<T> constraint must be an interface".to_string(),
            }
            .into());
        }

        // lower the erased dynamic constraint shape
        let constraint_type_id = types
            .get_instance_type_id(reference.symbol)
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: constraint_type_id.into_global(module_id),
                message: "dynamic constraint type missing instance shape".to_string(),
            })?;
        let constraint_type =
            self.lower_type(types, constraint_type_id, module_id, node, builder)?;

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
                message: "dynamic layout requires concrete nested types".to_string(),
            })?;
        let (table_size, table_alignment) = self
            .size_and_align_of_type(builder.tree().get(table_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "dynamic layout requires concrete nested types".to_string(),
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

        // compute layout and create the MIR dynamic type
        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Source);
        let mir_type = builder.type_dynamic(constraint_type);
        self.layout_cache.insert(mir_type, layout.clone());

        // resolve field indices for dynamic metadata
        let value_field_index =
            layout
                .field_index(value_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing dynamic value field".to_string(),
                })?;
        let table_field_index =
            layout
                .field_index(table_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing dynamic table field".to_string(),
                })?;

        // cache dynamic value metadata
        self.dynamic_value_layout_cache.insert(
            type_id,
            DynamicValueLayout {
                value_field_index,
                table_field_index,
                value_type,
                table_type,
            },
        );

        Ok(mir_type)
    }
}
