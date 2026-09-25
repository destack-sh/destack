use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult};

impl TypeLowerer<'_, '_> {
    /// Lower one class declaration to its managed reference nominal.
    pub(in crate::lower) fn lower_class(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::ClassDefinition,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
    ) -> CompilerResult<Vec<NominalField>> {
        // gather the nominal fields, inherited and own
        let fields = self.lower.nominal_fields(symbol)?;
        let mut field_nodes = Vec::with_capacity(fields.len());

        // embed the base storage first, grounded through its heritage application
        if let Some(heritage) = &definition.extends {
            let base = heritage.ty;
            let base = self.lower.stored(base)?;
            let dir::Type::Application(_) = self.lower.ty(base)? else {
                return Err(CompilerError::Internal {
                    message: "a class heritage outside an application type".to_string(),
                });
            };
            let storage = self.lower_nominal(base)?.storage;
            self.lower.fill_heritage(self.tree, storage)?;
            let storage = mir::Substitution::resolve(storage, self.tree);
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

        // lower each declared field's stored type into a field node
        let declared = self.lower.instance_fields(&definition.members)?;
        for field in &declared {
            let ty = self.optional_storage_representation(field.ty, field.is_optional)?;
            let name = match field.key {
                dir::StaticKey::Name(name) => Some(name),
                _ => None,
            };

            field_nodes.push(self.tree.intern_field(mir::Field {
                name,
                ty,
                attributes: Vec::new(),
            }));
        }

        // define the reference storage from its field nodes
        let definition = self.tree.intern_type(mir::Type::Struct {
            fields: field_nodes,
        });
        self.tree.get_mut(declaration).definition = Some(definition);

        Ok(fields)
    }
}
