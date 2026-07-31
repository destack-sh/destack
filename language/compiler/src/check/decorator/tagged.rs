use std::mem::take;

use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Answer, CheckState, DecoratorApplication, Origin};
use crate::{CompilerError, CompilerResult};

/// One Tagged provider configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TaggedOptions {
    /// The explicitly selected discriminator property.
    discriminator: Option<dir::StaticKey>,
    /// The constructor naming policy.
    case: TaggedCaseConvention,
    /// Explicit constructor names keyed by discriminant text.
    names: FxIndexMap<dir::StringId, dir::StringId>,
}

/// One constructible arm in a Tagged newtype backing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TaggedArm {
    /// The checked backing leaf.
    backing: dir::GlobalTypeId,
    /// The declared fields exposed by the leaf.
    declared_fields: Vec<dir::TypeProperty>,
    /// The fields accepted when constructing the leaf.
    constructor_fields: Vec<dir::TypeProperty>,
}

/// Constructor naming policy for one Tagged derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaggedCaseConvention {
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
    /// The constructor argument after removing the discriminator.
    argument: Option<dir::GlobalTypeId>,
}

impl CheckState<'_> {
    /// Apply every walked derive decorator onto its module declarations.
    ///
    /// Derived members belong to the declared module, so this pass runs at the end of the walk
    /// in declare and check alike and reads only written module-local syntax.
    /// Checking later validates the derive expressions against their provider declarations.
    pub(in crate::check) fn apply_derive_decorators(&mut self) -> CompilerResult<()> {
        // select every walked derive application
        let mut applications = Vec::new();
        for application in &self.decorators {
            if self.global.language.item(application.symbol) == Some(dir::LanguageItem::Derive) {
                applications.push(application.clone());
            }
        }

        // apply each application onto its annotated declaration
        for application in &applications {
            self.apply_derive_application(application)?;
        }

        Ok(())
    }

    /// Apply one walked derive application onto its annotated declaration.
    fn apply_derive_application(
        &mut self,
        application: &DecoratorApplication,
    ) -> CompilerResult<()> {
        let module = application.owner.module_id;
        for argument in application.expression.arguments.iter().copied() {
            let Some(expression) = self.module_view(module).get(argument).value() else {
                continue;
            };

            // split one provider reference from its written options
            let (target, options) = match self.module_view(module).get(expression) {
                dir::Expression::Call {
                    left, arguments, ..
                } => (*left, arguments.first().copied()),
                _ => (expression, None),
            };
            let options =
                options.and_then(|argument| self.module_view(module).get(argument).value());

            // providers resolve by language item, without foreign module loads
            let Some(symbol) = self.reference_symbol(target.into_global_any(module)) else {
                continue;
            };
            let symbol = self.resolve_symbol_alias(symbol)?;
            if self.global.language.item(symbol) != Some(dir::LanguageItem::Tagged) {
                continue;
            }

            // derived members require written literal options
            let origin = self.node_site(expression.into_global_any(module))?.origin();
            let Some(options) = self.decode_tagged_options(module, options) else {
                self.report_undecidable_static_value(module, expression.into_any());
                continue;
            };
            self.apply_tagged_derive(origin, application.owner, symbol, options)?;
        }

        Ok(())
    }

    /// Decode literal Tagged options from one written provider argument.
    fn decode_tagged_options(
        &self,
        module: ModuleId,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Option<TaggedOptions> {
        let mut options = TaggedOptions {
            discriminator: None,
            case: TaggedCaseConvention::UpperCamel,
            names: FxIndexMap::default(),
        };
        let Some(expression) = expression else {
            return Some(options);
        };
        let dir::Expression::ObjectExpression { properties } =
            self.module_view(module).get(expression)
        else {
            return None;
        };

        for property in properties {
            let (key, value) = self.literal_property(module, *property)?;
            match self.strings().get(key) {
                "discriminator" => {
                    options.discriminator = match self.literal_scalar(module, value)? {
                        dir::ScalarLiteral::Undefined => None,
                        dir::ScalarLiteral::String(name) => Some(dir::StaticKey::Name(name)),
                        _ => return None,
                    };
                }
                "case" => {
                    options.case = match self.literal_scalar(module, value)? {
                        dir::ScalarLiteral::Undefined => TaggedCaseConvention::UpperCamel,
                        dir::ScalarLiteral::String(name) => {
                            TaggedCaseConvention::from_text(self.strings().get(name))?
                        }
                        _ => return None,
                    };
                }
                "names" => {
                    let dir::Expression::ObjectExpression { properties } =
                        self.module_view(module).get(value)
                    else {
                        return None;
                    };
                    for property in properties {
                        let (key, value) = self.literal_property(module, *property)?;
                        let dir::ScalarLiteral::String(name) =
                            self.literal_scalar(module, value)?
                        else {
                            return None;
                        };
                        options.names.insert(key, name);
                    }
                }
                _ => return None,
            }
        }

        Some(options)
    }

    /// Return one written literal object property as a name and value.
    fn literal_property(
        &self,
        module: ModuleId,
        property: dir::LocalNodeId<dir::Property>,
    ) -> Option<(dir::StringId, dir::LocalNodeId<dir::Expression>)> {
        let dir::Property::Field { key, value, .. } = self.module_view(module).get(property) else {
            return None;
        };
        let Some(dir::StaticKey::Name(key)) = key.direct_static_key() else {
            return None;
        };

        Some((key, *value))
    }

    /// Return one written scalar literal expression value.
    fn literal_scalar(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::ScalarLiteral> {
        match self.module_view(module).get(expression) {
            dir::Expression::ScalarLiteral(value) => Some(*value),
            _ => None,
        }
    }

    /// Apply one Tagged provider to its annotated declaration.
    fn apply_tagged_derive(
        &mut self,
        origin: Origin,
        owner: dir::GlobalNodeIdAny,
        provider: dir::GlobalSymbolId,
        options: TaggedOptions,
    ) -> CompilerResult<()> {
        let module = owner.module_id;
        let symbol = self.module(module).declaration_symbol(owner.local_id);
        let Some(symbol) = symbol else {
            self.report_invalid_derive_target(origin, provider)?;

            return Ok(());
        };
        let (backing, template, is_tagged) = match self.definintion_maybe(symbol) {
            Some(dir::Definition::Newtype(definition)) => (
                definition.backing,
                definition
                    .template
                    .map(|template| template.into_global(module)),
                definition.is_tagged(),
            ),
            _ => {
                self.report_invalid_derive_target(origin, provider)?;

                return Ok(());
            }
        };
        if is_tagged {
            self.report_duplicate_derive_provider(origin, provider)?;

            return Ok(());
        }

        // collect every constructible backing arm before selecting a discriminator
        let mut active = FxIndexSet::default();
        let Some(arms) = self.tagged_arms(origin, backing, &mut active)? else {
            self.report_invalid_tagged_variant(origin)?;

            return Ok(());
        };
        if arms.is_empty() {
            return Err(CompilerError::Internal {
                message: "Tagged backing produced no constructible arms".to_string(),
            });
        }

        // select one explicit or uniquely inferred discriminator
        let Some((discriminator, discriminants)) =
            self.select_tagged_discriminator(origin, options.discriminator, &arms)?
        else {
            return Ok(());
        };
        let mut variants = Vec::with_capacity(arms.len());
        for (arm, discriminant) in arms.into_iter().zip(discriminants) {
            variants.push(self.derive_tagged_variant(
                origin.module(),
                arm,
                discriminator,
                discriminant,
            )?);
        }

        // validate every generated key before mutating bindings
        let mut cases = Vec::with_capacity(variants.len());
        let mut distinct_keys = FxIndexSet::default();
        for variant in variants {
            let Some(name) = options.case_name(variant.discriminant, self.strings()) else {
                self.report_invalid_tagged_case(origin, variant.discriminant)?;

                return Ok(());
            };
            if !distinct_keys.insert(name) {
                self.report_duplicate_tagged_case(origin, name)?;

                return Ok(());
            }
            cases.push((dir::StaticKey::Name(name), variant));
        }

        // create one static variant symbol for each validated backing arm
        let instance = self.declaration_instance(symbol)?;
        let owner_ty = self.intern_type(dir::Type::Application(instance))?;
        let mut members = Vec::with_capacity(cases.len());
        for (key, variant) in cases {
            let member = self.insert_tagged_variant_symbol(symbol, key)?;
            let ty =
                self.tagged_variant_member_type(owner_ty, member, template, variant.argument)?;
            self.bind_symbol_type(member, ty)?;
            members.push(dir::DefinitionMember::TaggedVariant(
                dir::TaggedVariantDefinition {
                    symbol: member,
                    source: owner,
                    key,
                    discriminant: variant.discriminant,
                    backing: variant.backing,
                    argument: variant.argument,
                },
            ));
        }

        // commit the derived members as the discriminator domain
        let state = self.module_mut(module);
        let Some(dir::Definition::Newtype(definition)) = state.definitions.definition_mut(symbol)
        else {
            return Err(CompilerError::Internal {
                message: format!("Tagged owner {symbol:?} lost its newtype definition"),
            });
        };
        definition.discriminator = Some(discriminator);
        definition.members.extend(members);

        Ok(())
    }

    /// Return the constructible arms beneath one Tagged backing type.
    fn tagged_arms(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<Vec<TaggedArm>>> {
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
                let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
                let mut arms = Vec::with_capacity(elements.len());
                for element in elements {
                    let Some(element_arms) = self.tagged_arms(origin, element, active)? else {
                        return Ok(None);
                    };
                    arms.extend(element_arms);
                }

                Ok(Some(arms))
            }

            // return one structural arm with its checked fields
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                let fields = self
                    .shape_properties(ty.module_id, shape.properties)?
                    .to_vec();
                let arm = TaggedArm {
                    backing: ty,
                    declared_fields: fields.clone(),
                    constructor_fields: fields,
                };

                Ok(Some(vec![arm]))
            }

            // return structs or flatten a nested newtype; derive
            //  foreign arms only when checking
            dir::Type::Application(instance) => {
                if self.is_declaration() && !self.is_own_module(instance.symbol.module_id) {
                    return Ok(None);
                }

                match self.definition(instance.symbol)? {
                    // return one struct arm with its instantiated fields
                    Some(dir::Definition::Struct(_)) => {
                        let declared_fields = match self.struct_fields(origin, ty)? {
                            Answer::Ready(fields) => fields,
                            Answer::Pending(blockers) => {
                                return Err(CompilerError::Internal {
                                    message: format!(
                                        "Tagged struct {ty:?} remained blocked after decorator \
                                         checking: {blockers:?}"
                                    ),
                                });
                            }
                        };
                        let constructor_fields = match self.struct_constructor_fields(origin, ty)? {
                            Answer::Ready(fields) => fields,
                            Answer::Pending(blockers) => {
                                return Err(CompilerError::Internal {
                                    message: format!(
                                        "Tagged struct constructor {ty:?} remained blocked after \
                                         decorator checking: {blockers:?}"
                                    ),
                                });
                            }
                        };
                        let arm = TaggedArm {
                            backing: ty,
                            declared_fields: declared_fields.into_vec(),
                            constructor_fields: constructor_fields.into_vec(),
                        };

                        Ok(Some(vec![arm]))
                    }

                    // flatten a nested newtype into its instantiated backing arms
                    Some(dir::Definition::Newtype(definition)) => {
                        let backing = definition.backing;
                        if !active.insert(instance.symbol) {
                            return Ok(None);
                        }

                        let substitution = self
                            .instance_substitution(ty.module_id, &instance)?
                            .with_receiver(ty);
                        let backing = self.substitute_type(backing, &substitution)?;
                        let arms = self.tagged_arms(origin, backing, active)?;
                        active.swap_remove(&instance.symbol);

                        Ok(arms)
                    }

                    // reject nominal leaves without field construction
                    _ => Ok(None),
                }
            }

            // reject leaves without structural construction
            _ => Ok(None),
        }
    }

    /// Select one explicit or uniquely inferred Tagged discriminator.
    fn select_tagged_discriminator(
        &mut self,
        origin: Origin,
        explicit: Option<dir::StaticKey>,
        arms: &[TaggedArm],
    ) -> CompilerResult<Option<(dir::StaticKey, Vec<dir::StringId>)>> {
        if let Some(discriminator) = explicit {
            let Some(discriminants) =
                self.collect_tagged_discriminants(origin, discriminator, arms)?
            else {
                self.report_invalid_tagged_discriminator(origin, discriminator)?;

                return Ok(None);
            };
            if let Some(duplicate) = Self::duplicate_tagged_discriminant(&discriminants) {
                self.report_duplicate_tagged_discriminant(origin, duplicate)?;

                return Ok(None);
            }

            return Ok(Some((discriminator, discriminants)));
        }

        // infer candidates from fields present on the first arm
        let mut candidates = Vec::new();
        let mut duplicates = Vec::new();
        for field in &arms[0].declared_fields {
            let Some(discriminants) = self.collect_tagged_discriminants(origin, field.key, arms)?
            else {
                continue;
            };
            if let Some(duplicate) = Self::duplicate_tagged_discriminant(&discriminants) {
                duplicates.push(duplicate);
            } else {
                candidates.push((field.key, discriminants));
            }
        }

        match candidates.as_slice() {
            [(discriminator, discriminants)] => Ok(Some((*discriminator, discriminants.clone()))),
            [] if duplicates.len() == 1 => {
                self.report_duplicate_tagged_discriminant(origin, duplicates[0])?;

                Ok(None)
            }
            [] => {
                self.report_missing_tagged_discriminator(origin)?;

                Ok(None)
            }
            _ => {
                let discriminators = candidates.iter().map(|(key, _)| *key).collect::<Vec<_>>();
                self.report_ambiguous_tagged_discriminator(origin, &discriminators)?;

                Ok(None)
            }
        }
    }

    /// Collect one string discriminant from every Tagged arm.
    fn collect_tagged_discriminants(
        &mut self,
        origin: Origin,
        discriminator: dir::StaticKey,
        arms: &[TaggedArm],
    ) -> CompilerResult<Option<Vec<dir::StringId>>> {
        let mut discriminants = Vec::with_capacity(arms.len());
        for arm in arms {
            let Some(field) = arm
                .declared_fields
                .iter()
                .find(|field| field.key == discriminator && !field.is_optional)
            else {
                return Ok(None);
            };
            let field_type = match self.reduce_type_head(origin, field.access.store())? {
                Answer::Ready(ty) => ty,
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "Tagged discriminator field {:?} remained blocked after decorator checking: {blockers:?}",
                            field.access.store(),
                        ),
                    });
                }
            };
            let dir::Type::Literal(dir::ScalarLiteral::String(discriminant)) =
                self.ty(field_type)?
            else {
                return Ok(None);
            };
            discriminants.push(discriminant);
        }

        Ok(Some(discriminants))
    }

    /// Return the first repeated Tagged discriminant.
    fn duplicate_tagged_discriminant(discriminants: &[dir::StringId]) -> Option<dir::StringId> {
        let mut distinct = FxIndexSet::default();

        discriminants
            .iter()
            .copied()
            .find(|discriminant| !distinct.insert(*discriminant))
    }

    /// Derive one Tagged variant from a selected arm.
    fn derive_tagged_variant(
        &mut self,
        module: ModuleId,
        arm: TaggedArm,
        discriminator: dir::StaticKey,
        discriminant: dir::StringId,
    ) -> CompilerResult<TaggedVariant> {
        // retain every constructor field except the discriminator
        let argument_fields: Vec<_> = arm
            .constructor_fields
            .iter()
            .filter(|field| field.key != discriminator)
            .copied()
            .collect();
        let argument = if argument_fields.is_empty() {
            None
        } else {
            let fields = self.intern_properties(module, &argument_fields)?;
            let shape = dir::ShapeType {
                properties: fields,
                call_signatures: dir::TypeListId::EMPTY,
                construct_signatures: dir::TypeListId::EMPTY,
                index_signatures: dir::TypeListId::EMPTY,
            };

            Some(self.intern_type(dir::Type::from(shape))?)
        };

        Ok(TaggedVariant {
            backing: arm.backing,
            discriminant,
            argument,
        })
    }

    /// Build one generated tagged variant member type.
    fn tagged_variant_member_type(
        &mut self,
        owner: dir::GlobalTypeId,
        member: dir::GlobalSymbolId,
        template: Option<dir::GlobalGenericTemplateId>,
        argument: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let variant = self.intern_type(dir::Type::Variant(dir::VariantType {
            owner,
            variant: member,
        }))?;
        let Some(argument) = argument else {
            return Ok(variant);
        };
        let (dir::Type::Shape(shape) | dir::Type::Object(shape)) = self.ty(argument)? else {
            return Err(CompilerError::Internal {
                message: format!("tagged constructor argument {argument:?} is not a shape"),
            });
        };
        let is_optional = self
            .shape_properties(argument.module_id, shape.properties)?
            .iter()
            .all(|field| field.is_optional);

        let parameter = dir::FunctionParameterType {
            ty: argument,
            is_optional,
            is_rest: false,
        };
        let parameters = self.intern_parameters(&[parameter])?;
        let signature = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter: None,
            parameters,
            return_type: Some(variant),
            is_generator: false,
        };

        self.intern_signature(signature)
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
}

impl TaggedOptions {
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
}

impl TaggedCaseConvention {
    /// Decode one written case convention name.
    fn from_text(text: &str) -> Option<Self> {
        let case = match text {
            "preserve" => Self::Preserve,
            "camelCase" => Self::Camel,
            "UpperCamelCase" => Self::UpperCamel,
            "snake_case" => Self::Snake,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            _ => return None,
        };

        Some(case)
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
