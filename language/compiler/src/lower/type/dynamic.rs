use destack_dir as dir;
use destack_mir as mir;

use super::lower::TypeLowerer;
use crate::{CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one structural constraint to its erased dynamic reference.
    pub(in crate::lower) fn lower_dynamic(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        let constraint = self.lower_dynamic_constraint(constraint)?;

        Ok(self.dynamic_over(constraint))
    }

    /// Lower one open shape, type algebra over a parameter, to the top dynamic reference.
    pub(in crate::lower) fn lower_open_dynamic(&mut self) -> CompilerResult<mir::TypeId> {
        let constraint = self.dynamic_shape_constraint(Vec::new(), false)?;

        Ok(self.dynamic_over(constraint))
    }

    /// Reference one constraint shape through the erased dynamic representation.
    fn dynamic_over(&mut self, constraint: mir::TypeId) -> mir::TypeId {
        self.tree.intern_type(mir::Type::Dynamic {
            kind: mir::Reference::Managed(mir::Space::Local),
            lifetime: mir::Lifetime::empty(),
            constraint,
            access: mir::Access::Mutable,
        })
    }

    /// Lower one constraint to its canonical shape type.
    pub(in crate::lower) fn lower_dynamic_constraint(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        // read the properties the constraint declares
        let (properties, is_keyed) = match self.lower.ty(constraint)? {
            dir::Type::Object(shape) => {
                let properties = self
                    .lower
                    .types(constraint.module_id)?
                    .properties(shape.properties)
                    .to_vec();

                (properties, !shape.index_signatures.is_empty())
            }
            // leave the top constraint empty
            dir::Type::Unknown => (Vec::new(), false),
            // leave a parameter's constraint to its argument
            dir::Type::Parameter(_) => return self.lower(constraint),
            // dispatch interface instances through their declared members
            dir::Type::Application(instance)
                if matches!(
                    self.lower.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(_))
                ) =>
            {
                return Ok(self.lower_nominal(constraint)?.storage);
            }
            // reject every other constraint head
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("a '{}' dynamic constraint", other.variant_name()),
                }
                .into());
            }
        };

        self.dynamic_shape_constraint(properties, is_keyed)
    }

    /// Intern one constraint shape over its declared properties, registering its dispatch shape.
    fn dynamic_shape_constraint(
        &mut self,
        properties: Vec<dir::TypeProperty>,
        is_keyed: bool,
    ) -> CompilerResult<mir::TypeId> {
        // build the constraint's fields and dispatch shape
        let mut fields = Vec::with_capacity(properties.len());
        let mut slots = Vec::with_capacity(properties.len());
        for property in &properties {
            let dir::StaticKey::Name(name) = property.key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a computed dynamic constraint key".to_string(),
                }
                .into());
            };

            // intern the property as a field and its dispatch slot
            let ty = self.lower_property_representation(property)?;
            let field = self.tree.intern_field(mir::Field {
                name: Some(name),
                ty,
                attributes: Vec::new(),
            });
            fields.push(field);
            slots.push(mir::DynamicSlot::Field { field, name });
        }

        // intern the constraint's fields as one non-copy struct
        let struct_type = self.tree.intern_type(mir::Type::Struct { fields });

        // register the constraint's dispatch shape once
        self.lower
            .dynamic_shapes
            .entry(struct_type)
            .or_insert(mir::DynamicShape {
                constraint: struct_type,
                slots,
                is_keyed,
            });

        Ok(struct_type)
    }
}
