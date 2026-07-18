use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, Nominal};
use crate::CompilerResult;

impl ModuleLowerer<'_> {
    /// Lower one class declaration to its managed reference nominal.
    pub(in crate::lower) fn lower_class(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        definition: dir::ClassDefinition,
        pointee: mir::LocalNodeId<mir::Type>,
        value: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Nominal> {
        // gather the instance fields in declaration order
        let fields = Self::instance_fields(&definition.members);

        // lower each field's checked type into a MIR field node
        let mut field_nodes = Vec::with_capacity(fields.len());
        for field in &fields {
            let ty = self.symbol_type(field.symbol.into_global(symbol.module_id))?;
            let ty = self.lower_nominal_type(builder, ty)?;
            let name = match field.key {
                dir::StaticKey::Name(name) => Some(name),
                _ => None,
            };

            field_nodes.push(builder.tree_mut().insert(mir::Field { name, ty }));
        }

        // instances store behind a reference and never copy in place
        builder.tree_mut().set(
            pointee,
            mir::Type::Struct {
                fields: field_nodes,
                copy: mir::Copy::No,
            },
        );

        Ok(Nominal {
            ty: pointee,
            value,
            fields,
        })
    }

    /// Insert one managed local reference over a pointee type.
    pub(in crate::lower) fn managed_reference(
        &self,
        tree: &mut mir::Tree,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.insert_reference(tree, mir::ReferenceKind::Managed, mir::Access::Mutable, pointee)
    }
}
