use super::*;

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
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // unwrap cached alias instances first
        let type_id = self.unwrap_assignability_alias_type(type_id, types);

        // expand alias references that carry static arguments
        let type_id =
            self.expand_assignability_alias_reference(module, profile, type_id, symbols, types);

        // substitute static parameter references with constraints
        let type_id =
            self.resolve_assignability_static_constraint(module, profile, type_id, symbols, types);

        // normalize the prepared type once so downstream checks see a stable shape
        self.normalize_type(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
        )
    }

    /// Resolve static parameter references to their constraints for assignability.
    pub(super) fn resolve_assignability_static_constraint(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let Type::Reference { symbol, .. } = types.get_type(type_id) else {
            return type_id;
        };

        // only substitute actual static parameters
        if !self.symbol_is_static_parameter(module, profile, *symbol, symbols, types) {
            return type_id;
        }

        // resolve the declared constraint type
        let constraint_id = self.static_parameter_constraint_type(
            module,
            profile,
            *symbol,
            types.get_type_source(type_id),
            symbols,
            types,
        );
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
    pub(super) fn reference_static_arguments_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        symbol: GlobalSymbolId,
        target_arguments: Option<&Vec<StaticArgument>>,
        source_arguments: Option<&Vec<StaticArgument>>,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let target_has_arguments = target_arguments.is_some_and(|args| !args.is_empty());
        let source_has_arguments = source_arguments.is_some_and(|args| !args.is_empty());
        if !target_has_arguments && !source_has_arguments {
            return true;
        }

        let target_arguments = self.resolved_reference_static_arguments_for_assignability(
            module,
            profile,
            types.get_type_source(target_id),
            symbol,
            target_arguments,
            source_has_arguments && !target_has_arguments,
            symbols,
            types,
            options,
        );
        let source_arguments = self.resolved_reference_static_arguments_for_assignability(
            module,
            profile,
            types.get_type_source(source_id),
            symbol,
            source_arguments,
            target_has_arguments && !source_has_arguments,
            symbols,
            types,
            options,
        );

        if target_arguments.is_empty() && source_arguments.is_empty() {
            return true;
        }
        if target_arguments.len() != source_arguments.len() {
            return false;
        }

        let local_tree = module.dir(profile).tree.try_read();
        let parameter_symbols = if let Some(tree) = local_tree.as_ref() {
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols, types)
        } else {
            self.with_module_types_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                types,
                AnalyzeDependencyStage::Declare,
                |_, owner_types| owner_types.get_static_parameter_symbols(symbol),
            )
            .ok()
            .flatten()
        };

        let target_source_id = types.get_type_source(target_id);
        let source_source_id = types.get_type_source(source_id);
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
                self.static_parameter_metadata_for_symbol(
                    module,
                    profile,
                    symbol,
                    local_tree.as_deref(),
                    symbols,
                    types,
                )
            } else {
                (None, None)
            };

            let target_ty_id =
                self.convert_static_argument_type(target_argument, target_source_id, types);
            let source_ty_id =
                self.convert_static_argument_type(source_argument, source_source_id, types);
            if self.type_blocks_cascading_diagnostic(target_ty_id, types)
                || self.type_blocks_cascading_diagnostic(source_ty_id, types)
            {
                continue;
            }

            let mut static_visited = HashSet::new();
            let target_has_static = self.type_contains_static_parameters(
                module,
                profile,
                target_ty_id,
                symbols,
                types,
                &mut static_visited,
            );
            let mut static_visited = HashSet::new();
            let source_has_static = self.type_contains_static_parameters(
                module,
                profile,
                source_ty_id,
                symbols,
                types,
                &mut static_visited,
            );
            let mut infer_visited = HashSet::new();
            let target_has_infer =
                self.type_contains_infer_vars(target_ty_id, types, &mut infer_visited);
            let mut infer_visited = HashSet::new();
            let source_has_infer =
                self.type_contains_infer_vars(source_ty_id, types, &mut infer_visited);
            if target_has_static || source_has_static || target_has_infer || source_has_infer {
                continue;
            }

            let target_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_ty_id,
                    source_ty_id,
                    types,
                    options,
                )
                .is_assignable();
            let source_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    source_ty_id,
                    target_ty_id,
                    types,
                    options,
                )
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
    pub(super) fn resolved_reference_static_arguments_for_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&Vec<StaticArgument>>,
        resolve_defaults: bool,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
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

        let tree = module.dir(profile).tree.read();
        let resolved = self
            .resolve_type_reference_static_arguments(
                module,
                profile,
                node_id,
                symbol,
                static_arguments.map(|args| args.as_slice()),
                false,
                options,
                &tree,
                symbols,
                types,
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
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // extract the alias reference and static arguments
        let (symbol, static_arguments) = match types.get_type(type_id) {
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
        let source_id = types.get_type_source(type_id);
        let Some(alias_target_id) = self
            .alias_target_type_id_for_symbol(module, profile, symbol, source_id, symbols, types)
        else {
            return type_id;
        };

        // ensure the alias target is evaluated before substitution
        if matches!(types.get_type(alias_target_id), Type::Unevaluated(_)) {
            // resolve local alias targets directly from this module state
            if symbol.module_id == module.id && types.module_id == module.id {
                let tree = module.dir(profile).tree.read();
                self.resolve_declared_type_or_report(
                    module,
                    profile,
                    alias_target_id,
                    &tree,
                    symbols,
                    types,
                );
            }
            // resolve remote alias targets through stage-gated reads
            else if let Err(error) = self.with_module_tree_symbols_at_stage(
                module,
                profile,
                symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    self.resolve_declared_type_or_report(
                        owner_module,
                        profile,
                        alias_target_id,
                        owner_tree,
                        owner_symbols,
                        types,
                    );
                },
            ) {
                self.error(AnalyzeError::from(error));
            }
        }

        // normalize directly for aliases without explicit static arguments
        let Some(arguments) = static_arguments.as_ref() else {
            return self.normalize_type(
                module,
                profile,
                alias_target_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
        };
        if arguments.is_empty() {
            return self.normalize_type(
                module,
                profile,
                alias_target_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
        }

        // resolve static arguments for substitution
        let tree = module.dir(profile).tree.read();
        let options = self.analyze_context_options_for_module(module.id);
        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                Some(arguments),
                false,
                &options,
                &tree,
                symbols,
                types,
            )
            .ok()
            .flatten();
        let arguments = resolved_arguments.as_deref().unwrap_or(arguments);
        if arguments.is_empty() {
            return alias_target_id;
        }

        // substitute parameters into the alias target
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module, profile, symbol, source_id, arguments, &tree, symbols, types,
        );
        if substitutions.is_empty() {
            return self.normalize_type(
                module,
                profile,
                alias_target_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
        }

        // apply substitutions and normalize the result
        let mut cache = HashMap::new();
        let substituted =
            self.substitute_static_parameters(alias_target_id, &substitutions, types, &mut cache);
        self.normalize_type(
            module,
            profile,
            substituted,
            symbols,
            types,
            NormalizationMode::Assign,
        )
    }

    /// Resolve one declared type id and emit diagnostics on failure.
    fn resolve_declared_type_or_report(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &destack_dir::NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) {
        if let Err(error) =
            self.resolve_declared_type(module, profile, type_id, tree, symbols, types)
        {
            self.error(error);
        }
    }

    /// Narrow the conditional then branch for assignability checks.
    pub(super) fn narrow_conditional_then_for_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
        then_type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // prefer the condition right side when the branch mirrors the left
        if types.get_type(left_id) == types.get_type(then_type_id) {
            return right_id;
        }

        // narrow static parameters with intersection constraints
        let symbol = match types.get_type(left_id) {
            Type::Reference { symbol, .. } => *symbol,
            _ => return then_type_id,
        };
        if !self.symbol_is_static_parameter(module, profile, symbol, symbols, types) {
            return then_type_id;
        }

        let source_id = types.get_type_source(left_id);
        let narrowed_left = types.insert_type_from_any(
            Type::Intersection {
                elements: vec![left_id, right_id],
            },
            source_id,
        );
        let mut substitutions = HashMap::new();
        substitutions.insert(symbol, narrowed_left);
        let mut cache = HashMap::new();
        self.substitute_static_parameters(then_type_id, &substitutions, types, &mut cache)
    }
}
