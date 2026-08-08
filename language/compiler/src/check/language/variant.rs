use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Answer, CheckState, Origin, answer};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return whether one checked type has exactly one possible value.
    pub(in crate::check) fn is_singleton_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if matches!(
            self.ty(ty)?,
            dir::Type::Void
                | dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(_)
                | dir::Type::Key(_)
                | dir::Type::Memory(_)
                | dir::Type::Static(_)
        ) {
            return Ok(true);
        }
        let dir::Type::Variant(variant) = self.ty(ty)? else {
            return Ok(false);
        };
        let owner = self.variant_owner(&variant)?;

        // enum variants and payload-free Tagged variants each denote one value
        let is_singleton = match self.definition(owner.symbol)? {
            Some(dir::Definition::Enum(_)) => true,
            Some(dir::Definition::Newtype(definition)) if definition.is_tagged() => definition
                .tagged_variant_by_symbol(variant.variant)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "Tagged variant type {ty:?} has unknown member {:?}",
                        variant.variant
                    ),
                })?
                .argument
                .is_none(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "variant type {ty:?} has non-variant owner {:?}",
                        owner.symbol
                    ),
                });
            }
        };

        Ok(is_singleton)
    }

    /// Return the precise variants declared by one enum or Tagged owner.
    pub(in crate::check) fn variant_types(
        &mut self,
        _module: ModuleId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let dir::Type::Application(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let variants = match self.definition(instance.symbol)? {
            Some(dir::Definition::Enum(definition)) => definition
                .variants()
                .map(|variant| variant.symbol)
                .collect::<Vec<_>>(),
            Some(dir::Definition::Newtype(definition)) if definition.is_tagged() => definition
                .tagged_variants()
                .map(|variant| variant.symbol)
                .collect::<Vec<_>>(),
            _ => return Ok(None),
        };

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

    /// Return the instantiated backing selected by one Tagged variant.
    pub(in crate::check) fn tagged_variant_backing(
        &mut self,
        _module: ModuleId,
        variant: &dir::VariantType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let owner = self.variant_owner(variant)?;
        let Some(definition) = self.definition(owner.symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("variant owner {:?} has no definition", owner.symbol),
            });
        };
        let dir::Definition::Newtype(definition) = definition else {
            return Ok(None);
        };
        if !definition.is_tagged() {
            return Err(CompilerError::Internal {
                message: format!("variant owner {:?} is not Tagged", owner.symbol),
            });
        }
        let Some(backing) = definition
            .tagged_variant_by_symbol(variant.variant)
            .map(|variant| variant.backing)
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "variant {:?} is missing from Tagged owner {:?}",
                    variant.variant, owner.symbol
                ),
            });
        };

        // instantiate the declared backing under the selected owner arguments
        let substitution = self
            .instance_substitution(variant.owner.module_id, &owner)?
            .with_receiver(variant.owner);
        let backing = self.substitute_type(backing, &substitution)?;

        Ok(Some(backing))
    }

    /// Build the receiver adjustment selected by one precise Tagged variant.
    pub(in crate::check) fn variant_receiver_adjustment(
        &mut self,
        variant: &dir::VariantType,
        backing: dir::GlobalTypeId,
        projected: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ReceiverAdjustment>> {
        let owner = self.variant_owner(variant)?;
        let Some(dir::Definition::Newtype(definition)) = self.definition(owner.symbol)? else {
            return Ok(None);
        };
        if !definition.is_tagged() {
            return Err(CompilerError::Internal {
                message: format!("variant owner {:?} is not Tagged", owner.symbol),
            });
        }
        let Some(member) = definition
            .tagged_variant_by_symbol(variant.variant)
            .cloned()
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "variant {:?} is missing from Tagged owner {:?}",
                    variant.variant, owner.symbol
                ),
            });
        };
        let Some(discriminator) = definition.discriminator else {
            return Err(CompilerError::Internal {
                message: format!("Tagged owner {:?} has no discriminator", owner.symbol),
            });
        };
        let case = dir::VariantCase {
            owner: owner.symbol,
            key: member.key,
            variant: member.symbol,
        };
        let discriminant = dir::ScalarLiteral::String(member.discriminant);
        let adjustment = dir::ReceiverAdjustment::VariantPayload {
            case,
            backing,
            discriminator,
            discriminant,
            ty: projected,
        };

        Ok(Some(adjustment))
    }

    /// Return the tag projection selected by one Tagged discriminator member.
    pub(in crate::check) fn tagged_discriminator_projection(
        &mut self,
        _module: ModuleId,
        value: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::Projection>> {
        let (carrier, instance, selected) = match self.ty(value)? {
            dir::Type::Application(instance) => (value, instance, None),
            dir::Type::Variant(variant) => {
                let instance = self.variant_owner(&variant)?;

                (variant.owner, instance, Some(variant.variant))
            }
            _ => return Ok(None),
        };
        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol)? else {
            return Ok(None);
        };
        if definition.discriminator != Some(key) {
            return Ok(None);
        }
        let discriminants = match selected {
            Some(selected) => vec![
                definition
                    .tagged_variant_by_symbol(selected)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!(
                            "variant {selected:?} is missing from Tagged owner {:?}",
                            instance.symbol
                        ),
                    })?
                    .discriminant,
            ],
            None => definition
                .tagged_variants()
                .map(|variant| variant.discriminant)
                .collect(),
        };

        // preserve the source string domain while selecting its physical tag
        let mut types = Vec::with_capacity(discriminants.len());
        for discriminant in discriminants {
            let literal = dir::ScalarLiteral::String(discriminant);
            types.push(self.intern_type(dir::Type::Literal(literal))?);
        }
        let ty = self.normalized_union_type(types)?;
        let projection = dir::Projection::VariantTag {
            carrier,
            discriminator: key,
            ty,
        };

        Ok(Some(projection))
    }

    /// Return the precise variant selected by one scalar discriminant type.
    pub(in crate::check) fn variant_for_discriminant(
        &mut self,
        module: ModuleId,
        carrier: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let dir::Type::Literal(discriminant) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(variants) = self.variant_types(module, carrier)? else {
            return Ok(None);
        };

        // select the one declaration guaranteed unique by the variant family
        let mut selected = None;
        for variant in variants {
            let dir::Type::Variant(identity) = self.ty(variant)? else {
                return Err(CompilerError::Internal {
                    message: "variant family produced a non-variant type".to_string(),
                });
            };
            if self.variant_discriminant(&identity)? != discriminant {
                continue;
            }
            if selected.replace(variant).is_some() {
                return Err(CompilerError::Internal {
                    message: "variant family has duplicate discriminants".to_string(),
                });
            }
        }

        Ok(selected)
    }

    /// Return the declared member domain of one value enum type.
    fn enum_discriminant_domain(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        // the value names an enum declaration instance
        let dir::Type::Application(instance) = self.ty(value)? else {
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

        // case-specific types expose only their selected discriminant
        if let dir::Type::Variant(variant) = self.ty(value)? {
            let discriminant = self.variant_discriminant(&variant)?;

            return Ok(Answer::Ready(Some(vec![discriminant])));
        }

        // enums discriminate over their declared members
        if let Some(domain) = self.enum_discriminant_domain(value)? {
            return Ok(Answer::Ready(Some(domain)));
        }

        self.tagged_discriminant_domain(origin, value)
    }

    /// Return the discriminant carried by one case-specific type.
    fn variant_discriminant(
        &mut self,
        variant: &dir::VariantType,
    ) -> CompilerResult<dir::ScalarLiteral> {
        let owner = self.variant_owner(variant)?;

        // read the declared discriminant from the owning variant family
        let discriminant = match self.definition(owner.symbol)? {
            Some(dir::Definition::Enum(definition)) => definition
                .variants()
                .find(|member| member.symbol == variant.variant)
                .map(|member| dir::ScalarLiteral::from(member.value)),
            Some(dir::Definition::Newtype(definition)) if definition.is_tagged() => definition
                .tagged_variant_by_symbol(variant.variant)
                .map(|member| dir::ScalarLiteral::String(member.discriminant)),
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
    pub(in crate::check) fn enum_case_key_from_discriminant(
        &mut self,
        mut value: dir::GlobalTypeId,
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        if let dir::Type::Variant(variant) = self.ty(value)? {
            value = variant.owner;
        }

        // the value names an enum declaration instance
        let dir::Type::Application(instance) = self.ty(value)? else {
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
