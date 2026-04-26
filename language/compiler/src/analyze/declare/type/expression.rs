use crate::analyze::AssociatedProjectionSelection;
use crate::analyze::common::{CanonicalSymbolMode, RelationMode, TypeContext};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Asynchrony, Declaration, DependencyItem, Expression, FunctionCardinality, FunctionKind,
    FunctionMode, FunctionSignature, GenericArgument, GenericParameterKind, GlobalSymbolId,
    IntrinsicType, LocalNodeId, LocalNodeIdAny, LocalTypeId, MappedTypeModifier,
    MappedTypeModifiers, NodeTree, NodeType, NodeVisitor, NodeVisitorOptions, NormalizationMode,
    Parameter, Path, PredicateSubject, PrimitiveType, ScalarLiteral, StaticKey, SymbolSpace,
    SymbolSpaceOrder, TupleElement, Type, TypeElement, TypeExpression, TypeField,
    TypeIndexSignature, TypeLiteral, TypeMember, TypeModifier, TypePredicateSubject,
    walk_type_expression,
};
use destack_workspace::ModuleSource;
use std::collections::HashSet;

use super::cache::DeclaredTypeResolutionContext;
use super::resolve::{TypeIndexResolutionKind, TypeMemberResolution};

/// Visitor for validating static value parameter usage in type expressions.
#[derive(Debug)]
struct StaticValueParameterValidator<'a> {
    /// The compiler shared state.
    compiler: &'a Compiler,
    /// The shared type ctx context.
    ctx: TypeContext<'a>,
    /// Whether to validate static argument bounds.
    validate_static_argument_bounds: bool,
    /// Whether to enforce implicit managed semantics.
    enforce_implicit_managed: bool,
    /// Track the first error encountered while walking.
    result: AnalyzeResult<()>,
    /// Node visitor options (unused, but required by trait).
    options: NodeVisitorOptions,
}

impl<'a> StaticValueParameterValidator<'a> {
    /// Create a new static value parameter validator.
    fn new(
        compiler: &'a Compiler,
        ctx: TypeContext<'a>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> Self {
        // build the validator state
        Self {
            compiler,
            ctx,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            result: Ok(()),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Check if this validator should continue.
    fn should_continue(&self) -> bool {
        // stop once an error is recorded
        self.result.is_ok()
    }

    /// Record a validation result, preserving the first error.
    fn record_result(&mut self, result: AnalyzeResult<()>) {
        // keep the first error in the validator
        if self.result.is_ok() && result.is_err() {
            self.result = result;
        }
    }

    /// Finish the walk and return the result.
    fn finish(self) -> AnalyzeResult<()> {
        // return the recorded result
        self.result
    }
}

impl NodeVisitor for StaticValueParameterValidator<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_type_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeExpression>,
        expression: &TypeExpression,
    ) {
        // stop on first error
        if !self.should_continue() {
            return;
        }

        // validate array-size usage for type index expressions
        if let TypeExpression::Index { left, index } = expression {
            // resolve the left type to decide between index access and array sizes
            let left_id = {
                match self.compiler.resolve_declared_type_expression(
                    &mut self.ctx.reborrow(),
                    *left,
                    self.validate_static_argument_bounds,
                    self.enforce_implicit_managed,
                ) {
                    Ok(left_id) => left_id,
                    Err(error) => {
                        self.record_result(Err(error));
                        return;
                    }
                }
            };

            let interpretation = match self.compiler.type_index_interpretation(
                &mut self.ctx.reborrow(),
                left_id,
                *index,
            ) {
                Ok(value) => value,
                Err(error) => {
                    self.record_result(Err(error));
                    return;
                }
            };

            if interpretation == TypeIndexResolutionKind::ArraySized {
                let is_array_size_candidate = match self
                    .compiler
                    .type_expression_is_array_size_candidate(&mut self.ctx.reborrow(), *index)
                {
                    Ok(value) => value,
                    Err(error) => {
                        self.record_result(Err(error));
                        return;
                    }
                };

                if is_array_size_candidate {
                    let result = self.compiler.resolve_array_size_type_parameter(
                        &mut self.ctx.reborrow(),
                        *index,
                        self.validate_static_argument_bounds,
                        self.enforce_implicit_managed,
                    );
                    self.record_result(result.map(|_| ()));
                }
            }
        }

        // walk nested expression nodes
        destack_core::ensure_sufficient_stack(|| {
            walk_type_expression(self, tree, id, expression);
        });
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Query one declared type expression value when its dependencies are ready.
    pub(crate) fn query_declared_type_expression_value(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
    ) -> AnalyzeResult<Option<Type>> {
        match self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            expression_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
            false,
            false,
        ) {
            Ok(ty) => Ok(Some(ty)),
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Resolve one bare intrinsic marker to its semantic intrinsic literal.
    fn intrinsic_alias_literal(
        &self,
        ctx: &TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<TypeLiteral> {
        // enclosing declaration
        let parent_id = ctx.tree.get_parent(expression_id.id)?;
        let declaration_id = parent_id.try_into_typed::<Declaration>().ok()?;
        let Declaration::Type(declaration) = ctx.tree.get(declaration_id) else {
            return None;
        };

        // intrinsic alias name
        let name = self.repository.strings.get(declaration.name.string());
        let intrinsic = match name.as_ref() {
            "Uppercase" => IntrinsicType::Uppercase,
            "Lowercase" => IntrinsicType::Lowercase,
            "Capitalize" => IntrinsicType::Capitalize,
            "Uncapitalize" => IntrinsicType::Uncapitalize,
            "NoInfer" => IntrinsicType::NoInfer,
            "BuiltinIteratorReturn" => IntrinsicType::BuiltinIteratorReturn,
            _ => return None,
        };

        Some(TypeLiteral::Intrinsic(intrinsic))
    }

    /// Return whether one declared index receiver is concrete enough for missing-member diagnostics.
    fn declared_index_receiver_is_diagnostic_concrete(
        &self,
        ctx: TypeContext<'_>,
        receiver_type_id: LocalTypeId,
    ) -> bool {
        let contains_any = self.type_contains_any(ctx.type_view(), receiver_type_id);
        let contains_unknown = self.type_contains_unknown(ctx.type_view(), receiver_type_id);
        let has_unevaluated_state =
            self.type_has_unevaluated_state(receiver_type_id, ctx.types, &mut HashSet::new());

        !(contains_any || contains_unknown || has_unevaluated_state)
    }

    /// Evaluate a Type (in-place).
    /// Converts Type::Unevaluated to the actual Type value.
    pub(crate) fn resolve_declared_type(
        &self,
        ctx: &mut TypeContext<'_>,
        ty_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        // pull the unevaluated expression id when needed
        let expression_id = {
            let ty = ctx.types.get_type(ty_id);
            let Type::Unevaluated(expression_id) = *ty else {
                return Ok(());
            };
            expression_id
        };
        if !ctx.tree.has_node_id(expression_id.id) {
            ctx.types.update_type(ty_id, Type::Error);
            return Ok(());
        }

        // evaluate and update in place
        // avoid eager static argument resolution for declaration modules
        let resolve_static_arguments = !ctx.module.language_type.is_declaration();
        let evaluated_ty = self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            expression_id,
            true,
            true,
            resolve_static_arguments,
            true,
            true,
        )?;

        // keep unresolved late materialization deferred
        if matches!(evaluated_ty, Type::Unevaluated(_)) {
            return Ok(());
        }

        ctx.types.update_type(ty_id, evaluated_ty);
        if !ctx.types.get_type(ty_id).is_unevaluated() {
            let cache_context =
                DeclaredTypeResolutionContext::new(true, true, resolve_static_arguments);
            self.cache_expression_type_maybe(
                ctx.module.id,
                expression_id,
                cache_context,
                Some(ty_id),
                None,
                false,
                ctx.types,
            );
        }

        Ok(())
    }

    /// Try to evaluate an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::Unevaluated if it fails.
    /// Set validate_static_argument_bounds to false to defer bound checks.
    /// Set enforce_implicit_managed to false to skip noImplicitManaged enforcement.
    /// Set resolve_static_arguments to false to preserve alias argument structure.
    /// Set use_declared_cache to false to skip declared cache lookups.
    pub(crate) fn resolve_declared_type_expression_value(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
        use_declared_cache: bool,
        use_expression_cache: bool,
    ) -> AnalyzeResult<Type> {
        // build cache context for this evaluation
        if !ctx.tree.has_node_id(expression_id.id) {
            return Ok(Type::Error);
        }

        let global_node_id = expression_id.into_global_any(ctx.module.id);
        let cache_context = DeclaredTypeResolutionContext::new(
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        );
        let cache_key = cache_context.cache_key(global_node_id);
        let is_reference_expression = matches!(
            ctx.tree.get(expression_id),
            TypeExpression::LocalReference { .. }
                | TypeExpression::ModuleReference { .. }
                | TypeExpression::GlobalReference { .. }
        );

        // reuse cached expression types when available
        if use_expression_cache {
            if let Some(existing_id) = ctx.types.get_expression_type_id_cache(cache_key) {
                let ty = ctx.types.get_type(existing_id);
                if !ty.is_unevaluated() {
                    return Ok(ty.clone());
                }
            }
            if let Some(existing) = ctx.types.get_expression_type_value_cache(cache_key) {
                return Ok(existing.clone());
            }
        }

        // reuse cached declared types when requested
        if use_declared_cache
            && let Some(existing) = ctx.types.get_declared_type_id(global_node_id)
            && !ctx.types.get_type(existing).is_unevaluated()
        {
            let ty = ctx.types.get_type(existing);
            let is_unknown_reference = is_reference_expression && ty.is_unknown();
            if !is_unknown_reference {
                let ty = ty.clone();
                self.cache_expression_type_maybe(
                    ctx.module.id,
                    expression_id,
                    cache_context,
                    Some(existing),
                    Some(&ty),
                    is_reference_expression,
                    ctx.types,
                );
                return Ok(ty);
            }
        }

        // keep recursive reentry deferrable here
        // direct and instantiated alias cycles are handled by declaration
        // collection and alias normalization
        if ctx.types.is_expression_type_in_progress(global_node_id) {
            return Ok(Type::Unevaluated(expression_id));
        }
        ctx.types.mark_expression_type_in_progress(global_node_id);

        let result = {
            let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION);
            self.resolve_declared_expression_type(
                &mut ctx.reborrow(),
                expression_id,
                validate_static_argument_bounds,
                enforce_implicit_managed,
                resolve_static_arguments,
            )
        };

        ctx.types.clear_expression_type_in_progress(global_node_id);

        // evaluate to a concrete type when possible
        let ty = result?.unwrap_or(Type::Unevaluated(expression_id));
        if use_expression_cache {
            self.cache_expression_type_maybe(
                ctx.module.id,
                expression_id,
                cache_context,
                None,
                Some(&ty),
                is_reference_expression,
                ctx.types,
            );
        }
        Ok(ty)
    }

    /// Evaluate an expression into a static integer literal when possible.
    pub(crate) fn validate_static_value_parameter_usage(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<()> {
        // skip comptime checks outside Destack modules
        if !ctx.module.language_type.is_destack() {
            return Ok(());
        }

        let mut validator = StaticValueParameterValidator::new(
            self,
            ctx.reborrow(),
            validate_static_argument_bounds,
            enforce_implicit_managed,
        );

        // walk the expression tree to validate index usages
        let expression = validator.ctx.tree.get(expression_id);
        validator.visit_type_expression(validator.ctx.tree, expression_id, expression);
        validator.finish()
    }

    /// Query static value parameter usage when its dependencies are ready.
    pub(crate) fn query_static_value_parameter_usage(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<()>> {
        match self.validate_static_value_parameter_usage(
            &mut ctx.reborrow(),
            expression_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        ) {
            Ok(()) => Ok(Some(())),
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Query one declared type expression when its dependencies are ready.
    pub(crate) fn query_declared_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        match self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            expression_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        ) {
            Ok(type_id) => Ok(Some(type_id)),
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Register a scalar literal type for a static integer expression.
    pub(crate) fn resolve_declared_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse cached expression types when available
        let global_node_id = expression_id.into_global_any(ctx.module.id);
        let cache_context = DeclaredTypeResolutionContext::new(
            validate_static_argument_bounds,
            enforce_implicit_managed,
            true,
        );
        let cache_key = cache_context.cache_key(global_node_id);
        let is_reference_expression = matches!(
            ctx.tree.get(expression_id),
            TypeExpression::LocalReference { .. }
                | TypeExpression::ModuleReference { .. }
                | TypeExpression::GlobalReference { .. }
        );
        let mut cached_type_id = None;
        if let Some(existing) = ctx.types.get_expression_type_id_cache(cache_key)
            && !ctx.types.get_type(existing).is_unevaluated()
        {
            cached_type_id = Some(existing);
        } else if let Some(existing) = ctx.types.get_declared_type_id(global_node_id) {
            if ctx.types.get_type(existing).is_unevaluated() {
                // skip unevaluated declared entries and re-evaluate
            } else {
                let ty = ctx.types.get_type(existing);
                let is_unknown_reference = is_reference_expression && ty.is_unknown();
                if !is_unknown_reference {
                    let ty = ty.clone();
                    self.cache_expression_type_maybe(
                        ctx.module.id,
                        expression_id,
                        cache_context,
                        Some(existing),
                        Some(&ty),
                        is_reference_expression,
                        ctx.types,
                    );
                    cached_type_id = Some(existing);
                }
            }
        }
        if let Some(existing) = cached_type_id {
            // materialize the type value so downstream phases do not see an untyped expression
            let type_value = Type::Value { value: existing };
            let type_value_id = ctx.types.insert_type_from(type_value, expression_id);
            ctx.types
                .set_inferred_type(expression_id.into_global_any(ctx.module.id), type_value_id);
            return Ok(existing);
        }

        // evaluate to a concrete type when possible
        let ty = self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            expression_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            true,
            true,
            true,
        )?;
        let ty_id = ctx.types.insert_type_from(ty.clone(), expression_id);

        // cache resolved type expressions for reuse
        self.cache_declared_type_maybe(global_node_id, cache_context, ty_id, &ty, ctx.types);
        self.cache_expression_type_maybe(
            ctx.module.id,
            expression_id,
            cache_context,
            Some(ty_id),
            Some(&ty),
            is_reference_expression,
            ctx.types,
        );

        // materialize the type value so downstream phases do not see an untyped expression
        let type_value = Type::Value { value: ty_id };
        let type_value_id = ctx.types.insert_type_from(type_value, expression_id);
        ctx.types
            .set_inferred_type(expression_id.into_global_any(ctx.module.id), type_value_id);

        Ok(ty_id)
    }

    /// Evaluate a function signature into a Type.
    pub(crate) fn resolve_declared_function_signature_type(
        &self,
        ctx: &mut TypeContext<'_>,
        signature: &FunctionSignature,
        source_id: LocalNodeIdAny,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<Type> {
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_SIGNATURE);

        // collect static parameter placeholders
        let generic_parameters =
            self.generic_parameter_placeholders_for_signature(&mut ctx.reborrow(), signature);

        // evaluate parameter types
        let mut parameters = Vec::with_capacity(signature.parameters.len());
        for parameter_id in signature.parameters.iter() {
            let declared_type_id = ctx
                .types
                .get_declared_type_id(parameter_id.into_global(ctx.module.id).into())
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    ctx.types.insert_type_from(ty, *parameter_id)
                });
            if !defer_type_evaluation {
                self.resolve_declared_type(&mut ctx.reborrow(), declared_type_id)?;
            }

            // expand tuple rest parameters into positional call parameters
            if matches!(
                ctx.tree.get(*parameter_id),
                Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
            ) && let Type::Tuple { elements, .. } = ctx.types.get_type(declared_type_id)
            {
                for element in elements {
                    parameters.push(element.ty);
                }
                continue;
            }

            parameters.push(declared_type_id);
        }

        // evaluate this parameter when present
        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            let declared_type_id = ctx
                .types
                .get_declared_type_id(this_parameter_id.into_global(ctx.module.id).into())
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    ctx.types.insert_type_from(ty, this_parameter_id)
                });
            if !defer_type_evaluation {
                self.resolve_declared_type(&mut ctx.reborrow(), declared_type_id)?;
            }

            Some(declared_type_id)
        } else {
            None
        };

        // evaluate return type
        let return_type = if let Some(return_type_expression_id) = signature.return_type {
            if defer_type_evaluation {
                Some(self.declared_type_id_for_expression_or_insert(
                    &mut ctx.reborrow(),
                    return_type_expression_id,
                ))
            } else {
                let return_type = self.resolve_declared_type_expression_value(
                    &mut ctx.reborrow(),
                    return_type_expression_id,
                    true,
                    true,
                    true,
                    true,
                    true,
                )?;
                let return_type_id = ctx
                    .types
                    .insert_type_from(return_type, return_type_expression_id);
                ctx.types.set_declared_type(
                    return_type_expression_id.into_global_any(ctx.module.id),
                    return_type_id,
                );
                Some(return_type_id)
            }
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Some(ctx.types.insert_type_from_any(ty, source_id))
        };

        // build function type
        Ok(Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        })
    }

    /// Return a declared type id for an expression, inserting an unevaluated type if needed.
    fn declared_type_id_for_expression_or_insert(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalTypeId {
        let global_id = expression_id.into_global_any(ctx.module.id);
        if let Some(existing) = ctx.types.get_declared_type_id(global_id) {
            return existing;
        }

        let ty_id = ctx
            .types
            .insert_type_from(Type::Unevaluated(expression_id), expression_id);
        ctx.types.set_declared_type(global_id, ty_id);
        ty_id
    }

    /// Evaluate static arguments for a type reference.
    fn resolve_declared_template_literal_span_type(
        &self,
        ctx: &mut TypeContext<'_>,
        span_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_TEMPLATE);

        // resolve direct references to avoid caching template spans as unknown
        match ctx.tree.get(span_id) {
            TypeExpression::LocalReference {
                target_symbol,
                generic_arguments,
                ..
            }
            | TypeExpression::ModuleReference {
                target_symbol,
                generic_arguments,
                ..
            }
            | TypeExpression::GlobalReference {
                target_symbol,
                generic_arguments,
                ..
            } => {
                let ty = self.resolve_declared_template_span_reference(
                    &mut ctx.reborrow(),
                    span_id,
                    *target_symbol,
                    Some(generic_arguments.as_slice()),
                    validate_static_argument_bounds,
                )?;
                return Ok(ty);
            }
            TypeExpression::Reference {
                path,
                generic_arguments,
                space_order,
            } => {
                let resolved_symbol = self.resolve_template_literal_span_path(
                    &mut ctx.reborrow(),
                    span_id,
                    path,
                    *space_order,
                );
                if let Some(resolved_symbol) = resolved_symbol {
                    let ty = self.resolve_declared_template_span_reference(
                        &mut ctx.reborrow(),
                        span_id,
                        resolved_symbol,
                        Some(generic_arguments.as_slice()),
                        validate_static_argument_bounds,
                    )?;
                    return Ok(ty);
                }
            }
            _ => {}
        }

        // bypass declared caches to avoid collapsing span references to unknown
        let ty = self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            span_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            true,
            false,
            true,
        )?;

        Ok(ctx.types.insert_type_from(ty, span_id))
    }

    /// Resolve a template literal span to a reference type.
    fn resolve_declared_template_span_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        span_id: LocalNodeId<TypeExpression>,
        target_symbol: GlobalSymbolId,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        validate_static_argument_bounds: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // follow dependency items for local imports before canonicalization
        let mut target_symbol = target_symbol;
        if target_symbol.module_id == ctx.module.id {
            let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::DependencyItem
            {
                let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                if let DependencyItem::Local {
                    target_symbol: dependency_target,
                    ..
                }
                | DependencyItem::Remote {
                    target_symbol: dependency_target,
                    ..
                } = ctx.tree.get(item_id)
                {
                    target_symbol = *dependency_target;
                }
            }
        }

        // preserve alias identity while resolving the reference
        let target_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            target_symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let target_symbol = self.merged_type_symbol_id(ctx.module_symbol_view(), target_symbol);

        // reject value static parameters in template spans
        if self.symbol_is_static_parameter(ctx.symbol_type_view(), target_symbol) {
            let kind = self.generic_parameter_kind_for_symbol(&mut ctx.reborrow(), target_symbol);
            if kind == GenericParameterKind::Value {
                self.error(AnalyzeError::StaticParameterRequiresComptime {
                    node: span_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(ctx.types.insert_type_from(ty, span_id));
            }
        }

        // resolve generic arguments for the referenced span
        let generic_arguments =
            self.evaluate_generic_arguments(&mut ctx.reborrow(), generic_arguments)?;
        let resolved_arguments = self.resolve_declared_type_reference_static_arguments(
            &mut ctx.reborrow(),
            span_id.into_any(),
            target_symbol,
            generic_arguments.as_deref(),
            validate_static_argument_bounds,
        )?;

        let ty = Type::Reference {
            symbol: target_symbol,
            generic_arguments: resolved_arguments,
        };
        Ok(ctx.types.insert_type_from(ty, span_id))
    }

    /// Resolve a template literal span path to a symbol id.
    fn resolve_template_literal_span_path(
        &self,
        ctx: &mut TypeContext<'_>,
        span_id: LocalNodeId<TypeExpression>,
        path: &Path,
        space_order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        // only handle single segment identifiers
        if path.segments.len() != 1 {
            return None;
        }

        let key = StaticKey::Name(path.segments[0]);
        let (scope_id, scope, _mark) = ctx.symbols.get_scope(span_id, ctx.tree);
        let mut scope_cursor = Some((scope_id, scope));
        let mut nearest_non_preferred_symbol = None;
        let preferred_spaces = space_order.spaces();

        // walk scopes from inner to outer
        while let Some((_scope_id, scope)) = scope_cursor {
            let mut best_preferred = None;

            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().rev() {
                if *candidate_key != key {
                    continue;
                }
                let candidate = ctx.symbols.get_symbol(*candidate_symbol_id);
                if !candidate.is_active {
                    continue;
                }
                let candidate_symbol = candidate_symbol_id.into_global(ctx.module.id);

                // type-value symbols satisfy all space orders
                let candidate_space = candidate.space;
                if candidate_space == SymbolSpace::TypeValue {
                    return Some(candidate_symbol);
                }

                // track the best preferred-space candidate for this scope
                if let Some(space_index) = preferred_spaces
                    .iter()
                    .position(|preferred_space| *preferred_space == candidate_space)
                {
                    let replace_best = match best_preferred {
                        Some((best_index, _)) => space_index < best_index,
                        None => true,
                    };
                    if replace_best {
                        best_preferred = Some((space_index, candidate_symbol));
                    }
                    continue;
                }

                // keep the nearest non-preferred symbol only when no preferred symbol exists anywhere
                if nearest_non_preferred_symbol.is_none() {
                    nearest_non_preferred_symbol = Some(candidate_symbol);
                }
            }

            // prefer this scope's best matching symbol space before checking parent scopes
            if let Some((_, preferred_symbol)) = best_preferred {
                return Some(preferred_symbol);
            }

            scope_cursor = scope.parent.map(|(parent_id, _parent_mark)| {
                (parent_id, ctx.symbols.get_scope_by_id(parent_id))
            });
        }

        nearest_non_preferred_symbol
    }

    /// Resolve one declared type-index expression.
    fn resolve_declared_type_index_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        left: LocalNodeId<TypeExpression>,
        index: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Type> {
        // resolve the left side first
        let left_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            left,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;

        // disambiguate indexed access vs fixed-size array construction
        let interpretation = self.type_index_interpretation(&mut ctx.reborrow(), left_id, index)?;
        if interpretation == TypeIndexResolutionKind::ArraySized {
            return self.resolve_declared_array_sized_type(
                &mut ctx.reborrow(),
                left_id,
                index,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            );
        }

        // otherwise this stays a regular indexed-access type
        let index_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            index,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;

        // keep indexed access symbolic when the receiver or key still depends on free static or
        // infer state: this allows later substitution and constraint materialization to preserve
        // the authored index access shape
        let left_contains_static = self.type_contains_free_static_parameters(
            ctx.type_view(),
            left_id,
            &HashSet::new(),
            &mut HashSet::new(),
        );
        let left_contains_infer =
            self.type_contains_infer_vars(left_id, ctx.types, &mut HashSet::new());
        let left_is_diagnostic_concrete =
            self.declared_index_receiver_is_diagnostic_concrete(ctx.reborrow(), left_id);
        let index_contains_static = self.type_contains_free_static_parameters(
            ctx.type_view(),
            index_id,
            &HashSet::new(),
            &mut HashSet::new(),
        );
        let index_contains_infer =
            self.type_contains_infer_vars(index_id, ctx.types, &mut HashSet::new());
        if left_contains_static
            || left_contains_infer
            || !left_is_diagnostic_concrete
            || index_contains_static
            || index_contains_infer
        {
            return Ok(Type::Index {
                left: left_id,
                index: index_id,
            });
        }

        // diagnose concrete missing keys without collapsing authored index access
        let mut visited = Vec::new();
        let resolution = self.resolve_index_access_types(
            &mut ctx.reborrow(),
            index.into_any(),
            left_id,
            index_id,
            NormalizationMode::Flow,
            RelationMode::INDEX_ACCESS,
            &mut visited,
        );
        if let Some(missing_key) = resolution.missing_keys.first().copied() {
            self.report_missing_member_diagnostic(
                ctx.type_view(),
                index.into_any(),
                left_id,
                missing_key,
                true,
            )?;

            return Ok(Type::Index {
                left: left_id,
                index: index_id,
            });
        }

        Ok(Type::Index {
            left: left_id,
            index: index_id,
        })
    }

    /// Report one declared type diagnostic and poison the local type result.
    fn report_declared_type_error(&self, error: AnalyzeError) -> AnalyzeResult<Option<Type>> {
        self.error(error);
        Ok(Some(Type::Error))
    }

    /// Report one declared type diagnostic and poison the concrete type result.
    fn report_declared_concrete_type_error(&self, error: AnalyzeError) -> AnalyzeResult<Type> {
        self.error(error);
        Ok(Type::Error)
    }

    /// Resolve one fixed-size array type from a type-index expression.
    fn resolve_declared_array_sized_type(
        &self,
        ctx: &mut TypeContext<'_>,
        element_type_id: LocalTypeId,
        index: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Type> {
        // resolve direct integer literal sizes first
        let integer_array_size =
            self.evaluate_integer_static_literal(&mut ctx.reborrow(), index)?;
        if let Some(value) = integer_array_size {
            if value < 0 {
                return self.report_declared_concrete_type_error(AnalyzeError::InvalidArraySize {
                    node: index
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            self.set_integer_literal_type_expression(ctx.module.id, index, value, ctx.types);
            let count_type_id =
                self.array_sized_count_type_id_for_type_expression(ctx.module.id, index, ctx.types);

            return Ok(Type::ArraySized {
                element: element_type_id,
                count: count_type_id,
                is_readonly: false,
            });
        }

        // resolve symbolic static-parameter counts when present
        if self.type_expression_is_array_size_candidate(&mut ctx.reborrow(), index)? {
            let count_type_id = self
                .resolve_array_size_type_parameter(
                    &mut ctx.reborrow(),
                    index,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?
                .unwrap_or_else(|| {
                    self.array_sized_count_type_id_for_type_expression(
                        ctx.module.id,
                        index,
                        ctx.types,
                    )
                });

            return Ok(Type::ArraySized {
                element: element_type_id,
                count: count_type_id,
                is_readonly: false,
            });
        }

        // keep indexed access when the index is not a valid array-size value
        let index_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            index,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;

        Ok(Type::Index {
            left: element_type_id,
            index: index_id,
        })
    }

    fn resolve_declared_expression_type(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
    ) -> AnalyzeResult<Option<Type>> {
        let module_checks = ctx
            .compiler_context
            .module_check_options_for_module(ctx.module.id);
        let is_user_module = matches!(ctx.module.source, ModuleSource::User);
        let defer_reference_resolution = ctx.module.language_type.is_declaration()
            && (module_checks.skip_lib_check
                || matches!(ctx.module.source, ModuleSource::Builtin(_)));

        let expression = ctx.tree.get(expression_id).clone();
        let ty = match expression {
            TypeExpression::Missing | TypeExpression::Error => {
                return Ok(Some(Type::Error));
            }
            TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE);
                let member_key = StaticKey::Name(name);
                let Some(selection) = self.resolve_type_member_symbol(
                    &mut ctx.reborrow(),
                    expression_id,
                    left,
                    member_key,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?
                else {
                    if is_user_module {
                        // evaluate the receiver type for a precise missing member diagnostic
                        let receiver_ty_id = self.resolve_declared_type_expression(
                            &mut ctx.reborrow(),
                            left,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )?;

                        // report missing-member unless receiver has a primary blocker
                        let _ = self.report_missing_member_diagnostic(
                            ctx.type_view(),
                            expression_id.into_any(),
                            receiver_ty_id,
                            member_key,
                            true,
                        )?;
                        return Ok(Some(Type::Error));
                    }

                    return Ok(None);
                };
                let (target_symbol, receiver_symbol, receiver_arguments) = match selection {
                    TypeMemberResolution::Namespace { target_symbol } => {
                        (target_symbol, None, Vec::new())
                    }
                    TypeMemberResolution::Associated(AssociatedProjectionSelection {
                        target_symbol,
                        receiver_symbol,
                        receiver_arguments,
                    }) => (target_symbol, Some(receiver_symbol), receiver_arguments),
                };

                // require explicit arguments for generic associated type projections
                let has_explicit_static_arguments = !generic_arguments.is_empty();
                if !has_explicit_static_arguments
                    && self.associated_type_requires_static_arguments(
                        ctx.tree_symbol_view(),
                        target_symbol,
                    )?
                {
                    let node = expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::MissingStaticArgument { node });
                    return Ok(Some(Type::Error));
                }

                // evaluate static arguments for the referenced symbol
                let generic_arguments = self.evaluate_generic_arguments(
                    &mut ctx.reborrow(),
                    Some(generic_arguments.as_slice()),
                )?;
                let resolve_static_arguments =
                    resolve_static_arguments && !defer_reference_resolution;
                let validate_member_static_argument_bounds =
                    validate_static_argument_bounds && receiver_symbol.is_none();
                let member_ty = self.resolve_type_reference_type(
                    &mut ctx.reborrow(),
                    expression_id,
                    target_symbol,
                    generic_arguments.clone(),
                    resolve_static_arguments,
                    validate_member_static_argument_bounds,
                    enforce_implicit_managed,
                )?;

                self.materialize_associated_member_projection(
                    &mut ctx.reborrow(),
                    expression_id.into_any(),
                    target_symbol,
                    receiver_symbol,
                    &receiver_arguments,
                    generic_arguments.as_deref(),
                    member_ty,
                )?
            }
            TypeExpression::LocalReference {
                target_symbol,
                generic_arguments,
                path: _,
                ..
            }
            | TypeExpression::ModuleReference {
                target_symbol,
                generic_arguments,
                path: _,
                ..
            }
            | TypeExpression::GlobalReference {
                target_symbol,
                generic_arguments,
                path: _,
                ..
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE);
                let target_symbol = if let Some((parameter_symbol, _)) =
                    self.generic_parameter_type_reference(&mut ctx.reborrow(), expression_id)?
                {
                    parameter_symbol
                } else {
                    self.resolve_type_reference_symbol(ctx, target_symbol)
                };

                let generic_arguments = {
                    let _timing =
                        self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_ARGUMENTS);
                    self.evaluate_generic_arguments(
                        &mut ctx.reborrow(),
                        Some(generic_arguments.as_slice()),
                    )?
                };
                let resolve_static_arguments =
                    resolve_static_arguments && !defer_reference_resolution;
                self.resolve_type_reference_type(
                    &mut ctx.reborrow(),
                    expression_id,
                    target_symbol,
                    generic_arguments,
                    resolve_static_arguments,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?
            }
            TypeExpression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            TypeExpression::Intrinsic => {
                // semantic intrinsic alias
                let Some(value) = self.intrinsic_alias_literal(ctx, expression_id) else {
                    return self.report_declared_type_error(AnalyzeError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                };

                Type::TypeLiteral { value }
            }
            TypeExpression::Literal { value } => {
                // reject forbidden type literals in user code
                if is_user_module {
                    // disallow explicit any
                    if ctx.options.no_any && matches!(value, TypeLiteral::Any) {
                        return self.report_declared_type_error(AnalyzeError::AnyTypeDisabled {
                            node: expression_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }

                    // disallow explicit unknown
                    if ctx.options.no_unknown && matches!(value, TypeLiteral::Unknown) {
                        return self.report_declared_type_error(
                            AnalyzeError::UnknownTypeDisabled {
                                node: expression_id
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            },
                        );
                    }

                    // disallow imprecise primitives
                    if ctx.options.no_imprecise_primitives
                        && matches!(value, TypeLiteral::Primitive(PrimitiveType::Number))
                    {
                        return self.report_declared_type_error(
                            AnalyzeError::ImprecisePrimitiveDisabled {
                                node: expression_id
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            },
                        );
                    }
                }

                // map builtin iterator return to configured strictness
                if let TypeLiteral::Intrinsic(IntrinsicType::BuiltinIteratorReturn) = value {
                    let current_profile = self.profile(ctx.profile);
                    let mapped = if current_profile.key.flags.strict_builtin_iterator_return {
                        TypeLiteral::Undefined
                    } else {
                        TypeLiteral::Any
                    };
                    Type::TypeLiteral { value: mapped }
                } else {
                    Type::TypeLiteral {
                        value: value.clone(),
                    }
                }
            }
            TypeExpression::This => Type::This,
            TypeExpression::Parenthesized { expression } => {
                return self.resolve_declared_expression_type(
                    &mut ctx.reborrow(),
                    expression,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                    resolve_static_arguments,
                );
            }

            TypeExpression::Declaration {
                declaration: declaration_id,
            } => {
                let declaration = ctx.tree.get(declaration_id).clone();
                if let Declaration::Function(declaration) = declaration {
                    self.resolve_declared_function_signature_type(
                        &mut ctx.reborrow(),
                        &declaration.signature,
                        declaration_id.into_any(),
                        false,
                    )?
                } else {
                    // #Incomplete: only function declarations are evaluable as ctx.types (?)
                    return Ok(None);
                }
            }
            TypeExpression::FunctionTypeDeclaration(function) => {
                let signature = FunctionSignature {
                    is_abstract: false,
                    is_override: false,
                    asynchrony: Asynchrony::Sync,
                    cardinality: FunctionCardinality::Scalar,
                    mode: None,
                    kind: FunctionKind::Lambda,
                    generic_parameters: function.generic_parameters.clone(),
                    where_clauses: function.where_clauses.clone(),
                    this_parameter: function.this_parameter,
                    parameters: function.parameters.clone(),
                    return_type: function.return_type,
                };

                self.resolve_declared_function_signature_type(
                    &mut ctx.reborrow(),
                    &signature,
                    expression_id.into_any(),
                    false,
                )?
            }
            TypeExpression::ConstructorTypeDeclaration(function) => {
                let signature = FunctionSignature {
                    is_abstract: function.is_abstract,
                    is_override: false,
                    asynchrony: Asynchrony::Sync,
                    cardinality: FunctionCardinality::Scalar,
                    mode: Some(FunctionMode::New),
                    kind: FunctionKind::Lambda,
                    generic_parameters: function.generic_parameters.clone(),
                    where_clauses: function.where_clauses.clone(),
                    this_parameter: None,
                    parameters: function.parameters.clone(),
                    return_type: function.return_type,
                };

                self.resolve_declared_function_signature_type(
                    &mut ctx.reborrow(),
                    &signature,
                    expression_id.into_any(),
                    false,
                )?
            }

            TypeExpression::Readonly { target_type } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let right_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Readonly {
                    target_type: right_id,
                }
            }
            TypeExpression::KeyOf { target_type } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let right_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::KeyOf {
                    target_type: right_id,
                }
            }
            TypeExpression::TypeOfValue { value } => {
                return Ok(Some(self.resolve_typeof_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    value,
                )?));
            }
            TypeExpression::Must { target_type } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let right_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Must {
                    target_type: right_id,
                }
            }
            TypeExpression::AsComptime { target_type } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let right_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::AsComptime {
                    target_type: right_id,
                }
            }
            TypeExpression::Not { target_type } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let right_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Not {
                    target_type: right_id,
                }
            }
            TypeExpression::ValueOf {
                mutability,
                variance,
                target_type,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let type_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::ValueOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            TypeExpression::ReferenceOf {
                mutability,
                variance,
                target_type,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let type_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::ReferenceOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            TypeExpression::PointerOf {
                mutability,
                target_type,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let type_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    target_type,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::PointerOf {
                    mutability,
                    right: type_id,
                }
            }
            TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let left_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    left,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let right_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    extends_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let distributive_symbol =
                    self.conditional_left_distributive_symbol(ctx.symbol_type_view(), left_id);
                let should_validate_branches = !self.type_contains_static_parameters(
                    ctx.type_view(),
                    left_id,
                    &mut HashSet::new(),
                ) && validate_static_argument_bounds;
                let then_type_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    then_type,
                    should_validate_branches,
                    enforce_implicit_managed,
                )?;
                let else_type_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    else_type,
                    should_validate_branches,
                    enforce_implicit_managed,
                )?;
                Type::Conditional {
                    distributive_symbol,
                    left: left_id,
                    right: right_id,
                    then_type: then_type_id,
                    else_type: else_type_id,
                }
            }
            TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_MAPPED);

                let source_type = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    parameter.source_type,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                // cache the mapped parameter source type for later validation
                let parameter_symbol = parameter.symbol.into_global(ctx.module.id);
                ctx.types
                    .set_static_parameter_constraint_type(parameter_symbol, source_type);
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        key_remap,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let key_remap = match key_remap {
                    Some(Ok(key_remap)) => Some(key_remap),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                let value_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    value,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let readonly = match readonly {
                    TypeModifier::Present => MappedTypeModifier::Present,
                    TypeModifier::Add => MappedTypeModifier::Add,
                    TypeModifier::Remove => MappedTypeModifier::Remove,
                    TypeModifier::None => MappedTypeModifier::None,
                };
                let optional = match optional {
                    TypeModifier::Present => MappedTypeModifier::Present,
                    TypeModifier::Add => MappedTypeModifier::Add,
                    TypeModifier::Remove => MappedTypeModifier::Remove,
                    TypeModifier::None => MappedTypeModifier::None,
                };

                let parameter = destack_dir::MappedTypeParameter {
                    name: parameter.name,
                    symbol: parameter_symbol,
                    constraint: source_type,
                    key_remap,
                };
                Type::Mapped {
                    parameter,
                    modifiers: MappedTypeModifiers { readonly, optional },
                    value: value_id,
                }
            }
            TypeExpression::Index { left, index } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_INDEX);
                self.resolve_declared_type_index_expression(
                    &mut ctx.reborrow(),
                    left,
                    index,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?
            }
            TypeExpression::TemplateLiteral { strings, spans } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.resolve_declared_template_literal_span_type(
                            &mut ctx.reborrow(),
                            *span,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )
                    })
                    .collect::<AnalyzeResult<Vec<_>>>()?;
                Type::TemplateLiteral {
                    strings: strings.clone(),
                    spans,
                }
            }
            TypeExpression::Import {
                target,
                arguments: _,
                qualifier,
                generic_arguments,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let generic_arguments = self.evaluate_generic_arguments(
                    &mut ctx.reborrow(),
                    Some(generic_arguments.as_slice()),
                )?;
                if let Expression::ScalarLiteral {
                    value: ScalarLiteral::String(target),
                } = ctx.tree.get(target)
                {
                    Type::Import {
                        target: *target,
                        qualifier: qualifier.clone(),
                        generic_arguments,
                    }
                } else {
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    }
                }
            }
            TypeExpression::Infer { name, constraint } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let constraint = constraint.map(|constraint| {
                    self.resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        constraint,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let constraint = match constraint {
                    Some(Ok(constraint)) => Some(constraint),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                Type::Infer { name, constraint }
            }
            TypeExpression::Predicate {
                asserts,
                subject,
                target,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let target = target.map(|target| {
                    self.resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        target,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let target = match target {
                    Some(Ok(target)) => Some(target),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };

                // predicate subject
                let subject = match subject {
                    TypePredicateSubject::Identifier(name) => PredicateSubject::Unresolved(name),
                    TypePredicateSubject::This => PredicateSubject::This,
                };

                Type::Predicate {
                    asserts,
                    subject,
                    target,
                }
            }

            TypeExpression::Union { elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.resolve_declared_type_expression(
                            &mut ctx.reborrow(),
                            *element,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )
                    })
                    .collect::<AnalyzeResult<Vec<_>>>()?;

                Type::Union { elements }
            }
            TypeExpression::Intersection { elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL);
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.resolve_declared_type_expression(
                            &mut ctx.reborrow(),
                            *element,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )
                    })
                    .collect::<AnalyzeResult<Vec<_>>>()?;

                Type::Intersection { elements }
            }
            TypeExpression::Tuple { elements } | TypeExpression::ArrayTuple { elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL);
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element_expression = ctx.tree.get(element_id);
                    let (label, value_id, is_optional, is_readonly, is_rest) =
                        match element_expression {
                            TupleElement::Element {
                                label,
                                value,
                                is_optional,
                                is_readonly,
                            } => (*label, *value, *is_optional, *is_readonly, false),
                            TupleElement::Spread { label, value } => {
                                (*label, *value, false, false, true)
                            }
                            TupleElement::Error => {
                                continue;
                            }
                        };
                    let value_ty_id = self.resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        value_id,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    if matches!(
                        ctx.types.get_type(value_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Void
                        }
                    ) {
                        return self.report_declared_type_error(AnalyzeError::VoidInTuple {
                            node: value_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                    let mut element = TypeElement::new(value_ty_id);
                    element.label = label;
                    element.is_optional = is_optional;
                    element.is_readonly = is_readonly;
                    element.is_rest = is_rest;
                    element_types.push(element);
                }

                Type::Tuple {
                    elements: element_types,
                    is_readonly: false,
                }
            }
            TypeExpression::Array { element } => {
                let left_id = self.resolve_declared_type_expression(
                    &mut ctx.reborrow(),
                    element,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                if matches!(
                    ctx.types.get_type(left_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Void
                    }
                ) {
                    return self.report_declared_type_error(AnalyzeError::VoidInArray {
                        node: element
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                Type::Array {
                    element: Some(left_id),
                    is_readonly: false,
                }
            }
            TypeExpression::Object { members } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL);
                // evaluate object fields
                // #Cleanup: extract property -> type field evaluation?
                let mut fields = Vec::with_capacity(members.len());
                let mut call_signatures = Vec::new();
                let mut construct_signatures = Vec::new();
                let mut index_signatures = Vec::new();
                for member_id in members {
                    let member = ctx.tree.get(member_id).clone();
                    let field = match member {
                        TypeMember::Field {
                            is_static: _,
                            is_optional,
                            is_readonly,
                            key,
                            declared_type,
                            symbol: _,
                        } => {
                            let Some(key) = self.static_key_from_key(
                                ctx.compiler_context.revision(),
                                ctx.profile,
                                ctx.tree,
                                ctx.symbols,
                                ctx.types,
                                key,
                            ) else {
                                if ctx.module.language_type.is_declaration() {
                                    continue;
                                }
                                return self.report_declared_type_error(
                                    AnalyzeError::UnsupportedConstruct {
                                        node: member_id
                                            .into_global_any(ctx.module.id)
                                            .into_anchored(Some(ctx.profile)),
                                    },
                                );
                            };

                            let Some(declared_type) = declared_type else {
                                return self.report_declared_type_error(
                                    AnalyzeError::MissingType {
                                        node: member_id
                                            .into_global_any(ctx.module.id)
                                            .into_anchored(Some(ctx.profile)),
                                    },
                                );
                            };

                            let ty = self.resolve_declared_type_expression(
                                &mut ctx.reborrow(),
                                declared_type,
                                validate_static_argument_bounds,
                                enforce_implicit_managed,
                            )?;

                            TypeField {
                                key,
                                ty,
                                is_optional,
                                is_readonly,
                            }
                        }
                        TypeMember::CallSignature { signature, .. } => {
                            let signature = FunctionSignature {
                                is_abstract: false,
                                is_override: false,
                                asynchrony: Asynchrony::Sync,
                                cardinality: FunctionCardinality::Scalar,
                                mode: None,
                                kind: FunctionKind::Lambda,
                                generic_parameters: signature.generic_parameters.clone(),
                                where_clauses: signature.where_clauses.clone(),
                                this_parameter: signature.this_parameter,
                                parameters: signature.parameters.clone(),
                                return_type: signature.return_type,
                            };
                            let ty = self.resolve_declared_function_signature_type(
                                &mut ctx.reborrow(),
                                &signature,
                                member_id.into_any(),
                                false,
                            )?;
                            let ty_id = ctx.types.insert_type_from(ty, member_id);
                            call_signatures.push(ty_id);
                            continue;
                        }
                        TypeMember::ConstructSignature { signature, .. } => {
                            let signature = FunctionSignature {
                                is_abstract: signature.is_abstract,
                                is_override: false,
                                asynchrony: Asynchrony::Sync,
                                cardinality: FunctionCardinality::Scalar,
                                mode: Some(FunctionMode::New),
                                kind: FunctionKind::Lambda,
                                generic_parameters: signature.generic_parameters.clone(),
                                where_clauses: signature.where_clauses.clone(),
                                this_parameter: None,
                                parameters: signature.parameters.clone(),
                                return_type: signature.return_type,
                            };
                            let ty = self.resolve_declared_function_signature_type(
                                &mut ctx.reborrow(),
                                &signature,
                                member_id.into_any(),
                                false,
                            )?;
                            let ty_id = ctx.types.insert_type_from(ty, member_id);
                            construct_signatures.push(ty_id);
                            continue;
                        }
                        TypeMember::Method {
                            is_static: _,
                            is_optional,
                            key,
                            signature,
                            body: _,
                            symbol: _,
                        } => {
                            let Some(key) = self.static_key_from_key(
                                ctx.compiler_context.revision(),
                                ctx.profile,
                                ctx.tree,
                                ctx.symbols,
                                ctx.types,
                                key,
                            ) else {
                                if ctx.module.language_type.is_declaration() {
                                    continue;
                                }
                                return self.report_declared_type_error(
                                    AnalyzeError::UnsupportedConstruct {
                                        node: member_id
                                            .into_global_any(ctx.module.id)
                                            .into_anchored(Some(ctx.profile)),
                                    },
                                );
                            };

                            let ty = self.resolve_declared_function_signature_type(
                                &mut ctx.reborrow(),
                                &signature,
                                member_id.into_any(),
                                false,
                            )?;
                            let ty_id = ctx.types.insert_type_from(ty, member_id);

                            TypeField {
                                key,
                                ty: ty_id,
                                is_optional,
                                is_readonly: false,
                            }
                        }
                        TypeMember::IndexSignature {
                            is_optional,
                            is_readonly,
                            name,
                            key_type,
                            value_type,
                            symbol: _,
                        } => {
                            let key_type = self.resolve_declared_type_expression(
                                &mut ctx.reborrow(),
                                key_type,
                                validate_static_argument_bounds,
                                enforce_implicit_managed,
                            )?;
                            let value_type = self.resolve_declared_type_expression(
                                &mut ctx.reborrow(),
                                value_type,
                                validate_static_argument_bounds,
                                enforce_implicit_managed,
                            )?;
                            index_signatures.push(TypeIndexSignature {
                                name,
                                key_type,
                                value_type,
                                is_optional,
                                is_readonly,
                            });
                            continue;
                        }
                        TypeMember::Embed { .. }
                        | TypeMember::AssociatedType { .. }
                        | TypeMember::AssociatedConst { .. }
                        | TypeMember::Error { .. } => {
                            return self.report_declared_type_error(
                                AnalyzeError::UnsupportedConstruct {
                                    node: member_id
                                        .into_global_any(ctx.module.id)
                                        .into_anchored(Some(ctx.profile)),
                                },
                            );
                        }
                    };

                    fields.push(field);
                }

                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                }
            }

            _ => return Ok(None),
        };

        // enforce implicit managed restrictions for type expressions
        if is_user_module
            && enforce_implicit_managed
            && ctx.options.no_implicit_managed
            && !self.type_expression_has_explicit_ownership(ctx.tree, expression_id)
            && self.type_is_implicit_managed(ctx.module_type_view(), &ty)
        {
            return self.report_declared_type_error(AnalyzeError::ImplicitManagedTypeDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        Ok(Some(ty))
    }
}
