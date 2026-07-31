use destack_dir as dir;
use destack_mir as mir;

use super::lower::TypeLowerer;
use crate::{CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one structural constraint to its erased dynamic carrier.
    pub(in crate::lower) fn lower_dynamic(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let constraint = self.lower_dynamic_constraint(constraint)?;

        Ok(self.tree.intern_type(mir::Type::Dynamic {
            kind: mir::ReferenceKind::Managed,
            lifetime: mir::Lifetime::empty(),
            constraint,
            storage: mir::Storage::Heap(mir::Space::Local),
            access: mir::Access::Mutable,
            nullability: mir::Nullability::None,
        }))
    }

    /// Lower one constraint to its canonical shape type.
    pub(in crate::lower) fn lower_dynamic_constraint(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // read the properties the constraint declares
        let constraint = self.lowerer.reduced_type(constraint)?;
        let properties = match self.lowerer.ty(constraint)? {
            dir::Type::Shape(shape) => self
                .lowerer
                .types(constraint.module_id)?
                .properties(shape.properties)
                .to_vec(),
            // leave the top constraint without entries
            dir::Type::Unknown => Vec::new(),
            // dispatch interface instances through their declared members
            dir::Type::Application(instance)
                if matches!(
                    self.lowerer.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(_))
                ) =>
            {
                let arguments = self
                    .lowerer
                    .types(constraint.module_id)?
                    .type_ids(instance.arguments)
                    .to_vec();

                return Ok(self.lower_nominal(instance.symbol, &arguments)?.storage);
            }
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a '{}' dynamic constraint", other.variant_name()),
                }
                .into());
            }
        };

        // build the constraint's fields and dispatch shape
        let mut fields = Vec::with_capacity(properties.len());
        let mut slots = Vec::with_capacity(properties.len());
        for property in &properties {
            let dir::StaticKey::Name(name) = property.key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a computed dynamic constraint key".to_string(),
                }
                .into());
            };
            let ty = self.lower_property_carrier(property)?;
            let field = self.tree.intern_field(
                mir::Field {
                    name: Some(name),
                    ty: mir::TypeId::from(ty),
                },
                Vec::new(),
            );
            fields.push(field);
            slots.push(mir::DynamicSlot::Field { field, name });
        }
        let copy = mir::Copy::No;
        let struct_type = self.tree.intern_type(mir::Type::Struct { fields, copy });

        // register the constraint's dispatch shape once
        self.lowerer
            .dynamic_shapes
            .entry(struct_type)
            .or_insert(mir::DynamicShape {
                constraint: struct_type,
                slots,
            });

        Ok(struct_type)
    }
}
