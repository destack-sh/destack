use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{ModuleLowerer, NominalField, TypeLowerer};

impl TypeLowerer<'_, '_> {
    /// Lower one struct declaration to its representation.
    pub(in crate::lower) fn lower_struct(
        &mut self,
        definition: dir::StructDefinition,
        ty: mir::TypeId,
        copy: mir::Copy,
    ) -> CompilerResult<Vec<NominalField>> {
        // gather the instance fields in declaration order
        let mut fields = Vec::with_capacity(definition.members.len());

        // lower each field's type into a field node
        let mut field_nodes = Vec::with_capacity(definition.members.len());
        for definition in ModuleLowerer::instance_fields(&definition.members) {
            let provenance = self.node_provenance(definition.source)?;
            let ty = self.lowerer.symbol_type(definition.symbol)?;
            let mut ty = self.lower(ty)?;

            // widen optional fields so their absent case stores as undefined
            if definition.is_optional {
                ty = self.insert_optional_representation(ty)?;
            }

            // intern the field name when it has text
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

        self.tree.define_type(
            ty,
            mir::Type::Struct {
                fields: field_nodes,
                copy,
            },
        );

        Ok(fields)
    }
}
