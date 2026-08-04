use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use super::lower::TypeLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one anonymous object row to its struct storage.
    pub(in crate::lower) fn lower_object_struct(
        &mut self,
        shape: &dir::ShapeType,
        module: ModuleId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let fields = self.object_row_fields(shape, module)?;

        Ok(self.tree.intern_type(mir::Type::Struct {
            fields,
            copy: mir::Copy::No,
        }))
    }

    /// Define one declared object row into its reserved alias identity.
    pub(in crate::lower) fn define_object_row(
        &mut self,
        shape: &dir::ShapeType,
        module: ModuleId,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        let fields = self.object_row_fields(shape, module)?;
        self.tree.define_type(
            ty,
            mir::Type::Struct {
                fields,
                copy: mir::Copy::No,
            },
        );

        Ok(())
    }

    /// Lower each written row property into a named field.
    fn object_row_fields(
        &mut self,
        shape: &dir::ShapeType,
        module: ModuleId,
    ) -> CompilerResult<Vec<mir::LocalNodeId<mir::Field>>> {
        let properties = self
            .lowerer
            .types(module)?
            .properties(shape.properties)
            .to_vec();

        let mut fields = Vec::with_capacity(properties.len());
        for property in &properties {
            let dir::StaticKey::Name(name) = property.key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a computed object property".to_string(),
                }
                .into());
            };
            let ty = self.lower_property_carrier(property)?;
            fields.push(self.tree.intern_field(
                mir::Field {
                    name: Some(name),
                    ty: mir::TypeId::from(ty),
                },
                Vec::new(),
            ));
        }

        Ok(fields)
    }

    /// Lower one row property to its stored carrier.
    ///
    /// Absent optional properties store as undefined: reference carriers
    /// ride their nullability niche, value carriers grow a variant case.
    pub(in crate::lower) fn lower_property_carrier(
        &mut self,
        property: &dir::TypeProperty,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let Some(read) = property.access.read() else {
            return Err(CompilerError::Internal {
                message: "a row property without a read type".to_string(),
            });
        };
        let value = self.lower(read)?;
        if !property.is_optional {
            return Ok(value);
        }

        // niche optional reference carriers in their spare values
        let nullability = match self.tree.get(value) {
            mir::Type::Reference { nullability, .. }
            | mir::Type::Slice { nullability, .. }
            | mir::Type::Dynamic { nullability, .. }
            | mir::Type::Function { nullability, .. } => Some(*nullability),
            _ => None,
        };
        if let Some(nullability) = nullability {
            let nullability = match nullability {
                mir::Nullability::None | mir::Nullability::Undefined => mir::Nullability::Undefined,
                mir::Nullability::Null | mir::Nullability::NullOrUndefined => {
                    mir::Nullability::NullOrUndefined
                }
            };

            return self.insert_nullability(value, nullability);
        }

        // grow a variant case for optional value carriers
        let undefined = self.tree.intern_type(mir::Type::Void);

        Ok(self.insert_union_variant(vec![value, undefined]))
    }
}
