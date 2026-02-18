use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(super) fn check_implicit_collection_conversion(
        &self,
        module: &Module,
        profile: ProfileId,
        anchor: LocalNodeIdAny,
    ) {
        // native outputs handle this in elaborate reify
        if self.program.profile(profile).key.output.is_native() {
            return;
        }

        // read the conversion policy from dsconfig
        let policy = self
            .program
            .with_dsconfig_options(module, |ds| ds.compiler.implicit_collection_conversions)
            .unwrap_or(ImplicitCollectionConversionPolicy::Allow);

        // honor the configured policy
        let node = anchor.into_global(module.id).into_anchored(Some(profile));
        match policy {
            ImplicitCollectionConversionPolicy::Allow => {}
            ImplicitCollectionConversionPolicy::Warn => {
                self.warning(AnalyzeWarning::ImplicitCollectionConversion { node });
            }
            ImplicitCollectionConversionPolicy::Deny => {
                self.error(AnalyzeError::ImplicitCollectionConversion { node });
            }
        }
    }

    /// Report unsound variance usage when configured.
    pub(super) fn report_unsound_variance(
        &self,
        module: &Module,
        profile: ProfileId,
        anchor: LocalNodeIdAny,
        options: &AnalyzeOptions,
    ) {
        if !options.no_unsound_variance {
            return;
        }

        let node = anchor.into_global(module.id).into_anchored(Some(profile));
        self.error(AnalyzeError::UnsoundVarianceDisabled { node });
    }

    /// Check if `source` type is assignable to `target` type.
    /// Returns true if a value of type `source` can be assigned to a location of type `target`.
    pub fn is_type_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // same type id: trivially assignable
        if target_id == source_id {
            return Assignability::Assignable;
        }

        let _timing = self.timing_scope(tags::ANALYZE_INFER_ASSIGN_CHECK);

        // normalize and resolve apparent types for assignability
        let target_id = self.normalize_apparent_type(
            module,
            profile,
            target_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let source_id = self.normalize_apparent_type(
            module,
            profile,
            source_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );

        // recheck equality after normalization
        if target_id == source_id {
            return Assignability::Assignable;
        }

        self.is_type_assignable_inner(
            module, profile, symbols, target_id, source_id, types, options,
        )
    }

    /// Check assignability assuming apparent type normalization is already applied.
    pub(crate) fn is_type_assignable_normalized(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // same type id: trivially assignable
        if target_id == source_id {
            return Assignability::Assignable;
        }

        let _timing = self.timing_scope(tags::ANALYZE_INFER_ASSIGN_CHECK);

        self.is_type_assignable_inner(
            module, profile, symbols, target_id, source_id, types, options,
        )
    }

    /// Check assignability, but defer unresolved infer variables.
    pub(crate) fn is_type_assignable_or_deferred(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // defer unresolved inference variables to constraint solving
        if self.is_infer_var_type(target_id, types) || self.is_infer_var_type(source_id, types) {
            return true;
        }

        // check concrete assignability when both sides are known
        self.is_type_assignable(
            module, profile, symbols, target_id, source_id, types, options,
        ) != Assignability::NotAssignable
    }

    /// Inner assignability check on Type values.
    pub(super) fn is_type_assignable_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // follow alias references and static constraints before assignability
        let target_id = self.prepare_assignability_type(module, profile, target_id, symbols, types);
        let source_id = self.prepare_assignability_type(module, profile, source_id, symbols, types);

        // capture type sources for instance type resolution
        let target_source_id = types.get_type_source(target_id);
        let source_source_id = types.get_type_source(source_id);

        // re-expand alias targets when normalization preserves references
        if let Type::Reference { symbol, .. } = types.get_type(target_id)
            && symbol.ty() == SymbolType::TypeAlias
        {
            let normalized_target = self.normalize_type(
                module,
                profile,
                target_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
            if normalized_target != target_id {
                return self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    normalized_target,
                    source_id,
                    types,
                    options,
                );
            }
        }

        // recheck equality after alias expansion
        if target_id == source_id {
            return Assignability::Assignable;
        }

        // recursion guard for assignability pairs
        if !types.mark_assignability_in_progress(target_id, source_id) {
            return Assignability::Assignable;
        }
        let _assignability_guard = AssignabilityGuard::new(types, target_id, source_id);

        let target = types.get_type(target_id).clone();
        let source = types.get_type(source_id).clone();

        // prevent implicit enum backing coercions
        if self.blocks_enum_backing_assignability(&source, &target, types) {
            return Assignability::NotAssignable;
        }

        // normalize conditional types that can be resolved in flow mode
        if let Some(assignability) = self.normalize_conditional_assignability(
            module, profile, symbols, target_id, source_id, types, options,
        ) {
            return assignability;
        }

        // infer placeholders: treat as wildcard with optional constraints
        if let Some(assignability) = self.resolve_infer_type_assignability(
            module, profile, symbols, target_id, source_id, &target, &source, types, options,
        ) {
            return assignability;
        }

        // conditional sources and targets
        if let Some(assignability) = self.check_conditional_type_assignability(
            module, profile, symbols, target_id, source_id, &target, &source, types, options,
        ) {
            return assignability;
        }

        // handle special target types first
        match &target {
            Type::InferVar { .. } => return Assignability::Assignable,
            // any accepts everything
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => return Assignability::Assignable,

            // unknown accepts everything
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            } => return Assignability::Assignable,

            // never accepts nothing
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            } => {
                if matches!(
                    source,
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    }
                ) {
                    return Assignability::Assignable;
                }
                return Assignability::NotAssignable;
            }

            // object accepts any non-primitive type
            Type::TypeLiteral {
                value: TypeLiteral::Object,
            } => {
                return match &source {
                    // non-primitives are assignable to object
                    Type::Object { .. }
                    | Type::Array { .. }
                    | Type::ArraySized { .. }
                    | Type::Tuple { .. }
                    | Type::Function { .. } => Assignability::Assignable,
                    // references to classes/interfaces are assignable to object
                    Type::Reference { symbol, .. }
                        if matches!(
                            symbol.ty(),
                            SymbolType::Class | SymbolType::Interface | SymbolType::Struct
                        ) =>
                    {
                        Assignability::Assignable
                    }
                    // object literal to object
                    Type::TypeLiteral {
                        value: TypeLiteral::Object,
                    } => Assignability::Assignable,
                    // primitives, null, undefined, etc. are NOT assignable to object
                    _ => Assignability::NotAssignable,
                };
            }

            _ => {}
        }

        // handle special source types
        match &source {
            Type::InferVar { .. } => return Assignability::Assignable,
            // never is assignable to everything (bottom type)
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            } => return Assignability::Assignable,

            // any is assignable to everything (escape hatch)
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => return Assignability::Assignable,

            _ => {}
        }

        // allow nullish assignments when strict null checks are disabled
        if !options.strict_null_checks
            && matches!(
                source,
                Type::TypeLiteral {
                    value: TypeLiteral::Null | TypeLiteral::Undefined,
                }
            )
        {
            if matches!(
                target,
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                }
            ) {
                return Assignability::NotAssignable;
            }

            return Assignability::Assignable;
        }

        // union and intersection relations
        if let Some(assignability) = self.check_union_intersection_assignability(
            module, profile, symbols, target_id, source_id, &target, &source, types, options,
        ) {
            return assignability;
        }

        // wrapper relations
        if let Some(assignability) = self.check_wrapper_type_assignability(
            module, profile, symbols, &target, &source, types, options,
        ) {
            return assignability;
        }

        // array and tuple relations
        if let Some(assignability) = self.check_array_tuple_assignability(
            module, profile, symbols, target_id, &target, &source, types, options,
        ) {
            return assignability;
        }

        // object, function, and interface bridge relations
        if let Some(assignability) = self.check_object_callable_assignability(
            module,
            profile,
            symbols,
            target_id,
            target_source_id,
            source_source_id,
            &target,
            &source,
            types,
            options,
        ) {
            return assignability;
        }

        // reference nominal and structural relations
        if let Some(assignability) = self.check_reference_type_assignability(
            module,
            profile,
            symbols,
            target_id,
            source_id,
            target_source_id,
            &target,
            &source,
            types,
            options,
        ) {
            return assignability;
        }

        // structural comparison
        match (target, source) {
            // record-like targets: treat assignability as structural via index signature
            (
                Type::Reference {
                    symbol: target_symbol,
                    static_arguments,
                },
                source_type,
            ) if let Some(index_signature) = self.record_like_index_signature_for_target(
                module,
                profile,
                target_symbol,
                static_arguments.as_deref(),
                symbols,
                types,
                target_id,
            ) =>
            {
                // check implicit collection conversion policy for record-like targets
                let anchor = types.get_type_source(target_id);
                self.check_implicit_collection_conversion(module, profile, anchor);

                // accept record-like references when static arguments align
                if let Type::Reference {
                    symbol: source_symbol,
                    static_arguments: ref source_arguments,
                } = source_type
                {
                    let record_symbol =
                        self.get_well_known_type_symbol(profile, WellKnownSymbol::Record);
                    let map_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Map);
                    let is_record_like_source = record_symbol
                        .is_some_and(|record_symbol| record_symbol == source_symbol)
                        || map_symbol.is_some_and(|map_symbol| map_symbol == source_symbol);
                    if is_record_like_source
                        && self.reference_static_arguments_assignable(
                            module,
                            profile,
                            target_id,
                            source_id,
                            target_symbol,
                            static_arguments.as_ref(),
                            source_arguments.as_ref(),
                            symbols,
                            types,
                            options,
                        )
                    {
                        return Assignability::Assignable;
                    }
                }

                let Some((
                    source_fields,
                    _source_call_signatures,
                    _source_construct_signatures,
                    source_index_signatures,
                )) = self.record_like_source_object_parts(&source_type, types)
                else {
                    return Assignability::NotAssignable;
                };

                let target_index_signatures = vec![index_signature];
                if self.is_index_signatures_assignable(
                    module,
                    profile,
                    symbols,
                    &target_index_signatures,
                    &source_index_signatures,
                    &source_fields,
                    types,
                    options,
                ) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // type literals: must match exactly (with some exceptions)
            (Type::TypeLiteral { value: target_lit }, Type::TypeLiteral { value: source_lit }) => {
                self.is_type_literal_assignable(&target_lit, &source_lit, options)
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
                Type::TemplateLiteral { .. },
            ) => Assignability::Assignable,
            (
                Type::TemplateLiteral { strings, spans },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
            ) => {
                if self.template_literal_is_string_supertype(
                    module, profile, &strings, &spans, symbols, types,
                ) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }
            (
                Type::TemplateLiteral { strings, spans },
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                },
            ) => {
                let value = self.program.strings.get(string_id).to_string();
                if self.template_literal_matches_string(
                    module, profile, &strings, &spans, &value, symbols, types,
                ) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }
            (
                Type::TemplateLiteral {
                    strings: target_strings,
                    spans: target_spans,
                },
                Type::TemplateLiteral {
                    strings: source_strings,
                    spans: source_spans,
                },
            ) => {
                if self.template_literal_matches_template(
                    module,
                    profile,
                    &target_strings,
                    &target_spans,
                    &source_strings,
                    &source_spans,
                    symbols,
                    types,
                    options,
                ) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // type descriptors: compare underlying value types
            (
                Type::Value {
                    value: target_value,
                },
                Type::Value {
                    value: source_value,
                },
            ) => self.is_type_assignable(
                module,
                profile,
                symbols,
                target_value,
                source_value,
                types,
                options,
            ),

            // error types: always assignable (to suppress cascading errors)
            (Type::Error, _) | (_, Type::Error) => Assignability::Assignable,

            // everything else: not assignable
            _ => Assignability::NotAssignable,
        }
    }

    /// Normalize conditional types before structural matching.
    fn normalize_conditional_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        // normalize target conditionals that can collapse in flow mode
        if let Some(normalized_target) =
            self.normalize_conditional_for_assignability(module, profile, target_id, symbols, types)
        {
            return Some(self.is_type_assignable(
                module,
                profile,
                symbols,
                normalized_target,
                source_id,
                types,
                options,
            ));
        }

        // normalize source conditionals that can collapse in flow mode
        if let Some(normalized_source) =
            self.normalize_conditional_for_assignability(module, profile, source_id, symbols, types)
        {
            return Some(self.is_type_assignable(
                module,
                profile,
                symbols,
                target_id,
                normalized_source,
                types,
                options,
            ));
        }

        None
    }

    /// Resolve infer placeholders as wildcard constraints.
    fn resolve_infer_type_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        // infer targets: treat as wildcard with optional constraints
        if let Type::Infer { constraint, .. } = target {
            if let Some(constraint_id) = *constraint {
                return Some(self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    constraint_id,
                    source_id,
                    types,
                    options,
                ));
            }

            return Some(Assignability::Assignable);
        }

        // infer sources: treat as wildcard with optional constraints
        if let Type::Infer { constraint, .. } = source {
            if let Some(constraint_id) = *constraint {
                return Some(self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_id,
                    constraint_id,
                    types,
                    options,
                ));
            }

            return Some(Assignability::Assignable);
        }

        None
    }

    /// Evaluate conditional branch assignability semantics.
    fn check_conditional_type_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        // conditional targets: at least one branch must accept the source
        if let Type::Conditional {
            then_type,
            else_type,
            ..
        } = target
        {
            let then_assignable = self
                .is_type_assignable(
                    module, profile, symbols, *then_type, source_id, types, options,
                )
                .is_assignable();
            let else_assignable = self
                .is_type_assignable(
                    module, profile, symbols, *else_type, source_id, types, options,
                )
                .is_assignable();

            if then_assignable || else_assignable {
                return Some(Assignability::Assignable);
            }

            return Some(Assignability::NotAssignable);
        }

        // conditional sources: every branch must satisfy the target
        if let Type::Conditional {
            then_type,
            else_type,
            left,
            right,
            ..
        } = source
        {
            let narrowed_then = self.narrow_conditional_then_for_assignability(
                module, profile, *left, *right, *then_type, symbols, types,
            );
            let then_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_id,
                    narrowed_then,
                    types,
                    options,
                )
                .is_assignable();
            let else_assignable = self
                .is_type_assignable(
                    module, profile, symbols, target_id, *else_type, types, options,
                )
                .is_assignable();

            if then_assignable && else_assignable {
                return Some(Assignability::Assignable);
            }

            return Some(Assignability::NotAssignable);
        }

        None
    }

    /// Evaluate union and intersection relation semantics.
    fn check_union_intersection_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        match (target, source) {
            // union to union: each source element must fit a target element
            (
                Type::Union {
                    elements: target_elements,
                },
                Type::Union {
                    elements: source_elements,
                },
            ) => {
                // NOTE #Performance: nested union checks are quadratic in element count
                for source_element in source_elements {
                    let mut is_assignable = false;
                    for target_element in target_elements {
                        if self
                            .is_type_assignable(
                                module,
                                profile,
                                symbols,
                                *target_element,
                                *source_element,
                                types,
                                options,
                            )
                            .is_assignable()
                        {
                            is_assignable = true;
                            break;
                        }
                    }

                    if !is_assignable {
                        return Some(Assignability::NotAssignable);
                    }
                }

                Some(Assignability::Assignable)
            }

            // union target: source must be assignable to at least one element
            (
                Type::Union {
                    elements: target_elements,
                },
                _,
            ) => {
                for target_element in target_elements {
                    if self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            *target_element,
                            source_id,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Some(Assignability::Assignable);
                    }
                }

                Some(Assignability::NotAssignable)
            }

            // union source: all elements must be assignable to target
            (
                _,
                Type::Union {
                    elements: source_elements,
                },
            ) => {
                for source_element in source_elements {
                    if !self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_id,
                            *source_element,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Some(Assignability::NotAssignable);
                    }
                }

                Some(Assignability::Assignable)
            }

            // intersection target: source must be assignable to all elements
            (
                Type::Intersection {
                    elements: target_elements,
                },
                _,
            ) => {
                for target_element in target_elements {
                    if !self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            *target_element,
                            source_id,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Some(Assignability::NotAssignable);
                    }
                }

                Some(Assignability::Assignable)
            }

            // intersection source: at least one element must be assignable to target
            (
                _,
                Type::Intersection {
                    elements: source_elements,
                },
            ) => {
                for source_element in source_elements {
                    if self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_id,
                            *source_element,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Some(Assignability::Assignable);
                    }
                }

                Some(Assignability::NotAssignable)
            }

            _ => None,
        }
    }

    /// Evaluate wrapper relation semantics.
    fn check_wrapper_type_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        match (target, source) {
            // owned values: invariant in mutability, variance, and inner type
            (
                Type::ValueOf {
                    mutability: target_mutability,
                    variance: target_variance,
                    right: target_right,
                },
                Type::ValueOf {
                    mutability: source_mutability,
                    variance: source_variance,
                    right: source_right,
                },
            ) => {
                if target_mutability != source_mutability || target_variance != source_variance {
                    return Some(Assignability::NotAssignable);
                }

                Some(self.check_bidirectional_inner_assignability(
                    module,
                    profile,
                    symbols,
                    *target_right,
                    *source_right,
                    types,
                    options,
                ))
            }

            // references: invariant in mutability, variance, and inner type
            (
                Type::ReferenceOf {
                    mutability: target_mutability,
                    variance: target_variance,
                    right: target_right,
                },
                Type::ReferenceOf {
                    mutability: source_mutability,
                    variance: source_variance,
                    right: source_right,
                },
            ) => {
                if target_mutability != source_mutability || target_variance != source_variance {
                    return Some(Assignability::NotAssignable);
                }

                Some(self.check_bidirectional_inner_assignability(
                    module,
                    profile,
                    symbols,
                    *target_right,
                    *source_right,
                    types,
                    options,
                ))
            }

            // pointers: invariant in mutability and pointee type
            (
                Type::PointerOf {
                    mutability: target_mutability,
                    right: target_right,
                },
                Type::PointerOf {
                    mutability: source_mutability,
                    right: source_right,
                },
            ) => {
                if target_mutability != source_mutability {
                    return Some(Assignability::NotAssignable);
                }

                Some(self.check_bidirectional_inner_assignability(
                    module,
                    profile,
                    symbols,
                    *target_right,
                    *source_right,
                    types,
                    options,
                ))
            }

            _ => None,
        }
    }

    /// Evaluate array and tuple relation semantics.
    fn check_array_tuple_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        match (target, source) {
            // arrays: covariant in element type
            (
                Type::Array {
                    element: Some(target_element),
                    is_readonly: target_readonly,
                },
                Type::Array {
                    element: Some(source_element),
                    is_readonly: source_readonly,
                },
            ) => {
                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                let assignability = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *target_element,
                    *source_element,
                    types,
                    options,
                );

                if assignability.is_assignable() {
                    let anchor = types.get_type_source(target_id);
                    self.check_unsound_array_variance(
                        module,
                        profile,
                        symbols,
                        anchor,
                        *target_readonly,
                        *source_readonly,
                        *target_element,
                        *source_element,
                        types,
                        options,
                    );
                }

                Some(assignability)
            }

            // sized arrays: assignable to arrays by element type
            (
                Type::Array {
                    element: Some(target_element),
                    is_readonly: target_readonly,
                },
                Type::ArraySized {
                    element: source_element,
                    is_readonly: source_readonly,
                    ..
                },
            ) => {
                let anchor = types.get_type_source(target_id);
                self.check_implicit_collection_conversion(module, profile, anchor);

                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                let assignability = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *target_element,
                    *source_element,
                    types,
                    options,
                );

                if assignability.is_assignable() {
                    self.check_unsound_array_variance(
                        module,
                        profile,
                        symbols,
                        anchor,
                        *target_readonly,
                        *source_readonly,
                        *target_element,
                        *source_element,
                        types,
                        options,
                    );
                }

                Some(assignability)
            }

            // sized arrays: assignable to unknown element arrays
            (
                Type::Array {
                    element: None,
                    is_readonly: target_readonly,
                },
                Type::ArraySized {
                    is_readonly: source_readonly,
                    ..
                },
            ) => {
                let anchor = types.get_type_source(target_id);
                self.check_implicit_collection_conversion(module, profile, anchor);

                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                Some(Assignability::Assignable)
            }

            // empty array is assignable to any array
            (
                Type::Array {
                    is_readonly: target_readonly,
                    ..
                },
                Type::Array {
                    element: None,
                    is_readonly: source_readonly,
                },
            ) => {
                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                Some(Assignability::Assignable)
            }

            // fixed array from tuple literal
            (
                Type::ArraySized {
                    element: target_element,
                    count: target_count,
                    is_readonly: target_readonly,
                },
                Type::Tuple {
                    elements: source_elements,
                    is_readonly: source_readonly,
                },
            ) => {
                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                if !self.array_sized_count_matches_length(
                    *target_count,
                    source_elements.len(),
                    types,
                ) {
                    return Some(Assignability::NotAssignable);
                }

                for element in source_elements {
                    if element.is_rest
                        || self.is_type_assignable(
                            module,
                            profile,
                            symbols,
                            *target_element,
                            element.ty,
                            types,
                            options,
                        ) == Assignability::NotAssignable
                    {
                        return Some(Assignability::NotAssignable);
                    }
                }

                Some(Assignability::Assignable)
            }

            // fixed arrays: covariant in element type and size
            (
                Type::ArraySized {
                    element: target_element,
                    count: target_count,
                    is_readonly: target_readonly,
                },
                Type::ArraySized {
                    element: source_element,
                    count: source_count,
                    is_readonly: source_readonly,
                },
            ) => {
                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                let assignability = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *target_element,
                    *source_element,
                    types,
                    options,
                );
                if !assignability.is_assignable() {
                    return Some(Assignability::NotAssignable);
                }

                let anchor = types.get_type_source(target_id);
                self.check_unsound_array_variance(
                    module,
                    profile,
                    symbols,
                    anchor,
                    *target_readonly,
                    *source_readonly,
                    *target_element,
                    *source_element,
                    types,
                    options,
                );

                if self.array_sized_counts_match(*target_count, *source_count, types) {
                    Some(Assignability::Assignable)
                } else {
                    Some(Assignability::NotAssignable)
                }
            }

            // tuples: same length and each element assignable
            (
                Type::Tuple {
                    elements: target_elements,
                    is_readonly: target_readonly,
                },
                Type::Tuple {
                    elements: source_elements,
                    is_readonly: source_readonly,
                },
            ) => {
                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                if target_elements.len() != source_elements.len() {
                    return Some(Assignability::NotAssignable);
                }

                for (target_element, source_element) in
                    target_elements.iter().zip(source_elements.iter())
                {
                    let target_element_readonly = *target_readonly || target_element.is_readonly;
                    let source_element_readonly = *source_readonly || source_element.is_readonly;
                    if !self.tuple_element_readonly_assignable(
                        target_element_readonly,
                        source_element_readonly,
                    ) {
                        return Some(Assignability::NotAssignable);
                    }

                    if !self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_element.ty,
                            source_element.ty,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Some(Assignability::NotAssignable);
                    }
                }

                Some(Assignability::Assignable)
            }

            // tuple assignable to array if each element fits the array element type
            (
                Type::Array {
                    element: Some(target_element),
                    is_readonly: target_readonly,
                },
                Type::Tuple {
                    elements: source_elements,
                    is_readonly: source_readonly,
                },
            ) => {
                let source_readonly = self.tuple_is_readonly(*source_readonly);
                if !self.array_readonly_assignable(*target_readonly, source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                for source_element in source_elements {
                    if !self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            *target_element,
                            source_element.ty,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Some(Assignability::NotAssignable);
                    }
                }

                Some(Assignability::Assignable)
            }

            _ => None,
        }
    }

    /// Evaluate object, callable, and interface bridge relations.
    fn check_object_callable_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        target_source_id: LocalNodeIdAny,
        source_source_id: LocalNodeIdAny,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        match (target, source) {
            // objects: structural subtyping
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Object {
                    fields: source_fields,
                    call_signatures: source_call_signatures,
                    construct_signatures: source_construct_signatures,
                    index_signatures: source_index_signatures,
                },
            ) => {
                if !target_index_signatures.is_empty() {
                    let anchor = types.get_type_source(target_id);
                    self.check_implicit_collection_conversion(module, profile, anchor);
                }

                Some(self.is_object_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                    source_fields,
                    source_call_signatures,
                    source_construct_signatures,
                    source_index_signatures,
                    types,
                    options,
                ))
            }

            // functions: contravariant params, covariant return
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => Some(self.is_function_type_assignable(
                module,
                profile,
                symbols,
                types.get_type_source(target_id),
                target_params,
                target_this,
                target_return,
                source_params,
                source_this,
                source_return,
                types,
                options,
            )),

            // callable objects: function values can satisfy call signatures
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => {
                if !target_index_signatures.is_empty() {
                    let anchor = types.get_type_source(target_id);
                    self.check_implicit_collection_conversion(module, profile, anchor);
                }

                Some(self.is_object_assignable_from_function(
                    module,
                    profile,
                    symbols,
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                    source_params,
                    source_this,
                    source_return,
                    types,
                    options,
                ))
            }

            // functions: callable object sources must provide a compatible signature
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Object {
                    call_signatures: source_call_signatures,
                    ..
                },
            ) => Some(self.is_function_assignable_from_object(
                module,
                profile,
                symbols,
                types.get_type_source(target_id),
                target_params,
                target_this,
                target_return,
                source_call_signatures,
                types,
                options,
            )),

            // interface target: allow structural assignability from object source
            (
                Type::Reference {
                    symbol: target_symbol,
                    ..
                },
                Type::Object {
                    fields: source_fields,
                    call_signatures: source_call_signatures,
                    construct_signatures: source_construct_signatures,
                    index_signatures: source_index_signatures,
                },
            ) => {
                if !target_symbol.ty().is_interface() {
                    return None;
                }

                let Some(target_instance_id) = self.require_instance_type(
                    module,
                    profile,
                    target_source_id,
                    *target_symbol,
                    symbols,
                    types,
                ) else {
                    return Some(Assignability::NotAssignable);
                };

                let target_instance = types.get_type(target_instance_id).clone();
                if let Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                } = target_instance
                {
                    return Some(self.is_object_type_assignable(
                        module,
                        profile,
                        symbols,
                        &target_fields,
                        &target_call_signatures,
                        &target_construct_signatures,
                        &target_index_signatures,
                        source_fields,
                        source_call_signatures,
                        source_construct_signatures,
                        source_index_signatures,
                        types,
                        options,
                    ));
                }

                Some(Assignability::NotAssignable)
            }

            // object target: allow interface sources with structural shape
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Reference {
                    symbol: source_symbol,
                    ..
                },
            ) => {
                if !source_symbol.ty().is_interface() {
                    return None;
                }

                let Some(source_instance_id) = self.require_instance_type(
                    module,
                    profile,
                    source_source_id,
                    *source_symbol,
                    symbols,
                    types,
                ) else {
                    return Some(Assignability::NotAssignable);
                };

                let source_instance = types.get_type(source_instance_id).clone();
                if let Type::Object {
                    fields: source_fields,
                    call_signatures: source_call_signatures,
                    construct_signatures: source_construct_signatures,
                    index_signatures: source_index_signatures,
                } = source_instance
                {
                    return Some(self.is_object_type_assignable(
                        module,
                        profile,
                        symbols,
                        target_fields,
                        target_call_signatures,
                        target_construct_signatures,
                        target_index_signatures,
                        &source_fields,
                        &source_call_signatures,
                        &source_construct_signatures,
                        &source_index_signatures,
                        types,
                        options,
                    ));
                }

                Some(Assignability::NotAssignable)
            }

            // interface target: allow function values to satisfy call signatures
            (
                Type::Reference {
                    symbol: target_symbol,
                    ..
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => {
                if !target_symbol.ty().is_interface() {
                    return None;
                }

                let Some(target_instance_id) = self.require_instance_type(
                    module,
                    profile,
                    target_source_id,
                    *target_symbol,
                    symbols,
                    types,
                ) else {
                    return Some(Assignability::NotAssignable);
                };

                let target_instance = types.get_type(target_instance_id).clone();
                if let Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                } = target_instance
                {
                    return Some(self.is_object_assignable_from_function(
                        module,
                        profile,
                        symbols,
                        &target_fields,
                        &target_call_signatures,
                        &target_construct_signatures,
                        &target_index_signatures,
                        source_params,
                        source_this,
                        source_return,
                        types,
                        options,
                    ));
                }

                Some(Assignability::NotAssignable)
            }

            // function target: accept callable interface sources
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Reference {
                    symbol: source_symbol,
                    ..
                },
            ) => {
                if !source_symbol.ty().is_interface() {
                    return None;
                }

                let Some(source_instance_id) = self.require_instance_type(
                    module,
                    profile,
                    source_source_id,
                    *source_symbol,
                    symbols,
                    types,
                ) else {
                    return Some(Assignability::NotAssignable);
                };

                let source_instance = types.get_type(source_instance_id).clone();
                if let Type::Object {
                    call_signatures: source_call_signatures,
                    ..
                } = source_instance
                {
                    return Some(self.is_function_assignable_from_object(
                        module,
                        profile,
                        symbols,
                        types.get_type_source(target_id),
                        target_params,
                        target_this,
                        target_return,
                        &source_call_signatures,
                        types,
                        options,
                    ));
                }

                Some(Assignability::NotAssignable)
            }

            _ => None,
        }
    }

    /// Evaluate reference nominal and structural relations.
    fn check_reference_type_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target_source_id: LocalNodeIdAny,
        target: &Type,
        source: &Type,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<Assignability> {
        let (
            Type::Reference {
                symbol: target_symbol,
                static_arguments: target_arguments,
            },
            Type::Reference {
                symbol: source_symbol,
                static_arguments: source_arguments,
            },
        ) = (target, source)
        else {
            return None;
        };

        let target_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            *target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let source_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            *source_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        if target_symbol == source_symbol {
            if self.reference_static_arguments_assignable(
                module,
                profile,
                target_id,
                source_id,
                target_symbol,
                target_arguments.as_ref(),
                source_arguments.as_ref(),
                symbols,
                types,
                options,
            ) {
                return Some(Assignability::Assignable);
            }

            return Some(Assignability::NotAssignable);
        }

        if self.is_type_lineage_assignable(
            module,
            profile,
            source_symbol,
            target_symbol,
            symbols,
            types,
        ) {
            return Some(Assignability::Assignable);
        }

        if target_symbol.ty().is_interface()
            && let Some(target_instance_id) = self.require_instance_type(
                module,
                profile,
                target_source_id,
                target_symbol,
                symbols,
                types,
            )
        {
            let target_instance = types.get_type(target_instance_id).clone();
            let Some((
                source_fields,
                source_call_signatures,
                source_construct_signatures,
                source_index_signatures,
            )) = self.record_like_source_object_parts(source, types)
            else {
                return Some(Assignability::NotAssignable);
            };

            if let Type::Object {
                fields: target_fields,
                call_signatures: target_call_signatures,
                construct_signatures: target_construct_signatures,
                index_signatures: target_index_signatures,
            } = target_instance
            {
                return Some(self.is_object_type_assignable(
                    module,
                    profile,
                    symbols,
                    &target_fields,
                    &target_call_signatures,
                    &target_construct_signatures,
                    &target_index_signatures,
                    &source_fields,
                    &source_call_signatures,
                    &source_construct_signatures,
                    &source_index_signatures,
                    types,
                    options,
                ));
            }
        }

        Some(Assignability::NotAssignable)
    }

    /// Evaluate bidirectional inner assignability for invariant wrappers.
    fn check_bidirectional_inner_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_inner: LocalTypeId,
        source_inner: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        let target_assignable = self.is_type_assignable(
            module,
            profile,
            symbols,
            target_inner,
            source_inner,
            types,
            options,
        );
        let source_assignable = self.is_type_assignable(
            module,
            profile,
            symbols,
            source_inner,
            target_inner,
            types,
            options,
        );

        if target_assignable.is_assignable() && source_assignable.is_assignable() {
            Assignability::Assignable
        } else {
            Assignability::NotAssignable
        }
    }
}
