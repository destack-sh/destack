use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{NominalField, TypeLowerer};

impl TypeLowerer<'_, '_> {
    /// Lower one class declaration to its managed reference nominal.
    pub(in crate::lower) fn lower_class(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::ClassDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // gather the instance fields in declaration order
        let fields = self.lowerer.instance_fields(&definition.members);

        // lower each field's type into a field node
        let mut field_nodes = Vec::with_capacity(fields.len());
        for field in &fields {
            let ty = self
                .lowerer
                .symbol_type(field.symbol.into_global(symbol.module_id))?;
            let ty = self.lower(ty)?;
            let name = match field.key {
                dir::StaticKey::Name(name) => Some(name),
                _ => None,
            };

            field_nodes.push(self.tree.intern_field(mir::Field { name, ty }, Vec::new()));
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
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.insert_reference(mir::ReferenceKind::Managed, mir::Access::Mutable, pointee)
    }
}
