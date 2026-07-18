use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, Nominal};
use crate::CompilerResult;

impl ModuleLowerer<'_> {
    /// Lower one struct declaration to its MIR type.
    pub(in crate::lower) fn lower_struct(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        definition: dir::StructDefinition,
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

        // conformance decides whether values copy or move
        let copy = match self.conforms(symbol, dir::AutoInterface::Copy)? {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };
        let ty = builder.tree_mut().insert(mir::Type::Struct {
            fields: field_nodes,
            copy,
        });

        Ok(Nominal {
            ty,
            value: ty,
            fields,
        })
    }
}
