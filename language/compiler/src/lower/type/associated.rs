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

    /// Lower one associated const projection to its witness value.
    pub(in crate::lower) fn lower_associated_value(
        &mut self,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::StaticId>> {
        // require a named projection
        let dir::Type::Member(member) = self.lower.ty(argument)? else {
            return Ok(None);
        };
        let member = *self.lower.types(argument.module_id)?.member(member);
        let dir::StaticKey::Name(name) = member.key else {
            return Ok(None);
        };

        // read the interface of a this projection
        let qualifier = match member.qualifier {
            Some(qualifier) => qualifier,
            None => {
                let (dir::Type::This, Some(receiver)) =
                    (self.lower.ty(member.owner)?, self.scope.receiver)
                else {
                    return Ok(None);
                };
                let Some(parameter) = self
                    .scope
                    .parameters
                    .iter()
                    .find_map(|(parameter, index)| (*index == receiver).then_some(*parameter))
                else {
                    return Err(CompilerError::Internal {
                        message: "an interface receiver index without its parameter".to_string(),
                    });
                };
                let generics = &self.lower.state(parameter.module_id)?.generics;
                let Some(constraint) = generics.get_parameter(parameter.local_id).constraint else {
                    return Err(CompilerError::Internal {
                        message: "an interface receiver without its interface".to_string(),
                    });
                };

                constraint
            }
        };

        // name the witness const
        let interface = self.lower_nominal(qualifier)?.storage;
        let receiver = self.lower(member.owner)?;
        let value = mir::Static::Witness {
            receiver,
            interface,
            member: name,
        };

        Ok(Some(self.tree.intern_static(value)))
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
