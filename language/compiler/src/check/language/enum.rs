use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Return the singleton member types of one value enum.
    pub(in crate::check) fn enum_member_types(
        &mut self,
        module: ModuleId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let dir::Type::Instance(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Enum(definition)) = self.definition(instance.symbol)? else {
            return Ok(None);
        };
        let members = definition
            .variants()
            .map(|variant| variant.symbol)
            .collect::<Vec<_>>();

        let mut variants = Vec::with_capacity(members.len());
        for member in members {
            let variant = self.intern_type(
                module,
                dir::Type::EnumMember(dir::EnumMemberType {
                    owner: value,
                    member,
                }),
            )?;
            variants.push(variant);
        }

        Ok(Some(variants))
    }

    /// Return the declared member domain of one value enum type.
    pub(in crate::check) fn enum_discriminant_domain(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        // the value names an enum declaration instance
        let dir::Type::Instance(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Enum(definition)) = self.definition(instance.symbol)? else {
            return Ok(None);
        };

        // every variant contributes its checked discriminant value
        let domain = definition
            .variants()
            .map(|variant| dir::ScalarLiteral::from(variant.value))
            .collect();

        Ok(Some(domain))
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
        let Some(dir::Definition::Enum(definition)) = self.definition(instance.symbol)? else {
            return Ok(None);
        };

        // name the variant carrying this discriminant
        for variant in definition.variants() {
            if dir::ScalarLiteral::from(variant.value) == discriminant {
                return Ok(Some(variant.key));
            }
        }

        Ok(None)
    }
}
