use std::iter::repeat_n;
use std::mem::take;

use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorApplication, Origin};
use crate::{CompilerError, CompilerResult};

/// One Tagged provider configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TaggedOptions {
    /// The explicitly selected discriminator property.
    discriminator: Option<dir::StaticKey>,
    /// The constructor naming policy.
    case: TaggedCaseConvention,
    /// The written naming convention text.
    case_text: Option<dir::StringId>,
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
    /// Apply every walked derive decorator onto its module declarations at the end of the walk.
    pub(in crate::check) fn apply_derive_decorators(&mut self) -> CompilerResult<()> {
        // select every walked derive application
        let mut applications = Vec::new();
        for application in &self.decorators {
            if self.environment_bound.language.item(application.symbol)
                == Some(dir::LanguageItem::Derive)
            {
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
            if self.environment_bound.language.item(symbol) != Some(dir::LanguageItem::Tagged) {
                continue;
            }

            // derived members require written literal options
            let origin = Origin::Node(expression.into_global_any(module), None);
            let Some(options) = self.decode_tagged_options(module, options) else {
                self.report_undecidable_static_value(module, expression.into_any());
                continue;
            };
            self.apply_tagged_derive(origin, application.owner, symbol, options)?;
        }

        Ok(())
    }

    /// Derive every tagged newtype definition the declared identities name.
    pub(in crate::check) fn derive_tagged_definitions(&mut self) -> CompilerResult<()> {
        // derive the rows the declare pass deferred on foreign content
        if !self.is_declaration() {
            return self.derive_deferred_tagged_definitions();
        }

        // collect the tagged newtypes the walk declared
        let module = self.module_id;
        let mut targets = Vec::new();
        for (symbol, definition) in self.module(module).iter_definitions() {
            if let dir::Definition::Newtype(definition) = definition
                && definition.is_tagged
            {
                targets.push(symbol);
            }
        }

        // record each derived definition onto the declared tail
        for symbol in targets {
            self.record_derived_tagged_definition(symbol)?;
        }

        Ok(())
    }

    /// Classify one tagged newtype and record its derived definition.
    fn record_derived_tagged_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(derived) = self.classify_tagged_newtype(symbol)? else {
            return Ok(());
        };

        // name each declared variant identity by its derived key
        for member in derived.tagged_variants() {
            let mut named = self
                .binding_table(symbol.module_id)
                .get_symbol(member.symbol.local_id)
                .clone();
            named.key = Some(member.key);
            self.module_mut(symbol.module_id)
                .bindings_tail
                .replace_symbol(member.symbol.local_id, named);
        }

        let source = self
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?
            .into_global(symbol.module_id);
        self.insert_definition(symbol, source, dir::Definition::Newtype(derived))
    }

    /// Derive the tagged rows the declare pass left unclassified.
    fn derive_deferred_tagged_definitions(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let mut deferred = Vec::new();
        {
            let state = self.module(module);
            let Some(declared) = &state.declared else {
                return Ok(());
            };
            for (symbol, definition) in declared.definitions.iter_definitions() {
                if let dir::Definition::Newtype(definition) = definition
                    && definition.is_tagged
                    && definition.tagged_variants().next().is_none()
                {
                    deferred.push(symbol);
                }
            }
        }
        for symbol in deferred {
            self.record_derived_tagged_definition(symbol)?;
        }

        Ok(())
    }

    /// Classify one tagged newtype's variants onto its declared identities.
    pub(in crate::check) fn classify_tagged_newtype(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::NewtypeDefinition>> {
        let module = symbol.module_id;
        let origin = Origin::Symbol(symbol);

        // read the declared newtype and its variant identities
        let Some(dir::Definition::Newtype(declared)) = self.definition_maybe(symbol).cloned()
        else {
            return Err(CompilerError::Internal {
                message: format!("tagged newtype {symbol:?} lost its declared definition"),
            });
        };
        let backing = declared.backing;
        let template = declared
            .template
            .map(|template| template.into_global(module));
        let identities = declared
            .members
            .iter()
            .filter_map(|member| match member {
                dir::DefinitionMember::TaggedKey(identity) => Some(identity.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if identities.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("tagged newtype {symbol:?} declared no variant identities"),
            });
        }

        // read the declared derive options
        let Some(written) = declared.tagged_options.clone() else {
            return Err(CompilerError::Internal {
                message: format!("tagged newtype {symbol:?} declared no Tagged options"),
            });
        };

        // resolve the written naming convention text
        let case = written
            .case
            .and_then(|text| TaggedCaseConvention::from_text(self.strings().get(text)))
            .unwrap_or(TaggedCaseConvention::UpperCamel);

        // assemble tagged options from the written derive input
        let options = TaggedOptions {
            discriminator: written.discriminator,
            case,
            case_text: written.case,
            names: written.names.iter().copied().collect(),
        };

        // classify every constructible backing arm
        let mut active = FxIndexSet::default();
        active.insert(symbol);
        let Some(arms) = self.tagged_arms(origin, backing, &mut active)? else {
            // defer foreign or invalid arms to the checking pass
            if !self.is_declaration() {
                self.report_invalid_tagged_variant(origin)?;
            }

            return Ok(None);
        };

        // select one explicit or uniquely inferred discriminator
        let Some((discriminator, discriminants)) =
            self.select_tagged_discriminator(origin, options.discriminator, &arms)?
        else {
            return Ok(None);
        };
        let mut variants = Vec::with_capacity(arms.len());
        for (arm, discriminant) in arms.into_iter().zip(discriminants) {
            variants.push(self.derive_tagged_variant(arm, discriminator, discriminant)?);
        }

        // validate every generated key before binding the identities
        let mut cases = Vec::with_capacity(variants.len());
        let mut distinct_keys = FxIndexSet::default();
        for variant in variants {
            let Some(name) = options.case_name(variant.discriminant, self.strings()) else {
                if !self.is_declaration() {
                    self.report_invalid_tagged_case(origin, variant.discriminant)?;
                }

                return Ok(None);
            };
            if !distinct_keys.insert(name) {
                if !self.is_declaration() {
                    self.report_duplicate_tagged_case(origin, name)?;
                }

                return Ok(None);
            }
            cases.push((dir::StaticKey::Name(name), variant));
        }

        // require the derived arms to line up with the declared identities
        if cases.len() != identities.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "tagged newtype {symbol:?} declared {} variant identities but derived {}",
                    identities.len(),
                    cases.len(),
                ),
            });
        }

        // bind each declared identity to its derived constructor
        let instance = self.declaration_instance(symbol)?;
        let owner_ty = self.intern_type(dir::Type::Application(instance))?;
        let mut derived = Vec::with_capacity(cases.len());
        for ((key, variant), identity) in cases.into_iter().zip(identities) {
            let member = identity.symbol;
            let ty =
                self.tagged_variant_member_type(owner_ty, member, template, variant.argument)?;
            self.bind_symbol_type(member, ty)?;
            derived.push(dir::DefinitionMember::TaggedVariant(
                dir::TaggedVariantDefinition {
                    symbol: member,
                    source: identity.source,
                    key,
                    discriminant: variant.discriminant,
                    backing: variant.backing,
                    argument: variant.argument,
                },
            ));
        }

        // replace the identities with the derived variants
        let mut definition = declared;
        definition.discriminator = Some(discriminator);
        definition
            .members
            .retain(|member| !matches!(member, dir::DefinitionMember::TaggedKey(_)));
        definition.members.extend(derived);

        Ok(Some(definition))
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
            case_text: None,
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
                            options.case_text = Some(name);

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
        let is_tagged = match self.definition_maybe(symbol) {
            Some(dir::Definition::Newtype(definition)) => definition.is_tagged(),
            _ => {
                self.report_invalid_derive_target(origin, provider)?;

                return Ok(());
            }
        };
        if is_tagged {
            self.report_duplicate_derive_provider(origin, provider)?;

            return Ok(());
        }

        // read the exact authored backing type expression
        let declaration = owner
            .local_id
            .try_into_typed::<dir::Declaration>()
            .map_err(|_| CompilerError::Internal {
                message: format!("Tagged owner {owner:?} is not a declaration"),
            })?;
        let dir::Declaration::Type(declaration) = self.module_view(module).get(declaration) else {
            return Err(CompilerError::Internal {
                message: format!("Tagged owner {owner:?} is not a type declaration"),
            });
        };
        let backing_source = declaration.value.into_global_any(module);

        // retain one authored source per derived variant
        let mut active = FxIndexSet::default();
        active.insert(symbol);
        let Some(sources) = self.tagged_variant_sources(backing_source, &mut active)? else {
            self.report_invalid_tagged_variant(origin)?;

            return Ok(());
        };

        // insert one positional symbol per derived variant
        let mut members = Vec::with_capacity(sources.len());
        for (index, source) in sources.into_iter().enumerate() {
            let member = self.insert_tagged_variant_symbol(symbol)?;
            members.push(dir::DefinitionMember::TaggedKey(dir::TaggedKeyDefinition {
                symbol: member,
                source,
                index: index as u32,
            }));
        }

        // mark the newtype tagged with its declared variant identities
        let state = self.module_mut(module);
        let Some(dir::Definition::Newtype(definition)) = state.definition_mut(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("Tagged owner {symbol:?} lost its newtype definition"),
            });
        };
        definition.is_tagged = true;
        definition.tagged_options = Some(dir::TaggedOptionsDefinition {
            discriminator: options.discriminator,
            case: options.case_text,
            names: options
                .names
                .iter()
                .map(|(key, name)| (*key, *name))
                .collect(),
        });
        definition.members.extend(members);

        Ok(())
    }

    /// Return the authored source node that introduced every derived variant.
    fn tagged_variant_sources(
        &mut self,
        source: dir::GlobalNodeIdAny,
        active: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<Vec<dir::GlobalNodeIdAny>>> {
        let module = source.module_id;
        let source_id = source
            .local_id
            .try_into_typed::<dir::TypeExpression>()
            .map_err(|_| CompilerError::Internal {
                message: format!("Tagged backing source {source:?} is not a type expression"),
            })?;
        let sources = match self.module_view(module).get(source_id) {
            dir::TypeExpression::Union { elements } => elements
                .iter()
                .map(|element| element.into_global_any(module))
                .collect(),
            _ => vec![source],
        };

        // repeat each authored source for the variants it expands into
        let mut expanded = Vec::new();
        for source in sources {
            let ty = self.require_node_type(source)?;
            let Some(count) = self.tagged_variant_count(ty, active)? else {
                return Ok(None);
            };
            expanded.extend(repeat_n(source, count));
        }

        Ok(Some(expanded))
    }

    /// Return the derived variant count of one checked backing type.
    fn tagged_variant_count(
        &mut self,
        ty: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<usize>> {
        match self.ty(ty)? {
            // count direct union variants in checked order
            dir::Type::Union(union) => {
                let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
                let mut count = 0;
                for element in elements {
                    let Some(element_count) = self.tagged_variant_count(element, active)? else {
                        return Ok(None);
                    };
                    count += element_count;
                }

                Ok(Some(count))
            }

            // count variants in an own nested newtype
            dir::Type::Application(instance) if self.is_own_module(instance.symbol.module_id) => {
                match self.definition_maybe(instance.symbol) {
                    Some(dir::Definition::Newtype(definition)) => {
                        let backing = definition.backing;
                        if !active.insert(instance.symbol) {
                            return Ok(None);
                        }
                        let count = self.tagged_variant_count(backing, active)?;
                        active.swap_remove(&instance.symbol);

                        Ok(count)
                    }
                    _ => Ok(Some(1)),
                }
            }

            // produce one variant for every other leaf
            _ => Ok(Some(1)),
        }
    }

    /// Return the constructible arms beneath one Tagged backing type.
    fn tagged_arms(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<Vec<TaggedArm>>> {
        let ty = self.shallow_resolve(ty)?;

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
            dir::Type::Object(shape) => {
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

            // return structs or flatten a nested newtype
            dir::Type::Application(instance) => {
                if !self.is_own_module(instance.symbol.module_id) {
                    self.import_external_module(instance.symbol.module_id)?;
                }
                match self.definition(instance.symbol)? {
                    // return one struct arm with its instantiated fields
                    Some(dir::Definition::Struct(_)) => {
                        let declared_fields = self.struct_fields(origin, ty)?;
                        let constructor_fields = self.struct_constructor_fields(origin, ty)?;
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
                if !self.is_declaration() {
                    self.report_invalid_tagged_discriminator(origin, discriminator)?;
                }

                return Ok(None);
            };
            if let Some(duplicate) = Self::duplicate_tagged_discriminant(&discriminants) {
                if !self.is_declaration() {
                    self.report_duplicate_tagged_discriminant(origin, duplicate)?;
                }

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
                if !self.is_declaration() {
                    self.report_duplicate_tagged_discriminant(origin, duplicates[0])?;
                }

                Ok(None)
            }
            [] => {
                if !self.is_declaration() {
                    self.report_missing_tagged_discriminator(origin)?;
                }

                Ok(None)
            }
            _ => {
                let discriminators = candidates.iter().map(|(key, _)| *key).collect::<Vec<_>>();
                if !self.is_declaration() {
                    self.report_ambiguous_tagged_discriminator(origin, &discriminators)?;
                }

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
            let field_type = self.normalize(origin, field.access.store())?;
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
            Some(self.intern_object(&argument_fields)?)
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
        let dir::Type::Object(shape) = self.ty(argument)? else {
            return Err(CompilerError::Internal {
                message: format!("tagged constructor argument {argument:?} is not an object type"),
            });
        };
        let is_optional = self
            .shape_properties(argument.module_id, shape.properties)?
            .iter()
            .all(|field| field.is_optional);

        let parameter = dir::FunctionParameterType {
            name: None,
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
            is_construct: false,
        };

        self.intern_signature(signature)
    }

    /// Insert one generated tagged variant member symbol.
    fn insert_tagged_variant_symbol(
        &mut self,
        owner: dir::GlobalSymbolId,
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
            None,
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
