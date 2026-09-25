use tspp_dir as dir;

use crate::sema::CheckState;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return whether one checked type has exactly one possible value.
    pub(in crate::sema) fn is_singleton_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // accept the types whose inhabitants their own shape already fixes
        let ty = self.shallow_resolve(ty)?;
        if matches!(
            self.ty(ty)?,
            dir::Type::Void
                | dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(_)
                | dir::Type::Key(_)
                | dir::Type::Static(_)
        ) {
            return Ok(true);
        }

        // leave every type outside a precise variant open
        let dir::Type::Variant(variant) = self.ty(ty)? else {
            return Ok(false);
        };
        let owner = self.variant_owner(&variant)?;

        // treat each enum variant as exactly one value
        match self.definition(owner.symbol)?.as_deref() {
            Some(dir::Definition::Enum(_)) => Ok(true),
            _ => Err(CompilerError::Internal {
                message: format!("variant type {ty:?} has non-enum owner {:?}", owner.symbol),
            }),
        }
    }

    /// Return the precise variants declared by one enum owner.
    pub(in crate::sema) fn variant_types(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        // require the value to name an enum declaration instance
        let dir::Type::Application(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Enum(definition)) = declared.as_deref() else {
            return Ok(None);
        };
        let variants = definition
            .variants()
            .map(|variant| variant.symbol)
            .collect::<Vec<_>>();

        // intern one precise variant type per declared member
        let mut types = Vec::with_capacity(variants.len());
        for variant in variants {
            let variant = self.intern_type(dir::Type::Variant(dir::VariantType {
                owner: value,
                variant,
            }))?;
            types.push(variant);
        }

        Ok(Some(types))
    }

    /// Return the declared member domain of one value enum type.
    fn enum_discriminant_domain(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::Literal>>> {
        // require the value to name an enum declaration instance
        let dir::Type::Application(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Enum(definition)) = declared.as_deref() else {
            return Ok(None);
        };

        // collect each variant's checked discriminant value
        let domain = definition
            .variants()
            .map(|variant| dir::Literal::from(variant.value))
            .collect();

        Ok(Some(domain))
    }

    /// Return the discriminant domain of one enum type.
    pub(in crate::sema) fn variant_discriminant_domain(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::Literal>>> {
        // read only the selected discriminant out of a case-specific type
        if let dir::Type::Variant(variant) = self.ty(value)? {
            let discriminant = self.variant_discriminant(&variant)?;

            return Ok(Some(vec![discriminant]));
        }

        self.enum_discriminant_domain(value)
    }

    /// Return the discriminant carried by one case-specific type.
    fn variant_discriminant(&mut self, variant: &dir::VariantType) -> CompilerResult<dir::Literal> {
        // read the enum owning the case
        let owner = self.variant_owner(variant)?;

        // read the declared discriminant from the owning enum
        let discriminant = match self.definition(owner.symbol)?.as_deref() {
            Some(dir::Definition::Enum(definition)) => definition
                .variants()
                .find(|member| member.symbol == variant.variant)
                .map(|member| dir::Literal::from(member.value)),
            _ => None,
        };
        let Some(discriminant) = discriminant else {
            return Err(CompilerError::Internal {
                message: format!(
                    "variant {:?} is missing from owner {:?}",
                    variant.variant, owner.symbol
                ),
            });
        };

        Ok(discriminant)
    }

    /// Return the declaration application owning one precise variant type.
    fn variant_owner(&self, variant: &dir::VariantType) -> CompilerResult<dir::GenericApplication> {
        // require an application owner
        let dir::Type::Application(owner) = self.ty(variant.owner)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "variant {:?} has non-application owner {:?}",
                    variant.variant, variant.owner
                ),
            });
        };

        Ok(owner)
    }

    /// Return the member key selected by one enum discriminant.
    pub(in crate::sema) fn enum_case_key_from_discriminant(
        &mut self,
        mut value: dir::GlobalTypeId,
        discriminant: dir::Literal,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // step from a case-specific type up to the enum owning it
        if let dir::Type::Variant(variant) = self.ty(value)? {
            value = variant.owner;
        }

        // require the value to name an enum declaration instance
        let dir::Type::Application(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Enum(definition)) = declared.as_deref() else {
            return Ok(None);
        };

        // name the variant carrying this discriminant
        for variant in definition.variants() {
            if dir::Literal::from(variant.value) == discriminant {
                return Ok(Some(variant.key));
            }
        }

        Ok(None)
    }
}
