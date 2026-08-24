use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult};

impl TypeLowerer<'_, '_> {
    /// Lower one class declaration to its managed reference nominal.
    pub(in crate::lower) fn lower_class(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::ClassDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // embed the base storage first, grounded through its heritage application
        let fields = self.lowerer.nominal_fields(symbol)?;
        let mut field_nodes = Vec::with_capacity(fields.len());
        if let Some(heritage) = &definition.extends {
            let base = self.lowerer.instance_type(self.instance, heritage.ty)?;
            let base = self.lowerer.peel_owned(base)?;
            let dir::Type::Application(_) = self.lowerer.ty(base)? else {
                return Err(CompilerError::Internal {
                    message: "a class heritage outside an application type".to_string(),
                });
            };
            let storage = self.lower_nominal(base)?.storage;
            let (storage, _) = self.tree.split_lifetime_application(storage);
            let mir::Type::Struct {
                fields: base_nodes, ..
            } = self.tree.get(storage)
            else {
                return Err(CompilerError::Internal {
                    message: "a class heritage without struct storage".to_string(),
                });
            };
            field_nodes.extend(base_nodes.iter().copied());
        }

        // lower each own field's type into a field node
        let own = self.lowerer.instance_fields(&definition.members);
        for field in &own {
            let ty = self.lowerer.symbol_type(field.symbol)?;
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
