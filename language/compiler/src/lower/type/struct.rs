use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{NominalField, TypeLowerer};

impl TypeLowerer<'_, '_> {
    /// Lower one struct declaration to its representation.
    pub(in crate::lower) fn lower_struct(
        &mut self,
        _symbol: dir::GlobalSymbolId,
        definition: dir::StructDefinition,
        ty: mir::LocalNodeId<mir::Type>,
        copy: mir::Copy,
    ) -> CompilerResult<Vec<NominalField>> {
        // gather the instance fields in declaration order
        let fields = self.lowerer.instance_fields(&definition.members);

        // lower each field's type into a field node
        let mut field_nodes = Vec::with_capacity(fields.len());
        for field in &fields {
            let ty = self.lowerer.symbol_type(field.symbol)?;
            let mut ty = self.lower(ty)?;

            // widen optional fields so their absent case stores as undefined
            if field.is_optional {
                ty = self.insert_optional_representation(ty)?;
            }

            // intern the field name when it has text
            let name = match field.key {
                dir::StaticKey::Name(name) => Some(name),
                _ => None,
            };
            field_nodes.push(self.tree.intern_field(mir::Field { name, ty }, Vec::new()));
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
