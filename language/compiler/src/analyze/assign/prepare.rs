use super::*;
use crate::AnalyzeResult;
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::TypeContext;
use crate::analyze::declare::StaticConstantResolutionMode;
use destack_dir::{LocalNodeId, TypeExpression, are_types_equal};
use destack_source::ModuleId;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve one declared type using the owner module syntax and symbols.
    fn resolve_declared_type_in_owner(
        &self,
        ctx: &mut TypeContext<'_>,
        module_id: ModuleId,
        type_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        // resolved types do not need any more work
        if !matches!(ctx.types.get_type(type_id), Type::Unevaluated(_)) {
            return Ok(());
        }

        // local declared types can resolve directly
        if module_id == ctx.module.id && ctx.types.module_id == ctx.module.id {
            self.resolve_declared_type_or_report(&mut ctx.reborrow(), type_id);
            return Ok(());
        }

        // remote declared types reuse local storage with owner syntax and symbols
        let dir = self
            .require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        let module = ctx.compiler_context.module(module_id);
        let options = ctx
            .compiler_context
            .analyze_context_options_for_module(module.id);
        let mut owner_ctx = TypeContext::new(
            ctx.compiler_context,
            module.as_ref(),
            ctx.profile,
            &options,
            &dir.tree,
            &dir.symbols,
            ctx.types,
            ctx.index.clone(),
        );

        self.resolve_declared_type_or_report(&mut owner_ctx, type_id);
        Ok(())
    }

    /// Follow cached alias instances for assignability comparisons.
    pub(super) fn unwrap_assignability_alias_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> LocalTypeId {
        let mut visited: Vec<GlobalSymbolId> = Vec::new();
        let mut current_id = type_id;

        loop {
            let Type::Reference {
                symbol,
                generic_arguments,
            } = types.get_type(current_id)
            else {
                break;
            };

            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }
            // avoid unwrapping alias instances with explicit static arguments
            if generic_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
            {
                break;
            }

            if visited.contains(symbol) {
                break;
            }
            visited.push(*symbol);

            let Some(instance_ty_id) = types.get_instance_type_id(*symbol) else {
                break;
            };
            current_id = instance_ty_id;
        }

        current_id
    }

    /// Normalize a type id for assignability checks.
    pub(super) fn prepare_assignability_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // unwrap cached alias instances first
        let type_id = self.unwrap_assignability_alias_type(type_id, ctx.types);

        // expand alias references that carry static arguments
        let type_id = self.expand_assignability_alias_reference(&mut ctx.reborrow(), type_id);

        // substitute static parameter references with constraints
        let type_id = self.resolve_assignability_static_constraint(&mut ctx.reborrow(), type_id);

        // preserve newtype references as nominal assignability boundaries
        if let Some(symbol) = self.unwrap_type_value_symbol(ctx.types, type_id) {
            let symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            );
            if symbol.ty() == SymbolType::Newtype
                || self.symbol_is_nominal_interface(ctx.tree_symbol_view(), symbol)
            {
                return type_id;
            }
        }

        // normalize the prepared type once so downstream checks see a stable shape
        let normalized =
            self.normalize_type(&mut ctx.reborrow(), type_id, NormalizationMode::Assign);
        if self.assignability_normalization_loses_count_context(type_id, normalized, ctx.types) {
            return type_id;
        }
        normalized
    }

    /// Return true when assignability normalization drops count projection information.
    fn assignability_normalization_loses_count_context(
        &self,
        before_id: LocalTypeId,
        after_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let before_type = types.get_type(before_id);
        let after_type = types.get_type(after_id);

        match (before_type, after_type) {
            (
                Type::ArraySized {
                    count: before_count,
                    ..
                },
                Type::ArraySized {
                    count: after_count, ..
                },
            ) => {
                matches!(
                    types.get_type(*before_count),
                    Type::Reference {
                        generic_arguments: Some(arguments),
                        ..
                    } if !arguments.is_empty()
                ) && matches!(types.get_type(*after_count), Type::Unevaluated(_))
            }
            (
                Type::Index {
                    index: before_index,
                    ..
                },
                Type::Index {
                    index: after_index, ..
                },
            ) => {
                matches!(
                    types.get_type(*before_index),
                    Type::Reference {
                        generic_arguments: Some(arguments),
                        ..
                    } if !arguments.is_empty()
                ) && matches!(
                    types.get_type(*after_index),
                    Type::Unevaluated(_)
                        | Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        }
                )
            }
            (
                Type::Index {
                    index: before_index,
                    ..
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
            ) => matches!(
                types.get_type(*before_index),
                Type::Reference {
                    generic_arguments: Some(arguments),
                    ..
                } if !arguments.is_empty()
            ),
            _ => false,
        }
    }

    /// Resolve static parameter references to their constraints for assignability.
    pub(super) fn resolve_assignability_static_constraint(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        let Type::Reference { symbol, .. } = ctx.types.get_type(type_id) else {
            return type_id;
        };
        let symbol = *symbol;

        // only substitute actual static parameters
        if !self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
            return type_id;
        }

        // resolve the declared constraint type
        let source_id = ctx.types.get_type_source(type_id);
        let constraint_id =
            self.generic_parameter_constraint_type(&mut ctx.reborrow(), symbol, source_id);
        let Some(constraint_id) = constraint_id else {
            return type_id;
        };

        // avoid redundant substitutions
        if constraint_id == type_id {
            return type_id;
        }

        constraint_id
    }

    /// Check whether two fixed array counts match.
    pub(super) fn array_sized_counts_match(
        &self,
        ctx: &mut AssignContext<'_>,
        target_count: LocalTypeId,
        source_count: LocalTypeId,
    ) -> AnalyzeResult<bool> {
        if target_count == source_count {
            return Ok(true);
        }

        let target_count =
            self.unwrapped_value_without_as_comptime_type_id(target_count, ctx.types);
        let source_count =
            self.unwrapped_value_without_as_comptime_type_id(source_count, ctx.types);
        if target_count == source_count || are_types_equal(target_count, source_count, ctx.types) {
            return Ok(true);
        }

        let target_value =
            self.query_array_sized_count_literal_value(&mut ctx.reborrow(), target_count)?;
        let source_value =
            self.query_array_sized_count_literal_value(&mut ctx.reborrow(), source_count)?;
        let (Some(target_value), Some(source_value)) = (target_value, source_value) else {
            return Ok(false);
        };

        Ok(target_value == source_value)
    }

    /// Check whether a fixed array count matches a literal length.
    pub(super) fn array_sized_count_matches_length(
        &self,
        ctx: &mut AssignContext<'_>,
        count: LocalTypeId,
        length: usize,
    ) -> AnalyzeResult<bool> {
        let value = self.query_array_sized_count_literal_value(&mut ctx.reborrow(), count)?;
        let Some(value) = value else {
            return Ok(false);
        };

        Ok(value == length as i64)
    }

    /// Extract an integer literal value for one fixed-array count type.
    fn query_array_sized_count_literal_value(
        &self,
        ctx: &mut AssignContext<'_>,
        count: LocalTypeId,
    ) -> AnalyzeResult<Option<i64>> {
        let count_ty_id = self.unwrapped_value_without_as_comptime_type_id(count, ctx.types);

        // fast path: integer literals already carry the count
        if let Some(value) = self.integer_literal_value_for_type_id(count_ty_id, ctx.types) {
            return Ok(Some(value));
        }

        // evaluate local unevaluated counts before looking through references
        if let Type::Unevaluated(expression_id) = ctx.types.get_type(count_ty_id).clone() {
            return self.query_array_sized_count_from_unevaluated(
                &mut ctx.reborrow(),
                count_ty_id,
                expression_id,
            );
        }

        // normalize once before handling references
        let normalized_count = self.normalize_type(
            &mut ctx.type_context_reborrow(),
            count_ty_id,
            NormalizationMode::Assign,
        );
        let normalized_count =
            self.unwrapped_value_without_as_comptime_type_id(normalized_count, ctx.types);
        if normalized_count != count_ty_id {
            let preserves_reference_context =
                matches!(
                    ctx.types.get_type(count_ty_id),
                    Type::Reference {
                        symbol,
                        generic_arguments: Some(arguments),
                    } if symbol.ty() == SymbolType::TypeAlias && !arguments.is_empty()
                ) && matches!(ctx.types.get_type(normalized_count), Type::Unevaluated(_));
            if preserves_reference_context {
                // keep alias references when normalization erases argument context
            } else {
                return self
                    .query_array_sized_count_literal_value(&mut ctx.reborrow(), normalized_count);
            }
        }
        let Type::Reference {
            symbol,
            generic_arguments,
        } = ctx.types.get_type(count_ty_id).clone()
        else {
            return Ok(None);
        };
        let source_id = ctx.types.get_type_source(count_ty_id);
        let symbol = self
            .with_module_symbols_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |owner_module, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                    GlobalSymbolId::new(owner_module.id, symbol.local_id.with_type(symbol_entry.ty))
                },
            )
            .map_err(AnalyzeError::from)?;

        // alias counts need instantiation before integer extraction
        if symbol.ty() == SymbolType::TypeAlias {
            return self.query_array_sized_count_from_alias_reference(
                &mut ctx.reborrow(),
                count_ty_id,
                source_id,
                symbol,
                generic_arguments.as_ref(),
            );
        }

        self.query_array_sized_count_from_static_reference(
            &mut ctx.reborrow(),
            count_ty_id,
            source_id,
            symbol,
            generic_arguments.as_ref(),
        )
    }

    /// Query one array-size count from one unevaluated type.
    fn query_array_sized_count_from_unevaluated(
        &self,
        ctx: &mut AssignContext<'_>,
        count_ty_id: LocalTypeId,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<i64>> {
        let node_id = expression_id.into_global_any(ctx.module.id);

        // follow cached declared or inferred results first
        if let Some(cached_type_id) = ctx.types.get_declared_or_inferred_type_id(node_id) {
            let cached_type_id =
                self.unwrapped_value_without_as_comptime_type_id(cached_type_id, ctx.types);
            if cached_type_id != count_ty_id {
                return self
                    .query_array_sized_count_literal_value(&mut ctx.reborrow(), cached_type_id);
            }
        }

        // otherwise resolve the local type expression directly
        if !ctx.tree.has_node_id(expression_id.id) {
            return Ok(None);
        }
        let value_type_id = self.resolve_declared_type_expression(
            &mut ctx.type_context_reborrow(),
            expression_id,
            true,
            true,
        )?;
        let value_type_id =
            self.unwrapped_value_without_as_comptime_type_id(value_type_id, ctx.types);
        if value_type_id == count_ty_id {
            return Ok(None);
        }

        self.query_array_sized_count_literal_value(&mut ctx.reborrow(), value_type_id)
    }

    /// Query one array-size count through one alias reference.
    fn query_array_sized_count_from_alias_reference(
        &self,
        ctx: &mut AssignContext<'_>,
        count_ty_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        generic_arguments: Option<&Vec<StaticArgument>>,
    ) -> AnalyzeResult<Option<i64>> {
        // expand one already-instantiated alias when possible
        let expanded = self
            .expand_assignability_alias_reference(&mut ctx.type_context_reborrow(), count_ty_id);
        let expanded = self.unwrapped_value_without_as_comptime_type_id(expanded, ctx.types);
        if expanded != count_ty_id && !matches!(ctx.types.get_type(expanded), Type::Unevaluated(_))
        {
            return self.query_array_sized_count_literal_value(&mut ctx.reborrow(), expanded);
        }

        // resolve the alias target in the owner module
        let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
            &mut ctx.type_context_reborrow(),
            symbol,
            source_id,
        ) else {
            return Ok(None);
        };
        self.resolve_declared_type_in_owner(
            &mut ctx.type_context_reborrow(),
            symbol.module_id,
            alias_target_id,
        )?;

        // build substitutions from the resolved static arguments
        let resolved_arguments = if let Some(arguments) = generic_arguments {
            self.resolve_type_reference_static_arguments(
                &mut ctx.type_context_reborrow(),
                source_id,
                symbol,
                Some(arguments.as_slice()),
                false,
            )?
        } else {
            None
        };
        let arguments = resolved_arguments
            .as_deref()
            .or(generic_arguments.map(Vec::as_slice));
        let substitutions = if let Some(arguments) = arguments {
            self.build_type_parameter_substitutions_for_symbol(
                &mut ctx.type_context_reborrow(),
                symbol,
                source_id,
                arguments,
            )
        } else {
            HashMap::new()
        };

        // instantiate the alias target under the current substitutions
        let mut instantiated_target_id = alias_target_id;
        if !substitutions.is_empty() {
            instantiated_target_id = self.apply_associated_projection_substitutions(
                &mut ctx.type_context_reborrow(),
                symbol,
                instantiated_target_id,
                &substitutions,
            )?;
        }
        let mut materialize_cache = HashMap::new();
        let mut substitute_cache = HashMap::new();
        let instantiated = self.instantiate_type_with_substitutions(
            &mut ctx.type_context_reborrow(),
            source_id,
            Some(symbol),
            instantiated_target_id,
            &substitutions,
            &mut materialize_cache,
            &mut substitute_cache,
        );
        let instantiated =
            self.unwrapped_value_without_as_comptime_type_id(instantiated, ctx.types);

        if instantiated == count_ty_id {
            return Ok(None);
        }

        self.query_array_sized_count_literal_value(&mut ctx.reborrow(), instantiated)
    }

    /// Query one array-size count through one non-alias static reference.
    fn query_array_sized_count_from_static_reference(
        &self,
        ctx: &mut AssignContext<'_>,
        count_ty_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        generic_arguments: Option<&Vec<StaticArgument>>,
    ) -> AnalyzeResult<Option<i64>> {
        // collect substitutions from the reference surface
        let member_kind = {
            let type_ctx = ctx.type_context_reborrow();
            self.query_static_member_symbol_kind_for_symbol(type_ctx.tree_symbol_view(), symbol)?
        };
        let mut substitutions = HashMap::new();
        if member_kind == Some(StaticMemberSymbolKind::AssociatedComptimeConst) {
            match self.query_owner_symbol_for_member_symbol(ctx.module_symbol_view(), symbol) {
                Ok(Some(receiver_symbol)) => {
                    let receiver_arguments = generic_arguments.cloned().unwrap_or_default();
                    let count_ty = ctx.types.get_type(count_ty_id).clone();
                    if !receiver_arguments.is_empty() {
                        let environment = self.projection_environment_for_member(
                            &mut ctx.type_context_reborrow(),
                            source_id,
                            symbol,
                            Some(receiver_symbol),
                            receiver_arguments.as_slice(),
                            None,
                            Some(&count_ty),
                            None,
                        )?;
                        substitutions.extend(environment.substitutions);
                    }
                }
                Ok(None) => {}
                Err(error) => return Err(error),
            }
        } else {
            let resolved_arguments = if let Some(arguments) = generic_arguments {
                self.resolve_type_reference_static_arguments(
                    &mut ctx.type_context_reborrow(),
                    source_id,
                    symbol,
                    Some(arguments.as_slice()),
                    false,
                )?
            } else {
                None
            };
            let arguments = resolved_arguments
                .as_deref()
                .or(generic_arguments.map(Vec::as_slice));
            if let Some(arguments) = arguments.filter(|arguments| !arguments.is_empty()) {
                substitutions.extend(self.build_type_parameter_substitutions_for_symbol(
                    &mut ctx.type_context_reborrow(),
                    symbol,
                    source_id,
                    arguments,
                ));
            }
        }

        // resolve the referenced static constant with the right instantiation mode
        let substitutions = if substitutions.is_empty() {
            None
        } else {
            Some(substitutions)
        };
        let resolution_mode =
            if member_kind == Some(StaticMemberSymbolKind::AssociatedComptimeConst) {
                StaticConstantResolutionMode::InstantiatedDeclare
            } else if substitutions.is_some() {
                StaticConstantResolutionMode::InstantiatedInfer
            } else {
                StaticConstantResolutionMode::Parametric
            };
        let mut visited_symbols = HashSet::new();
        let Some(static_value) = self.resolve_static_constant_reference_for_mode(
            &mut ctx.type_context_reborrow(),
            symbol,
            substitutions.as_ref(),
            &mut visited_symbols,
            resolution_mode,
        )?
        else {
            return Ok(None);
        };

        // evaluate local deferred static expressions before integer conversion
        let static_value = if let StaticExpression::Unevaluated { node } = static_value {
            let local_value = if ctx.tree.has_node_id(node.id) {
                self.evaluate_static_expression_value(&mut ctx.type_context_reborrow(), node, None)?
            } else {
                None
            };
            local_value.unwrap_or(StaticExpression::Unevaluated { node })
        } else {
            static_value
        };

        // convert the resulting static expression back into an integer literal
        let value_type_id =
            self.static_expression_type_id_for_substitution(source_id, &static_value, ctx.types);
        let Some(value_type_id) = value_type_id else {
            return Ok(None);
        };
        let value_type_id =
            self.unwrapped_value_without_as_comptime_type_id(value_type_id, ctx.types);
        Ok(self.integer_literal_value_for_type_id(value_type_id, ctx.types))
    }

    /// Check assignability of static arguments on the same reference symbol.
    pub(super) fn are_reference_static_arguments_assignable(
        &self,
        ctx: &mut TypeContext<'_>,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        symbol: GlobalSymbolId,
        target_arguments: Option<&Vec<StaticArgument>>,
        source_arguments: Option<&Vec<StaticArgument>>,
    ) -> bool {
        let target_has_arguments = target_arguments.is_some_and(|args| !args.is_empty());
        let source_has_arguments = source_arguments.is_some_and(|args| !args.is_empty());
        if !target_has_arguments && !source_has_arguments {
            return true;
        }

        let target_source_id = ctx.types.get_type_source(target_id);
        let source_source_id = ctx.types.get_type_source(source_id);

        let target_arguments = self.resolve_reference_static_arguments(
            &mut ctx.reborrow(),
            target_source_id,
            symbol,
            target_arguments,
            source_has_arguments && !target_has_arguments,
        );
        let source_arguments = self.resolve_reference_static_arguments(
            &mut ctx.reborrow(),
            source_source_id,
            symbol,
            source_arguments,
            target_has_arguments && !source_has_arguments,
        );

        if target_arguments.is_empty() && source_arguments.is_empty() {
            return true;
        }
        if target_arguments.len() != source_arguments.len() {
            return false;
        }

        let parameter_symbols = self.collect_static_parameter_symbols(ctx.type_view(), symbol);

        for (index, (target_argument, source_argument)) in target_arguments
            .iter()
            .zip(source_arguments.iter())
            .enumerate()
        {
            let parameter_symbol = parameter_symbols
                .as_ref()
                .and_then(|symbols| symbols.get(index))
                .copied();
            let (parameter_kind, parameter_variance) = if let Some(symbol) = parameter_symbol {
                self.generic_parameter_metadata_for_symbol(&mut ctx.reborrow(), symbol)
            } else {
                (None, None)
            };

            let target_ty_id =
                self.convert_static_argument_type(target_argument, target_source_id, ctx.types);
            let source_ty_id =
                self.convert_static_argument_type(source_argument, source_source_id, ctx.types);
            if self.type_blocks_cascading_diagnostic(target_ty_id, ctx.types)
                || self.type_blocks_cascading_diagnostic(source_ty_id, ctx.types)
            {
                continue;
            }

            let mut static_visited = HashSet::new();
            let target_has_static = self.type_contains_static_parameters(
                ctx.type_view(),
                target_ty_id,
                &mut static_visited,
            );
            let mut static_visited = HashSet::new();
            let source_has_static = self.type_contains_static_parameters(
                ctx.type_view(),
                source_ty_id,
                &mut static_visited,
            );
            let mut infer_visited = HashSet::new();
            let target_has_infer =
                self.type_contains_infer_vars(target_ty_id, ctx.types, &mut infer_visited);
            let mut infer_visited = HashSet::new();
            let source_has_infer =
                self.type_contains_infer_vars(source_ty_id, ctx.types, &mut infer_visited);
            if target_has_static || source_has_static || target_has_infer || source_has_infer {
                continue;
            }

            let target_assignable = self
                .is_type_assignable(&mut ctx.reborrow(), target_ty_id, source_ty_id)
                .is_assignable();
            let source_assignable = self
                .is_type_assignable(&mut ctx.reborrow(), source_ty_id, target_ty_id)
                .is_assignable();

            let use_variance = matches!(parameter_kind, Some(GenericParameterKind::Type));
            let variance = if use_variance {
                parameter_variance
            } else {
                None
            };

            match variance {
                Some(VarianceModifier::Out) => {
                    if !target_assignable {
                        return false;
                    }
                }
                Some(VarianceModifier::In) => {
                    if !source_assignable {
                        return false;
                    }
                }
                Some(VarianceModifier::InOut) | None => {
                    if !target_assignable || !source_assignable {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Resolve static arguments for assignability comparisons.
    pub(super) fn resolve_reference_static_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        generic_arguments: Option<&Vec<StaticArgument>>,
        resolve_defaults: bool,
    ) -> Vec<StaticArgument> {
        let has_arguments = generic_arguments.is_some_and(|args| !args.is_empty());
        if !has_arguments && !resolve_defaults {
            return Vec::new();
        }

        // reuse explicit evaluated positional arguments without re-resolving defaults
        let can_use_explicit_arguments = has_arguments
            && !resolve_defaults
            && generic_arguments.is_some_and(|arguments| {
                arguments.iter().all(|argument| {
                    matches!(
                        argument,
                        StaticArgument::Evaluated {
                            name: None,
                            value: _,
                        }
                    )
                })
            });
        if can_use_explicit_arguments {
            return generic_arguments.cloned().unwrap_or_default();
        }

        // query one resolved argument list, then fall back to the explicit surface on failure
        let resolved = match self.resolve_type_reference_static_arguments(
            &mut ctx.reborrow(),
            node_id,
            symbol,
            generic_arguments.map(|arguments| arguments.as_slice()),
            false,
        ) {
            Ok(arguments) => arguments,
            Err(error) => {
                self.error(error);
                None
            }
        };

        resolved
            .or_else(|| generic_arguments.cloned())
            .unwrap_or_default()
    }

    /// Query one resolved static argument list for assignability comparisons.
    /// Expand type alias references for assignability checks.
    pub(super) fn expand_assignability_alias_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // extract the alias reference and static arguments
        let (symbol, generic_arguments) = match ctx.types.get_type(type_id) {
            Type::Reference {
                symbol,
                generic_arguments,
            } => (*symbol, generic_arguments.clone()),
            _ => return type_id,
        };
        let symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::PreserveAliases,
        );

        // require an alias symbol
        if symbol.ty() != SymbolType::TypeAlias {
            return type_id;
        }

        // resolve the alias target directly for assignability
        let source_id = ctx.types.get_type_source(type_id);
        let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(&mut ctx.reborrow(), symbol, source_id)
        else {
            return type_id;
        };

        // ensure the alias target is evaluated before substitution
        if let Err(error) = self.resolve_declared_type_in_owner(
            &mut ctx.reborrow(),
            symbol.module_id,
            alias_target_id,
        ) {
            self.error(error);
        }

        // normalize directly for aliases without explicit static arguments
        let Some(arguments) = generic_arguments.as_ref() else {
            let mut materialize_cache = HashMap::new();
            return self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                alias_target_id,
                &mut materialize_cache,
            );
        };
        if arguments.is_empty() {
            let mut materialize_cache = HashMap::new();
            return self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                alias_target_id,
                &mut materialize_cache,
            );
        }

        // resolve static arguments for substitution
        let resolved_arguments = match self.resolve_type_reference_static_arguments(
            &mut ctx.reborrow(),
            source_id,
            symbol,
            Some(arguments.as_slice()),
            false,
        ) {
            Ok(arguments) => arguments,
            Err(error) => {
                self.error(error);
                None
            }
        };
        let arguments = resolved_arguments.as_deref().unwrap_or(arguments);
        if arguments.is_empty() {
            return alias_target_id;
        }

        // substitute parameters into the alias target
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            symbol,
            source_id,
            arguments,
        );
        if substitutions.is_empty() {
            return alias_target_id;
        }

        // apply projection substitutions on the alias declaration expression
        let mut alias_target_id = alias_target_id;
        match self.apply_associated_projection_substitutions(
            &mut ctx.reborrow(),
            symbol,
            alias_target_id,
            &substitutions,
        ) {
            Ok(mapped_alias_target_id) => {
                alias_target_id = mapped_alias_target_id;
            }
            Err(error) => {
                self.error(error);
            }
        }

        // instantiate static substitutions and normalize the resulting alias target
        let mut materialize_cache = HashMap::new();
        let mut substitute_cache = HashMap::new();

        self.instantiate_type_with_substitutions(
            &mut ctx.reborrow(),
            source_id,
            Some(symbol),
            alias_target_id,
            &substitutions,
            &mut materialize_cache,
            &mut substitute_cache,
        )
    }

    /// Resolve one declared type id and emit diagnostics on failure.
    fn resolve_declared_type_or_report(&self, ctx: &mut TypeContext<'_>, type_id: LocalTypeId) {
        if let Err(error) = self.resolve_declared_type(&mut ctx.reborrow(), type_id) {
            self.error(error);
        }
    }

    /// Narrow the conditional then branch for assignability checks.
    pub(super) fn narrow_conditional_then_for_assignability(
        &self,
        ctx: &mut TypeContext<'_>,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
        then_type_id: LocalTypeId,
    ) -> LocalTypeId {
        // prefer the condition right side when the branch mirrors the left
        if ctx.types.get_type(left_id) == ctx.types.get_type(then_type_id) {
            return right_id;
        }

        // narrow static parameters with intersection constraints
        let symbol = match ctx.types.get_type(left_id) {
            Type::Reference { symbol, .. } => *symbol,
            _ => return then_type_id,
        };
        if !self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
            return then_type_id;
        }

        let source_id = ctx.types.get_type_source(left_id);
        let narrowed_left = ctx.types.insert_type_from_any(
            Type::Intersection {
                elements: vec![left_id, right_id],
            },
            source_id,
        );
        let mut substitutions = HashMap::new();
        substitutions.insert(symbol, narrowed_left);
        let mut cache = HashMap::new();
        self.substitute_static_parameters(then_type_id, &substitutions, ctx.types, &mut cache)
    }
}
