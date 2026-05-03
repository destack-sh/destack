use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::{LowerError, LowerResult};

/// Layout metadata for interface reference types.
#[derive(Debug, Clone)]
pub(crate) struct InterfaceRefLayout {
    /// Field index for the object pointer.
    pub(crate) object_field_index: u32,
    /// Field index for the itab pointer.
    pub(crate) itab_field_index: u32,
    /// The MIR type of the object pointer field.
    pub(crate) object_type: mir::LocalNodeId<mir::Type>,
    /// The MIR type of the itab pointer field.
    pub(crate) itab_type: mir::LocalNodeId<mir::Type>,
}

impl TypeLowerer<'_> {
    /// Return cached interface reference layout metadata.
    pub(crate) fn interface_ref_layout(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<&InterfaceRefLayout> {
        self.interface_ref_cache.get(&type_id)
    }

    /// Lower an interface reference type into a fat pointer struct.
    pub(crate) fn lower_interface_reference_type(
        &mut self,
        types: &dir::TypeTable,
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
        let dir::Type::Reference { symbol, .. } = types.get_type(type_id) else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "expected interface reference type".to_string(),
            }
            .into());
        };

        // require an interface symbol
        if symbol.ty() != dir::SymbolType::Interface {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "expected interface reference type".to_string(),
            }
            .into());
        }

        // define object and itab field names and types
        let object_name = builder.intern("object");
        let itab_name = builder.intern("itab");
        let object_type = builder.type_managed_reference(self.ty_void);
        let itab_type = builder.type_reference(
            mir::ReferenceKind::Raw,
            self.ty_void,
            mir::Mutability::Immutable,
            mir::AddressSpace::Static,
            false,
        );

        // compute field sizes and alignments
        let (object_size, object_alignment) = self
            .size_and_align_of_type(builder.tree().get(object_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "interface layout requires concrete nested types".to_string(),
            })?;
        let (itab_size, itab_alignment) = self
            .size_and_align_of_type(builder.tree().get(itab_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "interface layout requires concrete nested types".to_string(),
            })?;

        // assemble field inputs
        let fields = vec![
            FieldInput {
                name: object_name,
                ty: object_type,
                size: object_size,
                alignment: object_alignment,
                source_index: Some(0),
                kind: FieldLayoutKind::Synthetic,
            },
            FieldInput {
                name: itab_name,
                ty: itab_type,
                size: itab_size,
                alignment: itab_alignment,
                source_index: Some(1),
                kind: FieldLayoutKind::Synthetic,
            },
        ];

        // compute layout and create the mir struct type
        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Source);
        let mir_type = self.create_struct_type(&layout, builder);
        self.layout_cache.insert(mir_type, layout.clone());

        // resolve field indices for interface metadata
        let object_field_index =
            layout
                .field_index(object_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing interface object field".to_string(),
                })?;
        let itab_field_index =
            layout
                .field_index(itab_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing interface itab field".to_string(),
                })?;

        // cache interface reference metadata
        self.interface_ref_cache.insert(
            type_id,
            InterfaceRefLayout {
                object_field_index,
                itab_field_index,
                object_type,
                itab_type,
            },
        );

        Ok(mir_type)
    }
}
