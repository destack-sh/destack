use tspp_core::StringId;
use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::TypeLowerer;
use crate::{CompilerError, CompilerResult};

impl TypeLowerer<'_, '_> {
    /// Lower one associated type projection.
    pub(in crate::lower) fn lower_associated_type(
        &mut self,
        qualifier: dir::GlobalTypeId,
        member: StringId,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        // read the interface application the projection reads through
        let Some(symbol) = self.lower.nominal_symbol(qualifier)? else {
            return Err(CompilerError::Internal {
                message: "an associated type qualifier outside an interface".to_string(),
            });
        };
        let interface = self.lower_nominal(qualifier)?.storage;
        let receiver = self.lower(owner)?;

        // an erased receiver takes the value the interface declares for the associated type
        if matches!(self.tree.get(receiver), mir::Type::Dynamic { .. })
            && let Some(declared) = self.declared_associated_value(symbol, member)?
        {
            return Ok(declared);
        }

        Ok(self.tree.intern_type(mir::Type::Witness {
            receiver,
            interface,
            member,
        }))
    }

    /// Lower the value one interface declares for an associated type.
    fn declared_associated_value(
        &mut self,
        interface: dir::GlobalSymbolId,
        member: StringId,
    ) -> CompilerResult<Option<mir::TypeId>> {
        // only an interface definition declares associated values
        let Some(dir::Definition::Interface(definition)) = self.lower.definition(interface)? else {
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
}
