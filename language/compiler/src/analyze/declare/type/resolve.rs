use crate::analyze::common::{
    CanonicalSymbolMode, ModuleSymbolView, TreeSymbolView, TypeContext, TypeRewriteCache, TypeView,
};
use crate::analyze::declare::StaticConstantResolutionMode;
use crate::analyze::infer::RemoteValueTypeReadDomain;
use crate::analyze::{AssociatedProjectionSelection, StaticMemberSymbolKind};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DependencyItem, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree,
    NodeType, NormalizationMode, ScalarLiteral, StaticArgument, StaticExpression, StaticKey,
    StaticParameterKind, SymbolKind, SymbolSpace, Type, TypeExpression, TypeLiteral, TypeTable,
    are_types_equal,
};
use destack_source::{ModuleId, SourcePartKey};
use destack_workspace::Module;
use std::collections::HashSet;

/// Selected member target for type evaluation.
#[derive(Clone, Debug)]
pub(crate) enum TypeMemberResolution {
    /// A namespace import member.
    Namespace {
        /// The selected member symbol.
        target_symbol: GlobalSymbolId,
    },
    /// An associated projection member.
    Associated(AssociatedProjectionSelection),
}

/// Classification for `TypeIndex` disambiguation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeIndexResolutionKind {
    /// Interpret `T[K]` as indexed access.
    IndexAccess,
    /// Interpret `T[N]` as fixed-size array construction.
    ArraySized,
}

/// The receiver normalization result for type-index queries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeIndexReceiverState {
    /// A concrete receiver type id after alias, instance, and constraint resolution.
    Resolved(LocalTypeId),
    /// An unconstrained static type parameter receiver.
    UnconstrainedStaticParameter,
    /// A receiver that could not be resolved for index queries.
    Unresolved,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Publish one symbolic array-size reference result.
    fn publish_symbolic_array_size_type(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        candidate_expression_id: LocalNodeId<TypeExpression>,
        symbol: GlobalSymbolId,
        static_arguments: Option<Vec<StaticArgument>>,
    ) -> LocalTypeId {
        let reference_type = Type::Reference {
            symbol,
            static_arguments,
        };
        let reference_type_id = ctx
            .types
            .insert_type_from_any(reference_type, expression_id.into_any());
        ctx.types.set_inferred_type(
            expression_id.into_global_any(ctx.module.id),
            reference_type_id,
        );

        // keep the candidate node typed when we synthesize one symbolic array-size reference
        if expression_id != candidate_expression_id {
            ctx.types.set_inferred_type(
                candidate_expression_id.into_global_any(ctx.module.id),
                reference_type_id,
            );
        }

        reference_type_id
    }

    pub(crate) fn set_integer_literal_type_expression(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<TypeExpression>,
        value: i64,
        types: &mut TypeTable,
    ) {
        let literal_type = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
        };
        let literal_type_id = types.insert_type_from(literal_type, expression_id);
        types.set_inferred_type(expression_id.into_global_any(module_id), literal_type_id);
    }

    /// Resolve one array-size count type id for an expression.
    pub(crate) fn array_sized_count_type_id_for_type_expression(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<TypeExpression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if let Some(type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        {
            return types.unwrap_value_type_id(type_id);
        }

        let unknown_type_id = types.insert_type_from(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            expression_id,
        );
        types.set_inferred_type(expression_id.into_global_any(module_id), unknown_type_id);
        unknown_type_id
    }

    /// Resolve the inferred type for a static value parameter used as an array size.
    pub(crate) fn resolve_array_size_type_parameter(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip comptime size validation outside destack modules
        if !ctx.module.language_type.is_destack() {
            return Ok(None);
        }
        let (candidate_expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_type_expression(expression_id, ctx.tree);

        // skip expressions that are not static value references
        let kind = if let Some(kind) =
            self.static_parameter_type_reference_kind(&mut ctx.reborrow(), candidate_expression_id)?
        {
            kind
        } else {
            let index_ty_id = self.resolve_declared_type_expression(
                &mut ctx.reborrow(),
                candidate_expression_id,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?;
            let mut materialize_cache = TypeRewriteCache::new();
            let materialized_type_id = self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                index_ty_id,
                &mut materialize_cache,
            );
            let materialized_type_id = self.normalize_type(
                &mut ctx.reborrow(),
                materialized_type_id,
                NormalizationMode::Assign,
            );
            let symbol = self.unwrap_type_value_symbol(ctx.types, index_ty_id);
            let selection = self.associated_comptime_selection_from_type_expression(
                &mut ctx.reborrow(),
                candidate_expression_id,
            )?;
            let symbol = selection
                .as_ref()
                .map(|selection| selection.target_symbol)
                .or(symbol);
            let Some(symbol) = symbol else {
                if is_explicit_comptime {
                    self.error(AnalyzeError::InvalidComptimeExpression {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }
                return Ok(None);
            };
            let is_static_parameter =
                self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol);
            let is_associated_comptime = selection.is_some()
                || matches!(
                    self.query_static_member_symbol_kind_for_symbol(
                        ctx.tree_symbol_view(),
                        symbol,
                    )?,
                    Some(StaticMemberSymbolKind::AssociatedComptimeConst)
                );
            if !is_static_parameter && !is_associated_comptime {
                // explicit comptime aliases stay symbolic unless we already have one concrete result
                if is_explicit_comptime {
                    let mut inferred_id = ctx.types.unwrap_value_type_id(index_ty_id);
                    if matches!(
                        ctx.types.get_type(inferred_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        }
                    ) {
                        let generic_arguments =
                            ctx.tree.get(candidate_expression_id).generic_arguments();
                        let static_arguments = self
                            .evaluate_generic_arguments(&mut ctx.reborrow(), generic_arguments)?;
                        inferred_id = self.publish_symbolic_array_size_type(
                            &mut ctx.reborrow(),
                            expression_id,
                            candidate_expression_id,
                            symbol,
                            static_arguments,
                        );
                    }
                    return Ok(Some(inferred_id));
                }

                return Ok(None);
            }
            if is_associated_comptime {
                let receiver_has_static_parameters = selection.as_ref().is_some_and(|selection| {
                    self.receiver_projection_arguments_have_static_parameters(
                        ctx.type_view(),
                        &selection.receiver_arguments,
                    )
                });
                if !receiver_has_static_parameters
                    && self.array_size_materialized_type_is_concrete(
                        ctx.type_view(),
                        materialized_type_id,
                    )
                {
                    ctx.types.set_inferred_type(
                        expression_id.into_global_any(ctx.module.id),
                        materialized_type_id,
                    );
                    return Ok(Some(materialized_type_id));
                }

                // keep associated comptime projections symbolic, while preserving receiver args for substitution
                let receiver_arguments = selection
                    .as_ref()
                    .map(|selection| selection.receiver_arguments.clone())
                    .filter(|arguments| !arguments.is_empty());

                if is_explicit_comptime && symbol.module_id == ctx.module.id {
                    self.error(AnalyzeError::InvalidComptimeExpression {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                    return Ok(None);
                }

                let reference_type_id = self.publish_symbolic_array_size_type(
                    &mut ctx.reborrow(),
                    expression_id,
                    candidate_expression_id,
                    symbol,
                    receiver_arguments,
                );
                return Ok(Some(reference_type_id));
            }

            if is_explicit_comptime
                && self
                    .array_size_materialized_type_is_concrete(ctx.type_view(), materialized_type_id)
            {
                ctx.types.set_inferred_type(
                    expression_id.into_global_any(ctx.module.id),
                    materialized_type_id,
                );
                return Ok(Some(materialized_type_id));
            }

            self.static_parameter_kind_for_symbol(&mut ctx.reborrow(), symbol)
        };

        // require comptime for value usage
        if kind != StaticParameterKind::Value {
            self.error(AnalyzeError::StaticParameterRequiresComptime {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(None);
        }

        // resolve the parameter type and cache it as the inferred type
        let index_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            candidate_expression_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        let inferred_id = if let Type::Value { value } = ctx.types.get_type(index_id) {
            *value
        } else {
            index_id
        };
        ctx.types
            .set_inferred_type(expression_id.into_global_any(ctx.module.id), inferred_id);
        Ok(Some(inferred_id))
    }

    /// Return whether one materialized array-size type is concrete enough to keep.
    fn array_size_materialized_type_is_concrete(
        &self,
        view: TypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        !matches!(
            view.types.get_type(type_id),
            Type::Reference { .. }
                | Type::Unevaluated(_)
                | Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
                | Type::Error
        )
    }

    /// Check whether an expression can be used as an array size candidate.
    pub(crate) fn type_expression_is_array_size_candidate(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<bool> {
        let (expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_type_expression(expression_id, ctx.tree);
        if is_explicit_comptime {
            return Ok(true);
        }

        if self
            .associated_comptime_selection_from_type_expression(&mut ctx.reborrow(), expression_id)?
            .is_some()
        {
            return Ok(true);
        }

        let integer_literal =
            self.evaluate_integer_static_literal(&mut ctx.reborrow(), expression_id)?;
        if integer_literal.is_some() {
            return Ok(true);
        }

        let type_id =
            self.resolve_declared_type_expression(&mut ctx.reborrow(), expression_id, true, true)?;
        let type_id = ctx.types.unwrap_value_type_id(type_id);
        let index_value = StaticExpression::Type { ty: type_id };

        Ok(self.static_expression_may_be_numeric(&index_value, ctx.types))
    }

    /// Resolve an associated comptime member symbol from one projection expression.
    pub(crate) fn associated_comptime_selection_from_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<AssociatedProjectionSelection>> {
        let TypeExpression::Member { left, name, .. } = ctx.tree.get(expression_id) else {
            return Ok(None);
        };

        self.select_associated_projection_type_member_symbol(
            &mut ctx.reborrow(),
            expression_id,
            *left,
            StaticKey::Name(*name),
            Some(StaticMemberSymbolKind::AssociatedComptimeConst),
            true,
            true,
        )
    }

    /// Classify one `TypeIndex` expression as index-access or fixed-array construction.
    pub(crate) fn type_index_interpretation(
        &self,
        ctx: &mut TypeContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<TypeIndexResolutionKind> {
        let (_, is_explicit_comptime) =
            self.unwrap_as_comptime_type_expression(index_expression_id, ctx.tree);
        if ctx.module.language_type.is_declaration() {
            return Ok(if is_explicit_comptime {
                TypeIndexResolutionKind::ArraySized
            } else {
                TypeIndexResolutionKind::IndexAccess
            });
        }
        let index_is_array_size_candidate =
            self.type_expression_is_array_size_candidate(&mut ctx.reborrow(), index_expression_id)?;
        let left_is_array_sized =
            matches!(ctx.types.get_type(left_type_id), Type::ArraySized { .. });
        let supports_index_access =
            self.type_supports_index_access(&mut ctx.reborrow(), left_type_id, true)?;

        if !is_explicit_comptime
            && self.type_index_is_ambiguous_without_comptime(
                &mut ctx.reborrow(),
                left_type_id,
                index_expression_id,
            )?
        {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: index_expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                message: "ambiguous type index: use `as comptime` for array sizes or constrain the index for type access".to_string(),
            });
            return Ok(TypeIndexResolutionKind::IndexAccess);
        }

        let is_index_access = self.type_index_should_use_index_access(
            &mut ctx.reborrow(),
            left_type_id,
            is_explicit_comptime,
            supports_index_access,
            left_is_array_sized,
            index_is_array_size_candidate,
        )?;

        Ok(if is_index_access {
            TypeIndexResolutionKind::IndexAccess
        } else {
            TypeIndexResolutionKind::ArraySized
        })
    }

    /// Decide whether one type-index expression should preserve indexed-access semantics.
    pub(crate) fn type_index_should_use_index_access(
        &self,
        ctx: &mut TypeContext<'_>,
        left_type_id: LocalTypeId,
        is_explicit_comptime: bool,
        supports_index_access: bool,
        left_is_array_sized: bool,
        index_is_array_size_candidate: bool,
    ) -> AnalyzeResult<bool> {
        // receivers that are primitive literals cannot use indexed-access type semantics
        let is_primitive_literal_receiver = self.type_is_primitive_literal(left_type_id, ctx.types);
        if !supports_index_access || is_primitive_literal_receiver {
            return Ok(false);
        }

        // detect explicit fixed-array intent
        let force_array_from_explicit_comptime =
            is_explicit_comptime && index_is_array_size_candidate;
        let force_array_from_nested_sized = left_is_array_sized && index_is_array_size_candidate;

        // keep indexed-access semantics for admissible index receivers, even when a concrete key
        // is missing, so validation can report missing-member diagnostics in the index-access model
        let force_fixed_array = force_array_from_explicit_comptime || force_array_from_nested_sized;

        Ok(!force_fixed_array)
    }

    /// Resolve one type-index receiver through constraints, aliases, and instance targets.
    fn resolve_type_index_receiver_type(
        &self,
        ctx: &mut TypeContext<'_>,
        receiver_type_id: LocalTypeId,
    ) -> AnalyzeResult<TypeIndexReceiverState> {
        // track visited types to avoid loops in alias or constraint chains
        let mut visited = HashSet::new();
        let mut receiver_type_id = receiver_type_id;

        loop {
            // stop on cycles
            if !visited.insert(receiver_type_id) {
                return Ok(TypeIndexReceiverState::Unresolved);
            }

            match ctx.types.get_type(receiver_type_id).clone() {
                Type::Reference { symbol, .. } => {
                    // follow static parameter constraints when available
                    if self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
                        let source_id = ctx.types.get_type_source(receiver_type_id);
                        if let Some(constraint_type_id) = self.static_parameter_constraint_type(
                            &mut ctx.reborrow(),
                            symbol,
                            source_id,
                        ) {
                            receiver_type_id = constraint_type_id;
                            continue;
                        }

                        return Ok(TypeIndexReceiverState::UnconstrainedStaticParameter);
                    }

                    // follow alias targets when available
                    if let Some(alias_target_type_id) = ctx.types.get_alias_target_type_id(symbol) {
                        receiver_type_id = alias_target_type_id;
                        continue;
                    }

                    // follow instance types when available
                    if let Some(instance_type_id) = ctx.types.get_instance_type_id(symbol) {
                        receiver_type_id = instance_type_id;
                        continue;
                    }

                    return Ok(TypeIndexReceiverState::Unresolved);
                }
                Type::Unevaluated(_) => {
                    // evaluate before resolving index-query semantics
                    self.resolve_declared_type(&mut ctx.reborrow(), receiver_type_id)?;
                    continue;
                }
                _ => return Ok(TypeIndexReceiverState::Resolved(receiver_type_id)),
            }
        }
    }

    /// Check whether one type index is ambiguous without explicit comptime intent.
    pub(crate) fn type_index_is_ambiguous_without_comptime(
        &self,
        ctx: &mut TypeContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<bool> {
        let (candidate_expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_type_expression(index_expression_id, ctx.tree);
        if is_explicit_comptime {
            return Ok(false);
        }

        let Some(target_symbol) = ctx.tree.get(candidate_expression_id).target_symbol() else {
            return Ok(false);
        };

        // mapped key parameters are always type-space index operands
        if self.type_index_symbol_is_mapped_parameter(
            ctx.module,
            target_symbol,
            candidate_expression_id,
            ctx.tree,
        ) {
            return Ok(false);
        }

        if !self.symbol_is_static_parameter(ctx.symbol_type_view(), target_symbol) {
            return Ok(false);
        }
        let parameter_kind =
            if target_symbol.module_id == ctx.module.id && ctx.types.module_id == ctx.module.id {
                Some(self.static_parameter_kind_for_symbol(&mut ctx.reborrow(), target_symbol))
            } else {
                self.with_module_types_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    target_symbol.module_id,
                    ctx.types,
                    destack_artifact::ArtifactKey::dir_declared,
                    |_owner_module, owner_types| {
                        owner_types.query_artifact_static_parameter_kind(target_symbol)
                    },
                )
                .ok()
                .flatten()
            };
        let Some(parameter_kind) = parameter_kind else {
            return Ok(false);
        };
        if parameter_kind != StaticParameterKind::Type {
            return Ok(false);
        }

        let constraint_matches = self.type_index_constraint_matches_left_keyof(
            &mut ctx.reborrow(),
            target_symbol,
            left_type_id,
            candidate_expression_id.into_any(),
        );
        if constraint_matches {
            return Ok(false);
        }

        let index_type_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            candidate_expression_id,
            true,
            true,
        )?;
        let index_type_id = ctx.types.unwrap_value_type_id(index_type_id);
        let index_value = StaticExpression::Type { ty: index_type_id };
        if !self.static_expression_may_be_numeric(&index_value, ctx.types) {
            return Ok(false);
        }

        Ok(true)
    }

    /// Return whether one type-index symbol comes from an enclosing mapped parameter.
    fn type_index_symbol_is_mapped_parameter(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        expression_id: LocalNodeId<TypeExpression>,
        tree: &NodeTree,
    ) -> bool {
        // mapped-parameter membership is local syntax context
        if symbol.module_id != module.id {
            return false;
        }

        let mut cursor = Some(expression_id.into_any());
        while let Some(node_id) = cursor {
            if node_id.ty == NodeType::TypeExpression {
                let parent_expression_id = node_id.into_typed::<TypeExpression>();
                if let TypeExpression::Mapped { parameter, .. } = tree.get(parent_expression_id)
                    && parameter.symbol == symbol.local_id
                {
                    return true;
                }
            }

            cursor = tree.get_parent(node_id.id);
        }

        false
    }

    /// Check whether one type index parameter is constrained by `keyof` over the receiver type.
    pub(crate) fn type_index_constraint_matches_left_keyof(
        &self,
        ctx: &mut TypeContext<'_>,
        parameter_symbol: GlobalSymbolId,
        left_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> bool {
        let mut visited_symbols = HashSet::new();
        self.type_index_constraint_matches_left_keyof_inner(
            &mut ctx.reborrow(),
            parameter_symbol,
            left_type_id,
            source_id,
            &mut visited_symbols,
        )
    }

    /// Check whether one type index parameter constraint chain reaches `keyof` on the receiver type.
    pub(crate) fn type_index_constraint_matches_left_keyof_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        parameter_symbol: GlobalSymbolId,
        left_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        if !visited_symbols.insert(parameter_symbol) {
            return false;
        }

        let Some(constraint_id) =
            self.static_parameter_constraint_type(&mut ctx.reborrow(), parameter_symbol, source_id)
        else {
            return false;
        };

        let constraint_id = ctx.types.unwrap_value_type_id(constraint_id);
        let constraint_ty = ctx.types.get_type(constraint_id).clone();

        if let Type::Reference { symbol, .. } = constraint_ty
            && self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol)
        {
            return self.type_index_constraint_matches_left_keyof_inner(
                &mut ctx.reborrow(),
                symbol,
                left_type_id,
                source_id,
                visited_symbols,
            );
        }

        if let Some(left_symbol) =
            self.resolved_reference_symbol_for_type_id(&mut ctx.reborrow(), left_type_id)
            && self.type_constraint_matches_keyof_symbol(
                &mut ctx.reborrow(),
                constraint_id,
                left_symbol,
                visited_symbols,
            )
        {
            return true;
        }

        let Type::KeyOf { target_type: right } = constraint_ty else {
            return false;
        };

        let left_type_id = ctx.types.unwrap_value_type_id(left_type_id);
        let right_type_id = ctx.types.unwrap_value_type_id(right);

        // symbolic keyof constraints preserve index-space intent until substitutions are available
        if let Some(right_symbol) =
            self.resolved_reference_symbol_for_type_id(&mut ctx.reborrow(), right_type_id)
            && self.symbol_is_static_parameter(ctx.symbol_type_view(), right_symbol)
        {
            return true;
        }

        if left_type_id == right_type_id {
            return true;
        }

        let normalized_left =
            self.normalize_type(&mut ctx.reborrow(), left_type_id, NormalizationMode::Assign);
        let normalized_right = self.normalize_type(
            &mut ctx.reborrow(),
            right_type_id,
            NormalizationMode::Assign,
        );
        if normalized_left == normalized_right
            || are_types_equal(normalized_left, normalized_right, ctx.types)
        {
            return true;
        }

        let (
            Type::Reference {
                symbol: left_symbol,
                ..
            },
            Type::Reference {
                symbol: right_symbol,
                ..
            },
        ) = (
            ctx.types.get_type(normalized_left),
            ctx.types.get_type(normalized_right),
        )
        else {
            return false;
        };

        self.resolve_type_reference_symbol(ctx, *left_symbol)
            == self.resolve_type_reference_symbol(ctx, *right_symbol)
    }

    /// Resolve one canonical reference symbol from a type id when possible.
    fn resolved_reference_symbol_for_type_id(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        let mut current = ctx.types.unwrap_value_type_id(type_id);
        for _ in 0..8 {
            match ctx.types.get_type(current) {
                Type::Value { value } => current = *value,
                Type::Reference { symbol, .. } => {
                    return Some(self.resolve_type_reference_symbol(ctx, *symbol));
                }
                _ => return None,
            }
        }

        None
    }

    /// Return whether one static parameter has a declared `keyof` constraint for the receiver symbol.
    fn static_parameter_declared_keyof_matches_left_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        parameter_symbol: GlobalSymbolId,
        left_symbol: GlobalSymbolId,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        if !visited_symbols.insert(parameter_symbol) {
            return false;
        }

        if parameter_symbol.module_id != ctx.module.id {
            return false;
        }

        let symbol_entry = ctx.symbols.get_symbol(parameter_symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return false;
        };
        let Some(declared_type_id) = ctx.types.get_declared_type_id(primary_declaration) else {
            return false;
        };

        self.type_constraint_matches_keyof_symbol(
            &mut ctx.reborrow(),
            declared_type_id,
            left_symbol,
            visited_symbols,
        )
    }

    /// Return whether a declared constraint type resolves to `keyof left_symbol`.
    fn type_constraint_matches_keyof_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        constraint_type_id: LocalTypeId,
        left_symbol: GlobalSymbolId,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        let constraint_type_id = ctx.types.unwrap_value_type_id(constraint_type_id);
        match ctx.types.get_type(constraint_type_id).clone() {
            Type::Unevaluated(expression_id) => self
                .type_constraint_expression_matches_keyof_symbol(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_symbol,
                    visited_symbols,
                ),
            Type::KeyOf { target_type: right } => self
                .resolved_reference_symbol_for_type_id(&mut ctx.reborrow(), right)
                .is_some_and(|symbol| symbol == left_symbol),
            Type::Reference { symbol, .. } => {
                let symbol = self.resolve_type_reference_symbol(&ctx.reborrow(), symbol);
                if !self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
                    return false;
                }
                self.static_parameter_declared_keyof_matches_left_symbol(
                    &mut ctx.reborrow(),
                    symbol,
                    left_symbol,
                    visited_symbols,
                )
            }
            _ => {
                let source_id = ctx.types.get_type_source(constraint_type_id);
                if source_id.ty != NodeType::TypeExpression {
                    return false;
                }

                let expression_id = source_id.into_typed::<TypeExpression>();
                self.type_constraint_expression_matches_keyof_symbol(
                    &mut ctx.reborrow(),
                    expression_id,
                    left_symbol,
                    visited_symbols,
                )
            }
        }
    }

    /// Return whether one constraint expression resolves to `keyof left_symbol`.
    fn type_constraint_expression_matches_keyof_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        left_symbol: GlobalSymbolId,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        let expression_id = self.unwrap_parenthesized_type_expression(expression_id, ctx.tree);
        match ctx.tree.get(expression_id) {
            TypeExpression::KeyOf { target_type } => {
                let right = self.unwrap_parenthesized_type_expression(*target_type, ctx.tree);
                let Some(right_symbol) = ctx.tree.get(right).target_symbol() else {
                    return false;
                };
                self.resolve_type_reference_symbol(ctx, right_symbol) == left_symbol
            }
            TypeExpression::LocalReference { target_symbol, .. }
            | TypeExpression::ModuleReference { target_symbol, .. }
            | TypeExpression::GlobalReference { target_symbol, .. } => {
                if !self.symbol_is_static_parameter(ctx.symbol_type_view(), *target_symbol) {
                    return false;
                }

                self.static_parameter_declared_keyof_matches_left_symbol(
                    &mut ctx.reborrow(),
                    *target_symbol,
                    left_symbol,
                    visited_symbols,
                )
            }
            _ => false,
        }
    }

    /// Decide whether one `TypeIndex` expression should use index-access semantics.
    pub(crate) fn type_index_uses_index_access(
        &self,
        ctx: &mut TypeContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<bool> {
        Ok(self.type_index_interpretation(
            &mut ctx.reborrow(),
            left_type_id,
            index_expression_id,
        )? == TypeIndexResolutionKind::IndexAccess)
    }

    /// Unwrap parenthesized expressions and explicit `as comptime` markers.
    pub(crate) fn unwrap_as_comptime_type_expression(
        &self,
        expression_id: LocalNodeId<TypeExpression>,
        tree: &NodeTree,
    ) -> (LocalNodeId<TypeExpression>, bool) {
        let mut expression_id = self.unwrap_parenthesized_type_expression(expression_id, tree);
        let mut is_explicit_comptime = false;

        while let TypeExpression::AsComptime { target_type } = tree.get(expression_id) {
            is_explicit_comptime = true;
            expression_id = self.unwrap_parenthesized_type_expression(*target_type, tree);
        }

        (expression_id, is_explicit_comptime)
    }

    /// Check whether a type supports indexed access in a type expression.
    pub(crate) fn type_supports_index_access(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        treat_unknown_as_indexable: bool,
    ) -> AnalyzeResult<bool> {
        // normalize the receiver before checking index support
        let receiver_resolution =
            self.resolve_type_index_receiver_type(&mut ctx.reborrow(), type_id)?;
        let receiver_type_id = match receiver_resolution {
            TypeIndexReceiverState::Resolved(receiver_type_id) => receiver_type_id,
            TypeIndexReceiverState::UnconstrainedStaticParameter => {
                return Ok(treat_unknown_as_indexable);
            }
            TypeIndexReceiverState::Unresolved => return Ok(false),
        };

        // classify one normalized receiver shape
        let receiver_type = ctx.types.get_type(receiver_type_id).clone();
        match receiver_type {
            Type::Tuple { .. } | Type::Array { .. } | Type::ArraySized { .. } => Ok(true),
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => {
                // keep unknown or any indexability configurable per disambiguation context
                Ok(treat_unknown_as_indexable)
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => Ok(!fields.is_empty()
                || !call_signatures.is_empty()
                || !construct_signatures.is_empty()
                || !index_signatures.is_empty()),
            _ => Ok(false),
        }
    }

    /// Check whether a type resolves to a primitive literal.
    pub(crate) fn type_is_primitive_literal(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(_),
            } => true,
            Type::Reference { symbol, .. } => {
                if let Some(alias_id) = types.get_alias_target_type_id(*symbol) {
                    matches!(
                        types.get_type(alias_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(_),
                        }
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Resolve the static parameter symbol and kind for a reference expression.
    pub(crate) fn resolve_namespace_member_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        // require a reference expression on the left side
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = ctx.tree.get(left)
        else {
            return None;
        };

        // namespace imports are local dependency items
        if target_symbol.module_id != ctx.module.id {
            return None;
        }

        // require a dependency declaration
        let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::DependencyItem {
            return None;
        }

        // require a dependency item the import helpers understand
        let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
        let dependency = ctx.tree.get(dependency_id);
        self.resolve_imported_namespace_member_symbol(ctx, expression_id, dependency, member_key)
    }

    /// Resolve the static parameter symbol and kind for a reference type expression.
    pub(crate) fn resolve_namespace_type_member_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        left: LocalNodeId<TypeExpression>,
        member_key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        // require a reference expression on the left side
        let (TypeExpression::LocalReference { target_symbol, .. }
        | TypeExpression::ModuleReference { target_symbol, .. }
        | TypeExpression::GlobalReference { target_symbol, .. }) = ctx.tree.get(left)
        else {
            return None;
        };

        // namespace imports are local dependency items
        if target_symbol.module_id != ctx.module.id {
            return None;
        }

        // require a dependency declaration
        let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::DependencyItem {
            return None;
        }

        // require a dependency item the import helpers understand
        let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
        let dependency = ctx.tree.get(dependency_id);
        self.resolve_imported_namespace_member_symbol_in_type_expression(
            ctx,
            expression_id,
            dependency,
            member_key,
        )
    }

    /// Select a type member symbol for one member expression.
    pub(crate) fn resolve_type_member_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        left: LocalNodeId<TypeExpression>,
        member_key: StaticKey,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<TypeMemberResolution>> {
        // select namespace imports before projection lookups
        if let Some(namespace_symbol) = self.resolve_namespace_type_member_symbol(
            ctx.tree_symbol_view(),
            expression_id,
            left,
            member_key,
        ) {
            let source_id = ctx.tree.get_source(expression_id.id);
            let span_type = TypeExpression::member_source_part(ctx.tree, expression_id);
            ctx.types.set_symbol_target_for_source_part(
                SourcePartKey::new(source_id, span_type),
                namespace_symbol,
            );

            return Ok(Some(TypeMemberResolution::Namespace {
                target_symbol: namespace_symbol,
            }));
        }

        // select projected members through nominal receivers
        let Some(selection) = self.select_associated_projection_type_member_symbol(
            &mut ctx.reborrow(),
            expression_id,
            left,
            member_key,
            Some(StaticMemberSymbolKind::AssociatedType),
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?
        else {
            if let Some(enum_member_symbol) = self.resolve_enum_member_symbol(
                ctx,
                left,
                member_key,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )? {
                let source_id = ctx.tree.get_source(expression_id.id);
                let span_type = TypeExpression::member_source_part(ctx.tree, expression_id);
                ctx.types.set_symbol_target_for_source_part(
                    SourcePartKey::new(source_id, span_type),
                    enum_member_symbol,
                );

                return Ok(Some(TypeMemberResolution::Namespace {
                    target_symbol: enum_member_symbol,
                }));
            }
            if let Some(projected_symbol) = ctx.tree.get(expression_id).target_symbol() {
                let is_enum_field = self
                    .with_module_tree_symbol_view_or_local_for_artifact(
                        ctx.compiler_context,
                        ctx.module,
                        ctx.profile,
                        projected_symbol.module_id,
                        ctx.tree,
                        ctx.symbols,
                        destack_artifact::ArtifactKey::dir_declared,
                        |view| {
                            let symbol_entry = view.symbols.get_symbol(projected_symbol.local_id);
                            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                                return false;
                            };
                            primary_declaration.local_id.ty == NodeType::EnumField
                        },
                    )
                    .map_err(AnalyzeError::from)?;
                if is_enum_field {
                    let source_id = ctx.tree.get_source(expression_id.id);
                    let span_type = TypeExpression::member_source_part(ctx.tree, expression_id);
                    ctx.types.set_symbol_target_for_source_part(
                        SourcePartKey::new(source_id, span_type),
                        projected_symbol,
                    );

                    return Ok(Some(TypeMemberResolution::Namespace {
                        target_symbol: projected_symbol,
                    }));
                }
            }

            return Ok(None);
        };

        let selection_kind = self.query_static_member_symbol_kind_for_symbol(
            ctx.tree_symbol_view(),
            selection.target_symbol,
        )?;

        match selection_kind {
            Some(StaticMemberSymbolKind::AssociatedType)
            | Some(StaticMemberSymbolKind::AssociatedComptimeConst) => {
                let source_id = ctx.tree.get_source(expression_id.id);
                let span_type = TypeExpression::member_source_part(ctx.tree, expression_id);
                ctx.types.set_symbol_target_for_source_part(
                    SourcePartKey::new(source_id, span_type),
                    selection.target_symbol,
                );

                Ok(Some(TypeMemberResolution::Associated(selection)))
            }
            Some(StaticMemberSymbolKind::EnumField) => {
                let source_id = ctx.tree.get_source(expression_id.id);
                let span_type = TypeExpression::member_source_part(ctx.tree, expression_id);
                ctx.types.set_symbol_target_for_source_part(
                    SourcePartKey::new(source_id, span_type),
                    selection.target_symbol,
                );

                Ok(Some(TypeMemberResolution::Namespace {
                    target_symbol: selection.target_symbol,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Select one enum member symbol for a type-position member expression.
    pub(crate) fn resolve_enum_member_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        left: LocalNodeId<TypeExpression>,
        member_key: StaticKey,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let left_target_symbol = self
            .reference_symbol_for_type_expression(ctx.tree_symbol_view(), left)
            .or_else(|| ctx.tree.get(left).target_symbol())
            .map(|symbol| self.resolve_type_reference_symbol(ctx, symbol))
            .or_else(|| {
                let left_ty_id = self
                    .resolve_declared_type_expression(
                        &mut ctx.reborrow(),
                        left,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                    .ok()?;
                let left_ty = ctx.types.get_type(left_ty_id);
                self.enum_symbol_for_type(left_ty, ctx.types)
                    .map(|symbol| self.resolve_type_reference_symbol(ctx, symbol))
            });
        let Some(left_target_symbol) = left_target_symbol else {
            return Ok(None);
        };

        self.enum_field_symbol_for_member_key(
            ctx.tree_symbol_view(),
            left_target_symbol,
            &member_key,
        )
    }

    /// Evaluate an Expression into a Type with validation controls.
    pub(crate) fn resolve_type_reference_symbol(
        &self,
        ctx: &TypeContext<'_>,
        target_symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        // follow import dependency items before normalization
        let mut resolved_symbol = target_symbol;
        if resolved_symbol.module_id == ctx.module.id {
            let symbol_entry = ctx.symbols.get_symbol(resolved_symbol.local_id);
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
                    resolved_symbol = *dependency_target;
                }
            }
        }

        // keep simple local type-space symbols in fast path form
        if resolved_symbol.module_id == ctx.module.id {
            let symbol_entry = ctx.symbols.get_symbol(resolved_symbol.local_id);
            if symbol_entry.is_static_parameter() {
                return GlobalSymbolId::new(
                    ctx.module.id,
                    resolved_symbol.local_id.with_type(symbol_entry.ty),
                );
            }

            let is_simple = symbol_entry.target_symbol.is_none()
                && symbol_entry.canonical_symbol.is_none()
                && symbol_entry.merge_group.is_none()
                && symbol_entry.kind != SymbolKind::Namespace;
            let is_type_space = matches!(
                symbol_entry.space,
                SymbolSpace::Type | SymbolSpace::TypeValue
            );
            if is_simple && is_type_space {
                return GlobalSymbolId::new(
                    ctx.module.id,
                    resolved_symbol.local_id.with_type(symbol_entry.ty),
                );
            }
        }

        // normalize and canonicalize for all non-fast-path cases
        let normalized =
            self.normalize_reference_symbol_id(ctx.module_symbol_view(), resolved_symbol);
        let canonical = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            normalized,
            CanonicalSymbolMode::PreserveAliases,
        );

        self.merged_type_symbol_id(ctx.module_symbol_view(), canonical)
    }

    /// Evaluate a reference to a nominal symbol into a Type.
    pub(crate) fn resolve_type_reference_type(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<Vec<StaticArgument>>,
        resolve_static_arguments: bool,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Type> {
        // check cached reference types first
        let reference_cache_key = self.type_reference_cache_key(
            target_symbol,
            static_arguments.as_deref(),
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        );
        let cached_reference = reference_cache_key
            .and_then(|cache_key| ctx.types.get_type_reference_cache(cache_key).cloned());
        if let Some(cached) = cached_reference {
            return Ok(cached);
        }

        // defer static argument resolution when requested
        if !resolve_static_arguments {
            let ty = Type::Reference {
                symbol: target_symbol,
                static_arguments,
            };
            self.cache_type_reference_maybe(reference_cache_key, &ty, ctx.types);
            return Ok(ty);
        }

        // resolve static arguments against declared bounds
        let has_explicit_arguments = static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty());
        let parameter_symbols =
            self.collect_static_parameter_symbols(ctx.type_view(), target_symbol);
        let parameters_known = parameter_symbols.is_some();
        let has_parameters = parameter_symbols.is_some_and(|parameters| !parameters.is_empty());
        let resolved_arguments = if !has_explicit_arguments && parameters_known && !has_parameters {
            None
        } else {
            let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_ARGUMENTS);
            self.resolve_declared_type_reference_static_arguments(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                target_symbol,
                static_arguments.as_deref(),
                validate_static_argument_bounds,
            )?
        };
        let static_arguments = resolved_arguments.or(static_arguments);

        // return errors directly when static arguments failed to resolve
        let has_error_argument = static_arguments.as_deref().is_some_and(|arguments| {
            arguments.iter().any(|argument| match argument {
                StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } => ctx.types.get_type(*ty).is_error(),
                _ => false,
            })
        });
        if has_error_argument {
            return Ok(Type::Error);
        }

        // fold value-space constant references used in type positions
        let symbol_space = self
            .query_symbol_space_for_reference_if_declared(ctx.module_symbol_view(), target_symbol);
        if symbol_space == Some(SymbolSpace::Value) {
            let member_kind = self.query_static_member_symbol_kind_for_symbol(
                ctx.tree_symbol_view(),
                target_symbol,
            )?;
            if matches!(
                member_kind,
                Some(StaticMemberSymbolKind::AssociatedComptimeConst)
            ) {
                let ty = Type::Reference {
                    symbol: target_symbol,
                    static_arguments,
                };
                self.cache_type_reference_maybe(reference_cache_key, &ty, ctx.types);
                return Ok(ty);
            }

            let mut visited = HashSet::new();
            if let Some(static_value) = self.resolve_static_constant_reference_for_mode(
                &mut ctx.reborrow(),
                target_symbol,
                None,
                &mut visited,
                StaticConstantResolutionMode::InstantiatedDeclare,
            )? {
                let constant_type = match static_value {
                    StaticExpression::ScalarLiteral { value } => Some(Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    }),
                    StaticExpression::TypeLiteral { value } => Some(Type::TypeLiteral { value }),
                    StaticExpression::Type { ty } => Some(ctx.types.get_type(ty).clone()),
                    _ => None,
                };

                if let Some(constant_type) = constant_type {
                    self.cache_type_reference_maybe(reference_cache_key, &constant_type, ctx.types);
                    return Ok(constant_type);
                }
            }

            return Ok(Type::Unevaluated(expression_id));
        }

        // normalize well known references into canonical structural types
        let normalized =
            if let Some(well_known) = self.well_known_array_kind(ctx.profile, target_symbol) {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_WELL_KNOWN);
                self.normalize_well_known_type_reference(
                    &mut ctx.reborrow(),
                    expression_id.into_any(),
                    well_known,
                    static_arguments.as_deref(),
                )
            } else {
                None
            };
        if let Some(normalized) = normalized {
            self.cache_type_reference_maybe(reference_cache_key, &normalized, ctx.types);
            return Ok(normalized);
        }

        // fall back to a nominal reference
        let ty = Type::Reference {
            symbol: target_symbol,
            static_arguments,
        };
        self.cache_type_reference_maybe(reference_cache_key, &ty, ctx.types);
        Ok(ty)
    }

    /// Resolve symbol space for one type-reference target when declare commitments are available.
    pub(crate) fn query_symbol_space_for_reference_if_declared(
        &self,
        view: ModuleSymbolView<'_>,
        target_symbol: GlobalSymbolId,
    ) -> Option<SymbolSpace> {
        self.with_module_symbols_or_local_for_artifact(
            view.compiler_context,
            view.module,
            view.profile,
            target_symbol.module_id,
            view.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |_owner_module, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(target_symbol.local_id);
                Some(symbol_entry.space)
            },
        )
        .ok()
        .flatten()
    }

    /// Evaluate a typeof type expression into a Type.
    pub(crate) fn resolve_typeof_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        right_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Type> {
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_TYPEOF);

        // unwrap parenthesized targets
        let mut target_id = right_id;
        loop {
            let Expression::Parenthesized { expression } = ctx.tree.get(target_id) else {
                break;
            };
            target_id = *expression;
        }

        // resolve the target symbol for a typeof reference
        let Some(target_symbol) = self
            .reference_symbol_for_expression(ctx.tree_symbol_view(), target_id)
            .or_else(|| ctx.tree.get(target_id).target_symbol())
        else {
            return Ok(Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            });
        };

        // resolve the value type for the target symbol
        if let Some(value_ty_id) = ctx.types.get_value_type_id(target_symbol) {
            return Ok(ctx.types.get_type(value_ty_id).clone());
        }

        if target_symbol.module_id != ctx.module.id {
            let value_ty_id = self.resolve_remote_symbol_value_type(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                target_symbol,
                RemoteValueTypeReadDomain::Surface,
            )?;
            return Ok(ctx.types.get_type(value_ty_id).clone());
        }

        // fall back to local declaration types when available
        if let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(ctx.tree_symbol_view(), target_symbol)
        {
            let declared_ty_id = ctx
                .types
                .get_declared_type_id(declarator_id.into_global_any(ctx.module.id));
            if let Some(declared_ty_id) = declared_ty_id {
                self.resolve_declared_type(&mut ctx.reborrow(), declared_ty_id)?;
                return Ok(ctx.types.get_type(declared_ty_id).clone());
            }

            let declarator = ctx.tree.get(declarator_id);
            if let Some(value_id) = declarator.value {
                if let Expression::Type { value, .. } = ctx.tree.get(value_id) {
                    let ty = self.resolve_declared_type_expression_value(
                        &mut ctx.reborrow(),
                        *value,
                        true,
                        true,
                        true,
                        true,
                        true,
                    )?;
                    if !matches!(ty, Type::Unevaluated(_)) {
                        return Ok(ty);
                    }
                }
            }
        }

        Ok(Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        })
    }
}
