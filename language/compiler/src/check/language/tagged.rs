use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Answer, CheckState, Origin, answer};
use crate::{CompilerError, CompilerResult};

/// The default discriminant field used by derived tagged unions.
const TAGGED_DISCRIMINANT_FIELD: &str = "kind";

impl CheckState<'_> {
    /// Return whether one declaration derives `Tagged`.
    pub(in crate::check) fn symbol_has_tagged_derive(&self, symbol: dir::GlobalSymbolId) -> bool {
        let bindings = self.binding_table(symbol.module_id);
        let binding = bindings.get_symbol(symbol.local_id);
        let Some(declaration) = binding.declaration else {
            return false;
        };

        // inspect attached derive invocations
        let invocations = self.decorator_invocations(symbol.module_id, declaration.local_id);
        invocations.iter().any(|invocation| {
            self.decorator_language_item(symbol.module_id, invocation)
                == Some(dir::LanguageItem::Derive)
                && invocation.arguments.iter().any(|argument| {
                    self.decorator_argument_language_item(symbol.module_id, *argument)
                        == Some(dir::LanguageItem::Tagged)
                })
        })
    }

    /// Insert one generated tagged variant member symbol.
    pub(in crate::check) fn insert_tagged_variant_symbol(
        &mut self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let module = owner.module_id;
        let (scope, scope_value) = {
            let bindings = self.binding_table(module);
            let Some(scope) = bindings.scope_for_owner(owner.local_id) else {
                return Err(CompilerError::Internal {
                    message: format!("tagged owner {owner:?} has no member scope"),
                });
            };
            let scope_value = bindings.get_scope(scope).clone();

            (scope, scope_value)
        };

        // make the owner scope writable in the checked binding tail
        let bindings = &mut self.module_mut(module).bindings_tail;
        bindings.make_scope_mutable(scope.id, &scope_value);

        let (symbol, _) = bindings.insert_symbol(
            dir::SymbolRole::Item,
            dir::SymbolKind::EnumField,
            Some(key),
            scope,
            None,
            dir::SymbolVisibility::Member,
        );

        Ok(symbol.into_global(module))
    }

    /// Return one generated tagged variant case.
    pub(in crate::check) fn tagged_variant_case(
        &self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> Option<dir::VariantCase> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(owner) else {
            return None;
        };
        let variant = definition.members.iter().find_map(|member| match member {
            dir::DefinitionMember::Variant(variant) if variant.key.matches(&key) => Some(variant),
            _ => None,
        })?;

        Some(dir::VariantCase {
            owner,
            key: variant.key,
            member: variant.symbol,
        })
    }

    /// Return the default tagged discriminant key.
    pub(in crate::check) fn tagged_discriminant_key(&mut self, module: ModuleId) -> dir::StaticKey {
        let name = self
            .module_mut(module)
            .strings
            .intern(TAGGED_DISCRIMINANT_FIELD);

        dir::StaticKey::Name(name)
    }

    /// Return the default source case key for one tagged discriminant.
    pub(in crate::check) fn tagged_case_key_from_discriminant(
        &mut self,
        module: ModuleId,
        discriminant: dir::ScalarLiteral,
    ) -> Option<dir::StaticKey> {
        let dir::ScalarLiteral::String(name) = discriminant else {
            return None;
        };
        let text = self.module(module).strings.get(name).to_string();
        let key = dir::StringMapping::Capitalize.apply(&text);
        let key = self.module_mut(module).strings.intern(&key);

        Some(dir::StaticKey::Name(key))
    }

    /// Return the finite discriminant domain of one tagged newtype value.
    pub(in crate::check) fn tagged_discriminant_domain(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::ScalarLiteral>>>> {
        let value = answer!(self.reduce_type_head(origin, value)?);
        let Some(instance) = self.tagged_domain_instance(value)? else {
            return Ok(Answer::Ready(None));
        };
        if !self.symbol_has_tagged_derive(instance.symbol) {
            return Ok(Answer::Ready(None));
        }

        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol) else {
            return Ok(Answer::Ready(None));
        };

        // substitute owner arguments through the backing type
        let substitution = self.instance_substitution(value.module_id, &instance)?;
        let backing = self.substitute_type(origin.module(), definition.value, &substitution)?;
        let backing = answer!(self.reduce_type_head(origin, backing)?);

        // collect every arm discriminant
        let mut arms = Vec::new();
        match self.ty(backing)? {
            dir::Type::Union(union) => arms.extend(
                self.type_ids(backing.module_id, union.elements)?
                    .iter()
                    .copied(),
            ),
            _ => arms.push(backing),
        }
        let mut domain = Vec::with_capacity(arms.len());
        for arm in arms {
            let arm = answer!(self.reduce_type_head(origin, arm)?);
            let Some(discriminant) = answer!(self.tagged_arm_discriminant(origin, arm)?) else {
                return Ok(Answer::Ready(None));
            };

            domain.push(discriminant);
        }

        Ok(Answer::Ready(Some(domain)))
    }

    /// Return the instance represented by one tagged-domain owner type.
    fn tagged_domain_instance(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GenericInstance>> {
        let instance = match self.ty(value)? {
            dir::Type::Instance(instance) => instance,
            dir::Type::Reference(reference) => {
                if let Some(template) = self.symbol_template(reference.symbol)
                    && !self.generic_template_parameters(template).is_empty()
                {
                    return Ok(None);
                }

                dir::GenericInstance {
                    symbol: reference.symbol,
                    arguments: dir::TypeListId::EMPTY,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }

    /// Return the tagged discriminant literal for one backing arm.
    pub(in crate::check) fn tagged_arm_discriminant(
        &mut self,
        origin: Origin,
        arm: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::ScalarLiteral>>> {
        let tag_key = self.tagged_discriminant_key(origin.module());
        match self.ty(arm)? {
            dir::Type::Shape(shape) => {
                let Some(tag) = self
                    .shape_fields(arm.module_id, shape.fields)?
                    .iter()
                    .find(|field| field.key == tag_key)
                    .copied()
                else {
                    return Ok(Answer::Ready(None));
                };
                let tag = answer!(self.reduce_type_head(origin, tag.ty)?);
                let dir::Type::Literal(discriminant) = self.ty(tag)? else {
                    return Ok(Answer::Ready(None));
                };

                Ok(Answer::Ready(Some(discriminant)))
            }
            dir::Type::Instance(_) => self.tagged_instance_discriminant(origin, arm, tag_key),
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the tagged discriminant literal for one ready backing arm.
    pub(in crate::check) fn require_tagged_arm_discriminant(
        &mut self,
        origin: Origin,
        arm: dir::GlobalTypeId,
        operation: &'static str,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        match self.tagged_arm_discriminant(origin, arm)? {
            Answer::Ready(value) => Ok(value),
            Answer::Pending(blockers) => Err(CompilerError::Internal {
                message: format!("{operation} requires a ready tagged arm: {blockers:?}"),
            }),
        }
    }

    /// Return the tagged discriminant literal for one nominal backing arm.
    pub(in crate::check) fn tagged_instance_discriminant(
        &mut self,
        origin: Origin,
        arm: dir::GlobalTypeId,
        tag_key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<dir::ScalarLiteral>>> {
        let lookup = answer!(self.lookup_member(
            origin,
            origin.module(),
            arm,
            dir::MemberSpace::Instance,
            tag_key,
        )?);
        let ty = lookup.value_type();
        let Some(ty) = ty else {
            return Ok(Answer::Ready(None));
        };
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        match self.ty(ty)? {
            dir::Type::Literal(literal) => Ok(Answer::Ready(Some(literal))),
            _ => Ok(Answer::Ready(None)),
        }
    }
}
