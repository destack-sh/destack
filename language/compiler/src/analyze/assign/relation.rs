use super::*;
use crate::analyze::common::{SymbolTypeView, TreeSymbolView, TypeContext};
use destack_dir::Declaration;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(super) fn check_implicit_collection_conversion(
        &self,
        ctx: &AssignContext<'_>,
        module: &Module,
        profile: ProfileId,
        anchor: LocalNodeIdAny,
    ) {
        // native outputs handle this in elaborate reify
        if self.profile(profile).key.emit.is_native() {
            return;
        }

        // read the conversion policy from config
        let policy = ctx
            .compiler_context
            .compiler_options_for_module(module)
            .map(|options| options.implicit_collection_conversions)
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
    pub(super) fn report_unsound_variance(&self, ctx: &AssignContext<'_>, anchor: LocalNodeIdAny) {
        if !ctx.options.no_unsound_variance {
            return;
        }
        let node = anchor
            .into_global(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::UnsoundVarianceDisabled { node });
    }

    /// Check if `source` type is assignable to `target` type.
    /// Returns true if a value of type `source` can be assigned to a location of type `target`.
    pub(crate) fn is_type_assignable(
        &self,
        ctx: &mut TypeContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) -> Assignability {
        // same type id: trivially assignable
        if target_id == source_id {
            return Assignability::Assignable;
        }

        let _timing = self.timing_scope(tags::ANALYZE_INFER_ASSIGN_CHECK);

        let mut ctx = AssignContext {
            compiler_context: ctx.compiler_context,
            module: ctx.module,
            profile: ctx.profile,
            tree: ctx.tree,
            symbols: ctx.symbols,
            index: ctx.index.clone(),
            types: ctx.types,
            options: ctx.options,
        };

        // normalize and resolve apparent types for assignability
        let target_id = self.normalize_apparent_type(
            &mut ctx.type_context_reborrow(),
            target_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let source_id = self.normalize_apparent_type(
            &mut ctx.type_context_reborrow(),
            source_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );

        // recheck equality after normalization
        if target_id == source_id {
            return Assignability::Assignable;
        }

        self.is_type_assignable_inner(&mut ctx, target_id, source_id)
    }

    /// Check one type assignment while preserving the current assign context.
    pub(super) fn is_type_assignable_in_context(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) -> Assignability {
        self.is_type_assignable(&mut ctx.type_context_reborrow(), target_id, source_id)
    }

    /// Inner assignability check on Type values.
    pub(super) fn is_type_assignable_inner(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) -> Assignability {
        let target_source_id = ctx.types.get_type_source(target_id);
        let source_source_id = ctx.types.get_type_source(source_id);

        // follow alias references and static constraints before assignability
        let target_id =
            self.prepare_assignability_type(&mut ctx.type_context_reborrow(), target_id);
        let source_id =
            self.prepare_assignability_type(&mut ctx.type_context_reborrow(), source_id);

        // re-expand alias targets when normalization preserves references
        if let Type::Reference { symbol, .. } = ctx.types.get_type(target_id)
            && symbol.ty() == SymbolType::TypeAlias
        {
            let normalized_target = self.normalize_type(
                &mut ctx.type_context_reborrow(),
                target_id,
                NormalizationMode::Assign,
            );
            if normalized_target != target_id {
                return self.is_type_assignable_in_context(ctx, normalized_target, source_id);
            }
        }

        // recheck equality after alias expansion
        if target_id == source_id {
            return Assignability::Assignable;
        }

        // recursion guard for assignability pairs
        if !ctx
            .types
            .mark_assignability_in_progress(target_id, source_id)
        {
            return Assignability::Assignable;
        }
        let _assignability_guard = AssignabilityGuard::new(ctx.types, target_id, source_id);

        let target = ctx.types.get_type(target_id).clone();
        let source = ctx.types.get_type(source_id).clone();

        // newtypes are nominal: only the same newtype symbol is assignable
        if let Some(target_symbol) = self.unwrap_type_value_symbol(ctx.types, target_id)
            && target_symbol.ty() == SymbolType::Newtype
        {
            let source_symbol = self.unwrap_type_value_symbol(ctx.types, source_id);
            if source_symbol != Some(target_symbol) {
                return Assignability::NotAssignable;
            }
        }

        // prevent implicit enum backing coercions
        if self.blocks_enum_backing_assignability(&source, &target, ctx.types) {
            return Assignability::NotAssignable;
        }

        // normalize conditional types that can be resolved in flow mode
        if let Some(assignability) =
            self.normalize_conditional_assignability(&mut ctx.reborrow(), target_id, source_id)
        {
            return assignability;
        }

        // infer placeholders: treat as wildcard with optional constraints
        if let Some(assignability) = self.resolve_infer_type_assignability(
            &mut ctx.reborrow(),
            target_id,
            source_id,
            &target,
            &source,
        ) {
            return assignability;
        }

        // conditional sources and targets
        if let Some(assignability) = self.check_conditional_type_assignability(
            &mut ctx.reborrow(),
            target_id,
            source_id,
            &target,
            &source,
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
        if !ctx.options.strict_null_checks
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
            &mut ctx.reborrow(),
            target_id,
            source_id,
            &target,
            &source,
        ) {
            return assignability;
        }

        // wrapper relations
        if let Some(assignability) =
            self.check_wrapper_type_assignability(&mut ctx.reborrow(), &target, &source)
        {
            return assignability;
        }

        // array and tuple relations
        if let Some(assignability) =
            self.check_array_tuple_assignability(&mut ctx.reborrow(), target_id, &target, &source)
        {
            return assignability;
        }

        // object, function, and interface bridge relations
        // reference nominal and structural relations
        {
            let mut relation_ctx = ctx.reborrow();
            if let Some(assignability) = self.check_object_callable_assignability(
                &mut relation_ctx,
                target_id,
                target_source_id,
                source_source_id,
                &target,
                &source,
            ) {
                return assignability;
            }

            if let Some(assignability) = self.check_reference_type_assignability(
                &mut relation_ctx,
                target_id,
                source_id,
                target_source_id,
                &target,
                &source,
            ) {
                return assignability;
            }
        }

        // structural comparison
        match (target, source) {
            // record-like targets: treat assignability as structural via index signature
            (
                Type::Reference {
                    symbol: target_symbol,
                    generic_arguments,
                },
                source_type,
            ) if let Some(index_signature) = self.record_like_index_signature_for_target(
                &mut ctx.type_context_reborrow(),
                target_symbol,
                generic_arguments.as_deref(),
                target_id,
            ) =>
            {
                // check implicit collection conversion policy for record-like targets
                let anchor = ctx.types.get_type_source(target_id);
                self.check_implicit_collection_conversion(ctx, ctx.module, ctx.profile, anchor);

                // accept record-like references when static arguments align
                if let Type::Reference {
                    symbol: source_symbol,
                    generic_arguments: ref source_arguments,
                } = source_type
                {
                    let record_symbol =
                        self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Record);
                    let map_symbol =
                        self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Map);
                    let is_record_like_source = record_symbol
                        .is_some_and(|record_symbol| record_symbol == source_symbol)
                        || map_symbol.is_some_and(|map_symbol| map_symbol == source_symbol);
                    if is_record_like_source
                        && self.are_reference_static_arguments_assignable(
                            &mut ctx.type_context_reborrow(),
                            target_id,
                            source_id,
                            target_symbol,
                            generic_arguments.as_ref(),
                            source_arguments.as_ref(),
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
                )) = self.record_like_source_object_parts(&source_type, ctx.types)
                else {
                    return Assignability::NotAssignable;
                };

                let target_index_signatures = vec![index_signature];
                if self.is_index_signatures_assignable(
                    &mut ctx.reborrow(),
                    &target_index_signatures,
                    &source_index_signatures,
                    &source_fields,
                ) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // type literals: must match exactly (with some exceptions)
            (Type::TypeLiteral { value: target_lit }, Type::TypeLiteral { value: source_lit }) => {
                self.is_type_literal_assignable(&target_lit, &source_lit, ctx.options)
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
                Type::TemplateLiteral { .. },
            ) => Assignability::Assignable,
            (Type::TemplateLiteral { strings, spans }, source_type) => match source_type {
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                } => {
                    if self.template_literal_is_string_supertype(
                        &mut ctx.type_context_reborrow(),
                        &strings,
                        &spans,
                    ) {
                        Assignability::Assignable
                    } else {
                        Assignability::NotAssignable
                    }
                }
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                } => {
                    let value = self.repository.strings.get(string_id).to_string();
                    if self.template_literal_matches_string(
                        &mut ctx.type_context_reborrow(),
                        &strings,
                        &spans,
                        &value,
                    ) {
                        Assignability::Assignable
                    } else {
                        Assignability::NotAssignable
                    }
                }
                Type::TemplateLiteral {
                    strings: source_strings,
                    spans: source_spans,
                } => {
                    if self.template_literal_matches_template(
                        &mut ctx.type_context_reborrow(),
                        &strings,
                        &spans,
                        &source_strings,
                        &source_spans,
                    ) {
                        Assignability::Assignable
                    } else {
                        Assignability::NotAssignable
                    }
                }
                _ => Assignability::NotAssignable,
            },

            // type descriptors: compare underlying value ctx.types
            (
                Type::Value {
                    value: target_value,
                },
                Type::Value {
                    value: source_value,
                },
            ) => self.is_type_assignable_in_context(ctx, target_value, source_value),

            // error types: always assignable (to suppress cascading errors)
            (Type::Error, _) | (_, Type::Error) => Assignability::Assignable,

            // everything else: not assignable
            _ => Assignability::NotAssignable,
        }
    }

    /// Normalize conditional types before structural matching.
    fn normalize_conditional_assignability(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) -> Option<Assignability> {
        // normalize target conditionals that can collapse in flow mode
        if let Some(normalized_target) = self
            .normalize_conditional_for_assignability(&mut ctx.type_context_reborrow(), target_id)
        {
            return Some(self.is_type_assignable_in_context(ctx, normalized_target, source_id));
        }

        // normalize source conditionals that can collapse in flow mode
        if let Some(normalized_source) = self
            .normalize_conditional_for_assignability(&mut ctx.type_context_reborrow(), source_id)
        {
            return Some(self.is_type_assignable_in_context(ctx, target_id, normalized_source));
        }

        None
    }

    /// Resolve infer placeholders as wildcard constraints.
    fn resolve_infer_type_assignability(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target: &Type,
        source: &Type,
    ) -> Option<Assignability> {
        // infer targets: treat as wildcard with optional constraints
        if let Type::Infer { constraint, .. } = target {
            if let Some(constraint_id) = *constraint {
                return Some(self.is_type_assignable_in_context(ctx, constraint_id, source_id));
            }

            return Some(Assignability::Assignable);
        }

        // infer sources: treat as wildcard with optional constraints
        if let Type::Infer { constraint, .. } = source {
            if let Some(constraint_id) = *constraint {
                return Some(self.is_type_assignable_in_context(ctx, target_id, constraint_id));
            }

            return Some(Assignability::Assignable);
        }

        None
    }

    /// Evaluate conditional branch assignability semantics.
    fn check_conditional_type_assignability(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target: &Type,
        source: &Type,
    ) -> Option<Assignability> {
        // conditional targets: at least one branch must accept the source
        if let Type::Conditional {
            then_type,
            else_type,
            ..
        } = target
        {
            let then_assignable = self
                .is_type_assignable(&mut ctx.type_context_reborrow(), *then_type, source_id)
                .is_assignable();
            let else_assignable = self
                .is_type_assignable(&mut ctx.type_context_reborrow(), *else_type, source_id)
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
                &mut ctx.type_context_reborrow(),
                *left,
                *right,
                *then_type,
            );
            let then_assignable = self
                .is_type_assignable(&mut ctx.type_context_reborrow(), target_id, narrowed_then)
                .is_assignable();
            let else_assignable = self
                .is_type_assignable(&mut ctx.type_context_reborrow(), target_id, *else_type)
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
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target: &Type,
        source: &Type,
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
                                &mut ctx.type_context_reborrow(),
                                *target_element,
                                *source_element,
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
                            &mut ctx.type_context_reborrow(),
                            *target_element,
                            source_id,
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
                            &mut ctx.type_context_reborrow(),
                            target_id,
                            *source_element,
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
                            &mut ctx.type_context_reborrow(),
                            *target_element,
                            source_id,
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
                            &mut ctx.type_context_reborrow(),
                            target_id,
                            *source_element,
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
        ctx: &mut AssignContext<'_>,
        target: &Type,
        source: &Type,
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
                    &mut ctx.reborrow(),
                    *target_right,
                    *source_right,
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
                    &mut ctx.reborrow(),
                    *target_right,
                    *source_right,
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
                    &mut ctx.reborrow(),
                    *target_right,
                    *source_right,
                ))
            }

            _ => None,
        }
    }

    /// Evaluate array and tuple relation semantics.
    fn check_array_tuple_assignability(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        target: &Type,
        source: &Type,
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

                let assignability =
                    self.is_type_assignable_in_context(ctx, *target_element, *source_element);

                if assignability.is_assignable() {
                    let anchor = ctx.types.get_type_source(target_id);
                    self.check_unsound_array_variance(
                        &mut ctx.reborrow(),
                        anchor,
                        *target_readonly,
                        *source_readonly,
                        *target_element,
                        *source_element,
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
                let anchor = ctx.types.get_type_source(target_id);
                self.check_implicit_collection_conversion(ctx, ctx.module, ctx.profile, anchor);

                if !self.array_readonly_assignable(*target_readonly, *source_readonly) {
                    return Some(Assignability::NotAssignable);
                }

                let assignability =
                    self.is_type_assignable_in_context(ctx, *target_element, *source_element);

                if assignability.is_assignable() {
                    self.check_unsound_array_variance(
                        &mut ctx.reborrow(),
                        anchor,
                        *target_readonly,
                        *source_readonly,
                        *target_element,
                        *source_element,
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
                let anchor = ctx.types.get_type_source(target_id);
                self.check_implicit_collection_conversion(ctx, ctx.module, ctx.profile, anchor);

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

                let count_matches = self.array_sized_count_matches_length(
                    &mut ctx.reborrow(),
                    *target_count,
                    source_elements.len(),
                );
                let count_matches = match count_matches {
                    Ok(count_matches) => count_matches,
                    Err(error) => {
                        self.error(error);
                        return Some(Assignability::NotAssignable);
                    }
                };
                if !count_matches {
                    return Some(Assignability::NotAssignable);
                }

                for element in source_elements {
                    if element.is_rest
                        || self.is_type_assignable_in_context(ctx, *target_element, element.ty)
                            == Assignability::NotAssignable
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

                let assignability =
                    self.is_type_assignable_in_context(ctx, *target_element, *source_element);
                if !assignability.is_assignable() {
                    return Some(Assignability::NotAssignable);
                }

                let anchor = ctx.types.get_type_source(target_id);
                self.check_unsound_array_variance(
                    &mut ctx.reborrow(),
                    anchor,
                    *target_readonly,
                    *source_readonly,
                    *target_element,
                    *source_element,
                );

                let counts_match = self.array_sized_counts_match(
                    &mut ctx.reborrow(),
                    *target_count,
                    *source_count,
                );
                let counts_match = match counts_match {
                    Ok(counts_match) => counts_match,
                    Err(error) => {
                        self.error(error);
                        return Some(Assignability::NotAssignable);
                    }
                };
                if counts_match {
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
                            &mut ctx.type_context_reborrow(),
                            target_element.ty,
                            source_element.ty,
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
                            &mut ctx.type_context_reborrow(),
                            *target_element,
                            source_element.ty,
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
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        target_source_id: LocalNodeIdAny,
        source_source_id: LocalNodeIdAny,
        target: &Type,
        source: &Type,
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
                    let anchor = ctx.types.get_type_source(target_id);
                    self.check_implicit_collection_conversion(ctx, ctx.module, ctx.profile, anchor);
                }

                let mut relation_ctx = ctx.reborrow();
                Some(self.is_object_type_assignable(
                    &mut relation_ctx,
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                    source_fields,
                    source_call_signatures,
                    source_construct_signatures,
                    source_index_signatures,
                ))
            }

            // objects: structural assignability from reference-like apparent object shapes
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                source_type,
            ) if let Some((
                source_fields,
                source_call_signatures,
                source_construct_signatures,
                source_index_signatures,
            )) = self.record_like_source_object_parts(source_type, ctx.types) =>
            {
                if !target_index_signatures.is_empty() {
                    let anchor = ctx.types.get_type_source(target_id);
                    self.check_implicit_collection_conversion(ctx, ctx.module, ctx.profile, anchor);
                }

                let mut relation_ctx = ctx.reborrow();
                Some(self.is_object_type_assignable(
                    &mut relation_ctx,
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                    &source_fields,
                    &source_call_signatures,
                    &source_construct_signatures,
                    &source_index_signatures,
                ))
            }

            // functions: contravariant params, covariant return
            (
                Type::Function {
                    parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Function {
                    parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => {
                let assignment_anchor = ctx.types.get_type_source(target_id);
                let mut relation_ctx = ctx.reborrow();
                Some(self.is_function_type_assignable(
                    &mut relation_ctx,
                    assignment_anchor,
                    target_params,
                    target_this,
                    target_return,
                    source_params,
                    source_this,
                    source_return,
                ))
            }

            // callable objects: function values can satisfy call signatures
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Function {
                    parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => {
                if !target_index_signatures.is_empty() {
                    let anchor = ctx.types.get_type_source(target_id);
                    self.check_implicit_collection_conversion(ctx, ctx.module, ctx.profile, anchor);
                }

                let mut relation_ctx = ctx.reborrow();
                Some(self.is_object_assignable_from_function(
                    &mut relation_ctx,
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                    source_params,
                    source_this,
                    source_return,
                ))
            }

            // functions: callable object sources must provide a compatible signature
            (
                Type::Function {
                    parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Object {
                    call_signatures: source_call_signatures,
                    ..
                },
            ) => {
                let assignment_anchor = ctx.types.get_type_source(target_id);
                let mut relation_ctx = ctx.reborrow();
                Some(self.is_function_assignable_from_object(
                    &mut relation_ctx,
                    assignment_anchor,
                    target_params,
                    target_this,
                    target_return,
                    source_call_signatures,
                ))
            }

            // interface target: allow structural assignability from object source
            (
                Type::Reference {
                    symbol: target_symbol,
                    generic_arguments: target_static_arguments,
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

                let Some((
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                )) = self.record_like_object_parts_for_reference(
                    &mut ctx.type_context_reborrow(),
                    target_source_id,
                    *target_symbol,
                    target_static_arguments.as_deref(),
                )
                else {
                    return Some(Assignability::NotAssignable);
                };
                let mut relation_ctx = ctx.reborrow();
                Some(self.is_object_type_assignable(
                    &mut relation_ctx,
                    &target_fields,
                    &target_call_signatures,
                    &target_construct_signatures,
                    &target_index_signatures,
                    source_fields,
                    source_call_signatures,
                    source_construct_signatures,
                    source_index_signatures,
                ))
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
                    generic_arguments: source_static_arguments,
                    ..
                },
            ) => {
                if !source_symbol.ty().is_interface() {
                    return None;
                }

                let Some((
                    source_fields,
                    source_call_signatures,
                    source_construct_signatures,
                    source_index_signatures,
                )) = self.record_like_object_parts_for_reference(
                    &mut ctx.type_context_reborrow(),
                    source_source_id,
                    *source_symbol,
                    source_static_arguments.as_deref(),
                )
                else {
                    return Some(Assignability::NotAssignable);
                };
                let mut relation_ctx = ctx.reborrow();
                Some(self.is_object_type_assignable(
                    &mut relation_ctx,
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                    &source_fields,
                    &source_call_signatures,
                    &source_construct_signatures,
                    &source_index_signatures,
                ))
            }

            // interface target: allow function values to satisfy call signatures
            (
                Type::Reference {
                    symbol: target_symbol,
                    generic_arguments: target_static_arguments,
                    ..
                },
                Type::Function {
                    parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => {
                if !target_symbol.ty().is_interface() {
                    return None;
                }

                let Some((
                    target_fields,
                    target_call_signatures,
                    target_construct_signatures,
                    target_index_signatures,
                )) = self.record_like_object_parts_for_reference(
                    &mut ctx.type_context_reborrow(),
                    target_source_id,
                    *target_symbol,
                    target_static_arguments.as_deref(),
                )
                else {
                    return Some(Assignability::NotAssignable);
                };
                let mut relation_ctx = ctx.reborrow();
                Some(self.is_object_assignable_from_function(
                    &mut relation_ctx,
                    &target_fields,
                    &target_call_signatures,
                    &target_construct_signatures,
                    &target_index_signatures,
                    source_params,
                    source_this,
                    source_return,
                ))
            }

            // function target: accept callable interface sources
            (
                Type::Function {
                    parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Reference {
                    symbol: source_symbol,
                    generic_arguments: source_static_arguments,
                    ..
                },
            ) => {
                if !source_symbol.ty().is_interface() {
                    return None;
                }

                let Some((
                    _source_fields,
                    source_call_signatures,
                    _source_construct_signatures,
                    _source_index_signatures,
                )) = self.record_like_object_parts_for_reference(
                    &mut ctx.type_context_reborrow(),
                    source_source_id,
                    *source_symbol,
                    source_static_arguments.as_deref(),
                )
                else {
                    return Some(Assignability::NotAssignable);
                };
                let assignment_anchor = ctx.types.get_type_source(target_id);
                let mut relation_ctx = ctx.reborrow();
                Some(self.is_function_assignable_from_object(
                    &mut relation_ctx,
                    assignment_anchor,
                    target_params,
                    target_this,
                    target_return,
                    &source_call_signatures,
                ))
            }

            _ => None,
        }
    }

    /// Evaluate reference nominal and structural relations.
    fn check_reference_type_assignability(
        &self,
        ctx: &mut AssignContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        target_source_id: LocalNodeIdAny,
        target: &Type,
        source: &Type,
    ) -> Option<Assignability> {
        let (
            Type::Reference {
                symbol: target_symbol,
                generic_arguments: target_arguments,
            },
            Type::Reference {
                symbol: source_symbol,
                generic_arguments: source_arguments,
            },
        ) = (target, source)
        else {
            return None;
        };

        let target_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            *target_symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let target_symbol =
            self.normalize_reference_relation_symbol(ctx.symbol_type_view(), target_symbol);
        let source_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            *source_symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let source_symbol =
            self.normalize_reference_relation_symbol(ctx.symbol_type_view(), source_symbol);

        // expand target alias references into their structural targets
        if target_symbol.ty() == SymbolType::TypeAlias {
            let target_source_id = ctx.types.get_type_source(target_id);
            if let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
                &mut ctx.type_context_reborrow(),
                target_symbol,
                target_source_id,
            ) {
                let prepared_alias_target_id = self
                    .prepare_assignability_type(&mut ctx.type_context_reborrow(), alias_target_id);

                if let Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                } = ctx.types.get_type(prepared_alias_target_id).clone()
                    && let Some((
                        source_fields,
                        source_call_signatures,
                        source_construct_signatures,
                        source_index_signatures,
                    )) = self.record_like_source_object_parts(source, ctx.types)
                {
                    let mut relation_ctx = ctx.reborrow();
                    return Some(self.is_object_type_assignable(
                        &mut relation_ctx,
                        &target_fields,
                        &target_call_signatures,
                        &target_construct_signatures,
                        &target_index_signatures,
                        &source_fields,
                        &source_call_signatures,
                        &source_construct_signatures,
                        &source_index_signatures,
                    ));
                }

                if prepared_alias_target_id != target_id {
                    return Some(self.is_type_assignable_in_context(
                        ctx,
                        prepared_alias_target_id,
                        source_id,
                    ));
                }
            }
        }

        // expand source alias references into their structural targets
        if source_symbol.ty() == SymbolType::TypeAlias {
            let source_source_id = ctx.types.get_type_source(source_id);
            if let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
                &mut ctx.type_context_reborrow(),
                source_symbol,
                source_source_id,
            ) {
                let prepared_alias_target_id = self
                    .prepare_assignability_type(&mut ctx.type_context_reborrow(), alias_target_id);
                if prepared_alias_target_id != source_id {
                    return Some(self.is_type_assignable_in_context(
                        ctx,
                        target_id,
                        prepared_alias_target_id,
                    ));
                }
            }
        }

        if target_symbol == source_symbol {
            if self.are_reference_static_arguments_assignable(
                &mut ctx.type_context_reborrow(),
                target_id,
                source_id,
                target_symbol,
                target_arguments.as_ref(),
                source_arguments.as_ref(),
            ) {
                return Some(Assignability::Assignable);
            }

            return Some(Assignability::NotAssignable);
        }

        let target_is_nominal_interface = self.symbol_is_nominal_interface(
            TreeSymbolView::new(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                ctx.tree,
                ctx.symbols,
            ),
            target_symbol,
        );
        let source_is_nominal_interface = self.symbol_is_nominal_interface(
            TreeSymbolView::new(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                ctx.tree,
                ctx.symbols,
            ),
            source_symbol,
        );

        // nominal interfaces only assign through explicit lineage
        if target_is_nominal_interface || source_is_nominal_interface {
            if self.is_type_lineage_assignable(ctx.symbol_type_view(), source_symbol, target_symbol)
            {
                return Some(Assignability::Assignable);
            }

            return Some(Assignability::NotAssignable);
        }

        // newtypes are nominal: never allow implicit cross symbol assignability
        if matches!(target_symbol.ty(), SymbolType::Newtype)
            || matches!(source_symbol.ty(), SymbolType::Newtype)
        {
            return Some(Assignability::NotAssignable);
        }

        if self.is_type_lineage_assignable(ctx.symbol_type_view(), source_symbol, target_symbol) {
            return Some(Assignability::Assignable);
        }

        if target_symbol.ty().is_interface()
            && let Some(target_instance_id) = self.require_instance_type(
                &mut ctx.type_context_reborrow(),
                target_source_id,
                target_symbol,
            )
        {
            let target_instance = ctx.types.get_type(target_instance_id).clone();
            let Some((
                source_fields,
                source_call_signatures,
                source_construct_signatures,
                source_index_signatures,
            )) = self.record_like_source_object_parts(source, ctx.types)
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
                let mut relation_ctx = ctx.reborrow();
                return Some(self.is_object_type_assignable(
                    &mut relation_ctx,
                    &target_fields,
                    &target_call_signatures,
                    &target_construct_signatures,
                    &target_index_signatures,
                    &source_fields,
                    &source_call_signatures,
                    &source_construct_signatures,
                    &source_index_signatures,
                ));
            }
        }

        Some(Assignability::NotAssignable)
    }

    /// Normalize one reference relation symbol before nominal assignability checks.
    fn normalize_reference_relation_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        self.query_enum_symbol_for_field_symbol(ctx, symbol)
            .unwrap_or(symbol)
    }

    /// Return whether one declared interface symbol is nominal.
    pub(crate) fn symbol_is_nominal_interface(
        &self,
        view: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> bool {
        if symbol.ty() != SymbolType::Interface {
            return false;
        }

        if self
            .require_remote_artifact_dir(
                view.compiler_context,
                view.module.id,
                symbol.module_id,
                view.profile,
                destack_artifact::ArtifactKey::dir_declared,
            )
            .is_err()
        {
            return false;
        }

        let remote_snapshot;

        // read the local declaration tables directly and fall back to the declared remote artifact
        let (tree, symbols) = if symbol.module_id == view.module.id {
            (view.tree, view.symbols)
        } else {
            let Ok(snapshot) = self.require_artifact_dir_declared(
                view.compiler_context.revision(),
                symbol.module_id,
                view.profile,
            ) else {
                return false;
            };
            remote_snapshot = snapshot;
            (
                remote_snapshot.tree.as_ref(),
                remote_snapshot.symbols.as_ref(),
            )
        };

        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return false;
        };
        let Ok(declaration_id) = primary_declaration.try_into_typed::<Declaration>() else {
            return false;
        };

        matches!(
            tree.get(declaration_id.into()),
            Declaration::Interface(declaration) if declaration.is_nominal
        )
    }

    /// Evaluate bidirectional inner assignability for invariant wrappers.
    fn check_bidirectional_inner_assignability(
        &self,
        ctx: &mut AssignContext<'_>,
        target_inner: LocalTypeId,
        source_inner: LocalTypeId,
    ) -> Assignability {
        let target_assignable = self.is_type_assignable_in_context(ctx, target_inner, source_inner);
        let source_assignable = self.is_type_assignable_in_context(ctx, source_inner, target_inner);

        if target_assignable.is_assignable() && source_assignable.is_assignable() {
            Assignability::Assignable
        } else {
            Assignability::NotAssignable
        }
    }
}
