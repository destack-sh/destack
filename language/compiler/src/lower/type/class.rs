use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult};

impl TypeLowerer<'_, '_> {
    /// Lower one class declaration to its managed reference nominal.
    pub(in crate::lower) fn lower_class(
        &mut self,
        definition: dir::ClassDefinition,
        ty: mir::TypeId,
    ) -> CompilerResult<Vec<NominalField>> {
        // embed the base storage first, grounded through its heritage application
        let mut fields = Vec::new();
        let mut field_nodes = Vec::new();
        if let Some(heritage) = &definition.extends {
            let site = self.node_provenance(heritage.source)?;

            // lower the applied base and copy its stored fields
            let base = self.lowerer.instance_type(self.instance, heritage.ty)?;
            let base = self.lowerer.peel_owned(base)?;
            let dir::Type::Application(_) = self.lowerer.ty(base)? else {
                return Err(CompilerError::Internal {
                    message: "a class heritage outside an application type".to_string(),
                });
            };
            let base = self.lower_nominal(base)?;
            let mut base_fields = self.lowerer.nominal(&base.key)?.fields.clone();

            // read each base field representation
            let storage = base.storage;
            let (storage, _) = self.tree.split_lifetime_application(storage);
            let mir::Type::Struct {
                fields: base_nodes, ..
            } = self.tree.ty(storage)
            else {
                return Err(CompilerError::Internal {
                    message: "a class heritage without struct storage".to_string(),
                });
            };

            // derive the inherited fields under this declaration
            for field in &mut base_fields {
                field.provenance = self.provenance.expand(field.provenance, site);
            }
            fields.extend(base_fields);
            field_nodes.extend(base_nodes.iter().cloned());
        }

        // lower each own field's type into a field node
        for definition in ModuleLowerer::instance_fields(&definition.members) {
            let provenance = self.node_provenance(definition.source)?;
            let ty = self.lowerer.symbol_type(definition.symbol)?;
            let ty = self.lower(ty)?;
            let name = match definition.key {
                dir::StaticKey::Name(name) => Some(name),
                _ => None,
            };

            field_nodes.push(mir::Field {
                name,
                ty,
                attributes: Vec::new(),
            });
            fields.push(NominalField::new(definition, provenance));
        }

        // store instances behind a reference without copying in place
        self.tree.define_type(
            ty,
            mir::Type::Struct {
                fields: field_nodes,
                copy: mir::Copy::No,
            },
        );

        Ok(fields)
    }

    /// Insert one managed local reference over a pointee type.
    pub(in crate::lower) fn insert_managed_reference(
        &mut self,
        pointee: mir::TypeId,
    ) -> mir::TypeId {
        self.insert_reference(mir::ReferenceKind::Managed, mir::Access::Mutable, pointee)
    }
}
