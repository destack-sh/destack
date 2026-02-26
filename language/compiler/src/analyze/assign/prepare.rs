use super::*;
use crate::analyze::common::TypeContext;

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
                static_arguments,
            } = types.get_type(current_id)
            else {
                break;
            };

            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }
            // avoid unwrapping alias instances with explicit static arguments
            if static_arguments
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
        if let Some(symbol) = self.unwrap_type_value_symbol(ctx.types, type_id)
            && symbol.ty() == SymbolType::Newtype
        {
            return type_id;
        }

        // normalize the prepared type once so downstream checks see a stable shape
        self.normalize_type(&mut ctx.reborrow(), type_id, NormalizationMode::Assign)
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
            self.static_parameter_constraint_type(&mut ctx.reborrow(), symbol, source_id);
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
        target_count: LocalTypeId,
        source_count: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        if target_count == source_count {
            return true;
        }

        let target_value = self.array_sized_count_literal_value(target_count, types);
        let source_value = self.array_sized_count_literal_value(source_count, types);
        let (Some(target_value), Some(source_value)) = (target_value, source_value) else {
            return false;
        };

        target_value == source_value
    }

    /// Check whether a fixed array count matches a literal length.
    pub(super) fn array_sized_count_matches_length(
        &self,
        count: LocalTypeId,
        length: usize,
        types: &TypeTable,
    ) -> bool {
        let Some(value) = self.array_sized_count_literal_value(count, types) else {
            return false;
        };

        value == length as i64
    }

    /// Extract an integer literal value for a fixed-array count type.
    pub(super) fn array_sized_count_literal_value(
        &self,
        count: LocalTypeId,
        types: &TypeTable,
    ) -> Option<i64> {
        let count_ty_id = types.unwrap_value_type_id(count);
        self.integer_literal_value_for_type_id(count_ty_id, types)
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
                self.static_parameter_metadata_for_symbol(&mut ctx.reborrow(), symbol)
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

            let use_variance = matches!(parameter_kind, Some(StaticParameterKind::Type));
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
        static_arguments: Option<&Vec<StaticArgument>>,
        resolve_defaults: bool,
    ) -> Vec<StaticArgument> {
        let has_arguments = static_arguments.is_some_and(|args| !args.is_empty());
        if !has_arguments && !resolve_defaults {
            return Vec::new();
        }

        // reuse explicit evaluated positional arguments without re-resolving defaults
        let can_use_explicit_arguments = has_arguments
            && !resolve_defaults
            && static_arguments.is_some_and(|arguments| {
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
            return static_arguments.cloned().unwrap_or_default();
        }

        let resolved = self
            .resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                node_id,
                symbol,
                static_arguments.map(|args| args.as_slice()),
                false,
            )
            .ok()
            .flatten();

        resolved
            .or_else(|| static_arguments.cloned())
            .unwrap_or_default()
    }

    /// Expand type alias references for assignability checks.
    pub(super) fn expand_assignability_alias_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // extract the alias reference and static arguments
        let (symbol, static_arguments) = match ctx.types.get_type(type_id) {
            Type::Reference {
                symbol,
                static_arguments,
            } => (*symbol, static_arguments.clone()),
            _ => return type_id,
        };

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
        if matches!(ctx.types.get_type(alias_target_id), Type::Unevaluated(_)) {
            // resolve local alias targets directly from this module state
            if symbol.module_id == ctx.module.id && ctx.types.module_id == ctx.module.id {
                self.resolve_declared_type_or_report(&mut ctx.reborrow(), alias_target_id);
            }
            // resolve remote alias targets through stage-gated reads
            else if let Err(error) = self.with_module_tree_symbol_view_at_stage(
                ctx.module,
                ctx.profile,
                symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |view| {
                    let options = self.analyze_context_options_for_module(view.module.id);
                    let mut ctx = ctx.reborrow_for_module_with_options(
                        view.module,
                        &options,
                        view.tree,
                        view.symbols,
                    );
                    self.resolve_declared_type_or_report(&mut ctx, alias_target_id);
                },
            ) {
                self.error(AnalyzeError::from(error));
            }
        }

        // normalize directly for aliases without explicit static arguments
        let Some(arguments) = static_arguments.as_ref() else {
            return self.normalize_type(
                &mut ctx.reborrow(),
                alias_target_id,
                NormalizationMode::Assign,
            );
        };
        if arguments.is_empty() {
            return self.normalize_type(
                &mut ctx.reborrow(),
                alias_target_id,
                NormalizationMode::Assign,
            );
        }

        // resolve static arguments for substitution
        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                Some(arguments.as_slice()),
                false,
            )
            .ok()
            .flatten();
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
            return self.normalize_type(
                &mut ctx.reborrow(),
                alias_target_id,
                NormalizationMode::Assign,
            );
        }

        // apply substitutions and normalize the result
        let mut cache = HashMap::new();
        let substituted = self.substitute_static_parameters(
            alias_target_id,
            &substitutions,
            ctx.types,
            &mut cache,
        );
        self.normalize_type(&mut ctx.reborrow(), substituted, NormalizationMode::Assign)
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
