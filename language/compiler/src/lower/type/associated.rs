use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult};

impl TypeLowerer<'_, '_> {
    /// Lower one associated type projection.
    pub(in crate::lower) fn lower_associated_type(
        &mut self,
        qualifier: dir::GlobalTypeId,
        member: StringId,
        base: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // read the interface application the projection reads through
        let symbol = match self.lower.ty(qualifier)? {
            dir::Type::Application(application) => application.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => {
                return Err(CompilerError::Internal {
                    message: "an associated type qualifier outside an interface".to_string(),
                });
            }
        };

        // split the lowered interface into its declaration and its arguments
        let interface = self.lower_nominal(qualifier)?.storage;
        let (interface_base, mut arguments) = match self.tree.get(interface) {
            mir::Type::Application {
                base, arguments, ..
            } => (*base, arguments.clone()),
            _ => (mir::TypeId::from(interface), Vec::new()),
        };

        // an erased base takes the value the interface declares for the associated type
        if matches!(self.tree.get(base), mir::Type::Dynamic { .. })
            && let Some(declared) = self.declared_associated_value(symbol, member)?
        {
            return Ok(declared);
        }

        // apply the associated declaration to the interface's arguments and the base
        let declaration = self.associated_declaration(symbol, member, interface_base)?;
        arguments.push(mir::GenericArgument::Type(mir::TypeId::from(base)));

        Ok(self.tree.intern_type(mir::Type::Application {
            base: mir::TypeId::from(declaration),
            arguments,
        }))
    }

    /// Lower the value one interface declares for an associated type.
    fn declared_associated_value(
        &mut self,
        interface: dir::GlobalSymbolId,
        member: StringId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // only an interface definition declares associated values
        let Some(dir::Definition::Interface(definition)) =
            self.lower.definition(interface)?.cloned()
        else {
            return Ok(None);
        };

        // find the member's declared value, a bare requirement declaring none
        let value = definition
            .members
            .iter()
            .find_map(|declared| match declared {
                dir::DefinitionMember::AssociatedType(associated)
                    if associated.key == dir::StaticKey::Name(member) =>
                {
                    associated.value
                }
                _ => None,
            });
        let Some(value) = value else {
            return Ok(None);
        };

        Ok(Some(self.lower(value)?))
    }

    /// Declare one interface's associated type once.
    fn associated_declaration(
        &mut self,
        interface: dir::GlobalSymbolId,
        member: StringId,
        interface_base: mir::TypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // reuse the declaration a previous projection made
        if let Some(declared) = self.lower.associated_types.get(&(interface, member)) {
            return Ok(*declared);
        }

        // name the declaration after the interface and member
        let member_name = self.lower.strings.get(member).to_string();
        let path = format!("{}.{member_name}", self.lower.symbol_path(interface)?);
        let name = self.lower.strings.intern(&path);
        let symbol = mir::Symbol::declared(
            interface.module_id,
            name,
            ModuleLowerer::symbol_identity(interface),
        );
        let ty = self.tree.reserve_type(symbol);

        // keep the declaration an import already brought for the placeholder
        if self.tree.type_declaration(ty).is_some() {
            self.lower.associated_types.insert((interface, member), ty);

            return Ok(ty);
        }

        // take the interface's parameters and add the implementing type last
        let mut generics = match self.tree.type_declaration(interface_base) {
            Some(declaration) => self.tree.get(declaration).generics.clone(),
            None => Vec::new(),
        };
        generics.push(mir::GenericParameter {
            name: self.lower.strings.intern("this"),
            domain: mir::GenericParameterDomain::Type { bounds: Vec::new() },
        });

        // declare the opaque template and remember it for later projections
        self.tree
            .insert_type_declaration(name, generics, ty, mir::TypeHeritage::default());
        self.lower.associated_types.insert((interface, member), ty);

        Ok(ty)
    }
}
