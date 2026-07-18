use std::mem::take;

use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Answer, CheckState, DecoratorApplication, DecoratorObject, Origin};
use crate::{CompilerError, CompilerResult};

/// One Tagged provider configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TaggedOptions {
    /// The property carrying each variant's string discriminant.
    discriminant: dir::StringId,
    /// The constructor naming policy.
    case: TaggedCase,
    /// Explicit constructor names keyed by discriminant text.
    names: FxIndexMap<dir::StringId, dir::StringId>,
}

/// Constructor naming policy for one Tagged derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaggedCase {
    /// Preserve the discriminant text.
    Preserve,
    /// Convert the discriminant to lower camel case.
    Camel,
    /// Convert the discriminant to upper camel case.
    UpperCamel,
    /// Convert the discriminant to snake case.
    Snake,
    /// Convert the discriminant to screaming snake case.
    ScreamingSnake,
}

/// One declared leaf in a Tagged newtype domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaggedVariant {
    /// The declared backing leaf.
    backing: dir::GlobalTypeId,
    /// The explicit string discriminator.
    discriminant: dir::StringId,
}

impl CheckState<'_> {
    /// Apply one compiler-owned derive decorator from its provider values.
    pub(in crate::check) fn apply_derive_decorator(
        &mut self,
        module: ModuleId,
        application: &DecoratorApplication,
        resolution: &dir::DecoratorResolution,
        value: &dir::StaticTerm,
    ) -> CompilerResult<()> {
        let dir::DecoratorSelection::Derive { providers } = &resolution.selection else {
            return Err(CompilerError::Internal {
                message: format!(
                    "derive decorator {:?} has a non-derive selection",
                    application.expression.decorator
                ),
            });
        };
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "derive decorator {:?} has a non-newtype value",
                    application.expression.decorator
                ),
            });
        };
        let Some(values) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "derive decorator {:?} has a non-tuple value",
                    application.expression.decorator
                ),
            });
        };
        if providers.len() != values.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "derive decorator {:?} selected {} providers but evaluated {} values",
                    application.expression.decorator,
                    providers.len(),
                    values.len()
                ),
            });
        }

        // apply providers in authored order
        for (provider, value) in providers.iter().zip(values) {
            let argument = provider.argument.local_id;
            let expression = self
                .module_view(module)
                .get(argument)
                .value()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("derive provider argument {argument:?} has no expression"),
                })?;
            let origin = self.node_site(expression.into_global_any(module))?.origin();
            match self.global.language.item(provider.newtype.symbol) {
                Some(dir::LanguageItem::Tagged) => self.apply_tagged_derive(
                    origin,
                    application.owner,
                    provider.newtype.symbol,
                    value,
                )?,
                item => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "derive provider {:?} has unsupported language item {item:?}",
                            provider.newtype.symbol
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Apply one Tagged provider to its annotated declaration.
    fn apply_tagged_derive(
        &mut self,
        origin: Origin,
        owner: dir::GlobalNodeIdAny,
        provider: dir::GlobalSymbolId,
        value: &dir::StaticTerm,
    ) -> CompilerResult<()> {
        let module = owner.module_id;
        let symbol = self.module(module).declaration_symbol(owner.local_id);
        let Some(symbol) = symbol else {
            self.report_invalid_derive_target(origin, provider)?;

            return Ok(());
        };
        let definition = self.loaded_definition(symbol).cloned();
        let Some(dir::Definition::Newtype(definition)) = definition else {
            self.report_invalid_derive_target(origin, provider)?;

            return Ok(());
        };
        if definition.is_tagged() {
            self.report_duplicate_derive_provider(origin, provider)?;

            return Ok(());
        }

        // decode the provider value and declared discriminator domain
        let options = TaggedOptions::decode(value, self.strings())?;
        let discriminant = dir::StaticKey::Name(options.discriminant);
        let mut variants = Vec::new();
        let mut active = FxIndexSet::default();
        let is_tagged = self.collect_tagged_variants(
            origin,
            definition.backing,
            discriminant,
            &mut active,
            &mut variants,
        )?;
        if !is_tagged || variants.is_empty() {
            self.report_invalid_tagged_variant(origin, options.discriminant)?;

            return Ok(());
        }

        // validate every generated key before mutating bindings
        let mut cases = Vec::with_capacity(variants.len());
        let mut distinct_discriminants = FxIndexSet::default();
        let mut distinct_keys = FxIndexSet::default();
        for variant in variants {
            let Some(name) = options.case_name(variant.discriminant, self.strings()) else {
                self.report_invalid_tagged_case(origin, variant.discriminant)?;

                return Ok(());
            };
            if !distinct_discriminants.insert(variant.discriminant) || !distinct_keys.insert(name) {
                self.report_duplicate_tagged_case(origin, name)?;

                return Ok(());
            }
            cases.push((dir::StaticKey::Name(name), variant));
        }

        // create one static variant symbol for each validated backing arm
        let instance = self.declaration_instance(module, symbol)?;
        let owner_ty = self.intern_type(module, dir::Type::Instance(instance))?;
        let mut members = Vec::with_capacity(cases.len());
        for (key, variant) in cases {
            let member = self.insert_tagged_variant_symbol(symbol, key)?;
            let ty = dir::Type::EnumMember(dir::EnumMemberType {
                owner: owner_ty,
                member,
            });
            let ty = self.intern_type(module, ty)?;
            self.bind_symbol_type(member, ty)?;
            members.push(dir::DefinitionMember::TaggedVariant(
                dir::TaggedVariantDefinition {
                    symbol: member,
                    source: owner,
                    key,
                    discriminant: variant.discriminant,
                    backing: variant.backing,
                },
            ));
        }

        // commit the derived members as the discriminator domain
        let state = self.module_mut(module);
        let Some(dir::Definition::Newtype(definition)) = state.definitions.definition_mut(symbol)
        else {
            return Err(CompilerError::Internal {
                message: format!("Tagged owner {symbol:?} lost its working newtype definition"),
            });
        };
        definition.discriminant = Some(options.discriminant);
        definition.members.extend(members);

        Ok(())
    }

    /// Collect the declared variants beneath one tagged backing type.
    fn collect_tagged_variants(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        discriminant: dir::StaticKey,
        active: &mut FxIndexSet<dir::GlobalSymbolId>,
        variants: &mut Vec<TaggedVariant>,
    ) -> CompilerResult<bool> {
        let ty = self.settled_root(ty)?;
        let ty = match self.reduce_type_head(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "Tagged backing {ty:?} remained blocked after decorator checking: \
                         {blockers:?}"
                    ),
                });
            }
        };

        match self.ty(ty)? {
            // flatten direct union arms in declaration order
            dir::Type::Union(union) => {
                let arms = self.type_ids(ty.module_id, union.elements)?.to_vec();
                for arm in arms {
                    if !self.collect_tagged_variants(origin, arm, discriminant, active, variants)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }

            // accept one explicitly discriminated structural arm
            dir::Type::Shape(shape) => {
                let value = self.declared_shape_discriminant(ty.module_id, shape, discriminant)?;
                let Some(dir::ScalarLiteral::String(value)) = value else {
                    return Ok(false);
                };
                variants.push(TaggedVariant {
                    backing: ty,
                    discriminant: value,
                });

                Ok(true)
            }

            // accept a directly discriminated nominal arm or flatten a nested newtype
            dir::Type::Instance(instance) => {
                let value = self.declared_nominal_discriminant(instance.symbol, discriminant)?;
                if let Some(dir::ScalarLiteral::String(value)) = value {
                    variants.push(TaggedVariant {
                        backing: ty,
                        discriminant: value,
                    });

                    return Ok(true);
                }

                let definition = self.definition(instance.symbol)?.cloned();
                let Some(dir::Definition::Newtype(definition)) = definition else {
                    return Ok(false);
                };
                if !active.insert(instance.symbol) {
                    return Ok(false);
                }

                // instantiate the nested backing under its written arguments
                let substitution = self
                    .instance_substitution(ty.module_id, &instance)?
                    .with_receiver(ty);
                let backing =
                    self.substitute_type(origin.module(), definition.backing, &substitution)?;
                let is_tagged =
                    self.collect_tagged_variants(origin, backing, discriminant, active, variants)?;
                active.swap_remove(&instance.symbol);

                Ok(is_tagged)
            }

            // every tagged leaf must expose an explicit discriminator
            _ => Ok(false),
        }
    }

    /// Insert one generated tagged variant member symbol.
    fn insert_tagged_variant_symbol(
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

        // make the owner scope writable in the binding tail
        let bindings = &mut self.module_mut(module).bindings_tail;
        bindings.make_scope_mutable(scope.id, &scope_value);

        let (symbol, _) = bindings.insert_symbol(
            dir::SymbolRole::Item,
            dir::SymbolKind::Variant,
            Some(key),
            scope,
            None,
            dir::SymbolVisibility::Member,
        );

        Ok(symbol.into_global(module))
    }

    /// Return one declared structural discriminant.
    fn declared_shape_discriminant(
        &mut self,
        module: ModuleId,
        shape: dir::ShapeType,
        tag_key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        let field = self
            .shape_fields(module, shape.fields)?
            .iter()
            .find(|field| field.key == tag_key)
            .copied();
        let Some(field) = field else {
            return Ok(None);
        };

        self.declared_discriminant_literal(field.ty)
    }

    /// Return one declared nominal discriminant.
    fn declared_nominal_discriminant(
        &mut self,
        symbol: dir::GlobalSymbolId,
        tag_key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(None);
        };
        let field = definition.members().iter().find_map(|member| match member {
            dir::DefinitionMember::Field(field)
                if field.space == dir::MemberSpace::Instance && field.key == tag_key =>
            {
                Some(field.symbol)
            }
            _ => None,
        });
        let Some(field) = field else {
            return Ok(None);
        };
        let ty = self.require_symbol_type(field)?;

        self.declared_discriminant_literal(ty)
    }

    /// Return one literal discriminant type.
    fn declared_discriminant_literal(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        match self.ty(ty)? {
            dir::Type::Literal(literal) => Ok(Some(literal)),
            _ => Ok(None),
        }
    }
}

impl TaggedOptions {
    /// Decode one Tagged provider value.
    fn decode(value: &dir::StaticTerm, strings: &StringPool) -> CompilerResult<Self> {
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: "Tagged provider has a non-newtype value".to_string(),
            });
        };
        let mut options = Self {
            discriminant: strings.intern("kind"),
            case: TaggedCase::UpperCamel,
            names: FxIndexMap::default(),
        };

        // decode the selected provider backing
        if let Some(elements) = value.as_tuple()
            && elements.is_empty()
        {
            return Ok(options);
        }
        options.apply_properties(value, strings)?;

        Ok(options)
    }

    /// Return the constructor name selected for one discriminant.
    fn case_name(
        &self,
        discriminant: dir::StringId,
        strings: &StringPool,
    ) -> Option<dir::StringId> {
        let key = match self.names.get(&discriminant) {
            Some(key) => {
                if !dir::is_identifier(strings.get(*key)) {
                    return None;
                }

                *key
            }
            None => {
                let text = strings.get(discriminant);
                let text = self.case.apply(text)?;
                if !dir::is_identifier(&text) {
                    return None;
                }

                strings.intern(&text)
            }
        };

        Some(key)
    }

    /// Apply Tagged options in object evaluation order.
    fn apply_properties(
        &mut self,
        value: &dir::StaticTerm,
        strings: &StringPool,
    ) -> CompilerResult<()> {
        let object = DecoratorObject::try_from(value)?;
        for (key, value) in object.fields {
            self.apply_property(key, value, strings)?;
        }

        Ok(())
    }

    /// Apply one Tagged option.
    fn apply_property(
        &mut self,
        key: dir::StringId,
        value: &dir::StaticTerm,
        strings: &StringPool,
    ) -> CompilerResult<()> {
        match strings.get(key) {
            "discriminant" => {
                self.discriminant = value.as_string().ok_or_else(|| CompilerError::Internal {
                    message: "Tagged discriminant has a non-string value".to_string(),
                })?;
            }
            "case" => {
                self.case = if value.as_scalar() == Some(dir::ScalarLiteral::Undefined) {
                    TaggedCase::UpperCamel
                } else {
                    TaggedCase::decode(value, strings)?
                };
            }
            "names" => {
                self.names.clear();
                if value.as_scalar() != Some(dir::ScalarLiteral::Undefined) {
                    self.apply_names(value)?;
                }
            }
            name => {
                return Err(CompilerError::Internal {
                    message: format!("Tagged provider has unknown field '{name}'"),
                });
            }
        }

        Ok(())
    }

    /// Apply explicit Tagged names from one object value.
    fn apply_names(&mut self, value: &dir::StaticTerm) -> CompilerResult<()> {
        let object = DecoratorObject::try_from(value)?;
        for (key, value) in object.fields {
            let value = value.as_string().ok_or_else(|| CompilerError::Internal {
                message: "Tagged name has a non-string value".to_string(),
            })?;
            self.names.insert(key, value);
        }

        Ok(())
    }
}

impl TaggedCase {
    /// Decode one Tagged constructor naming policy.
    fn decode(value: &dir::StaticTerm, strings: &StringPool) -> CompilerResult<Self> {
        let value = value.as_string().ok_or_else(|| CompilerError::Internal {
            message: "Tagged case has a non-string value".to_string(),
        })?;
        let case = match strings.get(value) {
            "preserve" => Self::Preserve,
            "camelCase" => Self::Camel,
            "UpperCamelCase" => Self::UpperCamel,
            "snake_case" => Self::Snake,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            value => {
                return Err(CompilerError::Internal {
                    message: format!("Tagged provider has invalid case '{value}'"),
                });
            }
        };

        Ok(case)
    }

    /// Apply this naming policy to one discriminant.
    fn apply(self, text: &str) -> Option<String> {
        if self == Self::Preserve {
            return Some(text.to_string());
        }
        let words = Self::words(text);
        if words.is_empty() {
            return None;
        }

        let value = match self {
            Self::Preserve => text.to_string(),
            Self::Camel => {
                let mut words = words.into_iter();
                let first = words.next()?;
                let mut result = first.to_lowercase();
                for word in words {
                    result.push_str(&Self::upper_camel_word(&word)?);
                }

                result
            }
            Self::UpperCamel => words
                .into_iter()
                .map(|word| Self::upper_camel_word(&word))
                .collect::<Option<String>>()?,
            Self::Snake => words
                .into_iter()
                .map(|word| word.to_lowercase())
                .collect::<Vec<_>>()
                .join("_"),
            Self::ScreamingSnake => words
                .into_iter()
                .map(|word| word.to_uppercase())
                .collect::<Vec<_>>()
                .join("_"),
        };

        Some(value)
    }

    /// Split one identifier-like discriminant into casing words.
    fn words(text: &str) -> Vec<String> {
        let characters = text.chars().collect::<Vec<_>>();
        let mut words = Vec::new();
        let mut word = String::new();

        for (index, character) in characters.iter().copied().enumerate() {
            if !character.is_alphanumeric() {
                if !word.is_empty() {
                    words.push(take(&mut word));
                }

                continue;
            }

            // split ordinary camel boundaries and the final capital of an acronym
            let previous = index
                .checked_sub(1)
                .and_then(|previous| characters.get(previous))
                .copied();
            let next = characters.get(index + 1).copied();
            let starts_word = !word.is_empty()
                && character.is_uppercase()
                && (previous.is_some_and(|value| value.is_lowercase())
                    || next.is_some_and(|value| value.is_lowercase()));
            if starts_word {
                words.push(take(&mut word));
            }
            word.push(character);
        }
        if !word.is_empty() {
            words.push(word);
        }

        words
    }

    /// Convert one casing word to upper camel form.
    fn upper_camel_word(word: &str) -> Option<String> {
        let mut characters = word.chars();
        let first = characters.next()?;
        let mut result = first.to_uppercase().collect::<String>();
        result.push_str(&characters.as_str().to_lowercase());

        Some(result)
    }
}
