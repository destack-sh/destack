use destack_dir as dir;

use crate::check::{Answer, CheckState, Origin, answer};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the declared member domain of one value enum type.
    pub(in crate::check) fn enum_discriminant_domain(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        // the value names an enum declaration instance
        let dir::Type::Instance(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Enum(definition)) = self.definition(instance.symbol)?.cloned()
        else {
            return Ok(None);
        };

        // every member contributes its sealed discriminant value
        let mut domain = Vec::new();
        for member in &definition.members {
            let dir::DefinitionMember::Variant(variant) = member else {
                continue;
            };
            domain.push(self.enum_member_discriminant(variant.symbol)?);
        }

        Ok(Some(domain))
    }

    /// Return the sealed discriminant value of one enum member symbol.
    pub(in crate::check) fn enum_member_discriminant(
        &mut self,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::ScalarLiteral> {
        let Some(value) = self.static_value(member) else {
            return Err(CompilerError::Internal {
                message: format!("enum member {member:?} is missing its discriminant value"),
            });
        };
        let dir::Type::Literal(literal) = self.ty(value)? else {
            return Err(CompilerError::Internal {
                message: format!("enum member {member:?} sealed a non-literal discriminant"),
            });
        };

        Ok(literal)
    }

    /// Return the discriminant domain of one variant-shaped type: enum or tagged.
    pub(in crate::check) fn variant_discriminant_domain(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::ScalarLiteral>>>> {
        let value = answer!(self.reduce_type_head(origin, value)?);

        // enums discriminate over their declared members
        if let Some(domain) = self.enum_discriminant_domain(value)? {
            return Ok(Answer::Ready(Some(domain)));
        }

        self.tagged_discriminant_domain(origin, value)
    }

    /// Return the declared member selected by one static key.
    pub(in crate::check) fn enum_member_with_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(dir::Definition::Enum(definition)) = self.definition(symbol)?.cloned() else {
            return Ok(None);
        };

        Ok(definition.members.iter().find_map(|member| match member {
            dir::DefinitionMember::Variant(variant) if variant.key == key => Some(variant.symbol),
            _ => None,
        }))
    }

    /// Return the member key selected by one enum discriminant.
    pub(in crate::check) fn enum_case_key_from_discriminant(
        &mut self,
        value: dir::GlobalTypeId,
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // the value names an enum declaration instance
        let dir::Type::Instance(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Enum(definition)) = self.definition(instance.symbol)?.cloned()
        else {
            return Ok(None);
        };

        // name the member sealing this discriminant
        for member in &definition.members {
            let dir::DefinitionMember::Variant(variant) = member else {
                continue;
            };
            if self.enum_member_discriminant(variant.symbol)? == discriminant {
                return Ok(Some(variant.key));
            }
        }

        Ok(None)
    }
}
