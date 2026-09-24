use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use super::lower::TypeLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one anonymous object type to its struct storage.
    pub(in crate::lower) fn lower_object_struct(
        &mut self,
        shape: &dir::ObjectType,
        module: ModuleId,
    ) -> CompilerResult<mir::TypeId> {
        let fields = self.object_fields(shape, module)?;

        Ok(self.tree.intern_type(mir::Type::Struct { fields }))
    }

    /// Lower each written property into a named field.
    fn object_fields(
        &mut self,
        shape: &dir::ObjectType,
        module: ModuleId,
    ) -> CompilerResult<Vec<mir::FieldId>> {
        // read the properties the shape declares
        let properties = self
            .lower
            .types(module)?
            .properties(shape.properties)
            .to_vec();

        // intern one named field per written property
        let mut fields = Vec::with_capacity(properties.len());
        for property in &properties {
            let dir::StaticKey::Name(name) = property.key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a computed object property".to_string(),
                }
                .into());
            };
            let ty = self.lower_property_representation(property)?;
            fields.push(self.tree.intern_field(mir::Field {
                name: Some(name),
                ty,
                attributes: Vec::new(),
            }));
        }

        Ok(fields)
    }

    /// Lower one object property to its stored representation.
    pub(in crate::lower) fn lower_property_representation(
        &mut self,
        property: &dir::TypeProperty,
    ) -> CompilerResult<mir::TypeId> {
        let Some(read) = property.access.read() else {
            return Err(CompilerError::Internal {
                message: "an object property without a read type".to_string(),
            });
        };

        self.optional_storage_representation(read, property.is_optional)
    }

    /// Lower one property's storage, holding undefined beside an optional property's value.
    pub(in crate::lower) fn optional_storage_representation(
        &mut self,
        ty: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<mir::TypeId> {
        if !is_optional {
            return self.lower(ty);
        }

        // keep a variant already holding an undefined case
        let value = self.lower(ty)?;
        let cases = match self.tree.get(value) {
            mir::Type::Variant { cases, .. } => cases.iter().map(|case| case.ty).collect(),
            _ => Vec::new(),
        };
        if cases
            .iter()
            .any(|case| matches!(self.tree.get(*case), mir::Type::Void))
        {
            return Ok(value);
        }

        // grow a variant over its own cases, else beside the value
        let mut payloads = if cases.is_empty() { vec![value] } else { cases };
        payloads.push(self.tree.intern_type(mir::Type::Void));

        Ok(self.insert_union_variant(payloads))
    }
}
