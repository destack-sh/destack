use std::collections::{HashMap, HashSet};

use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, ContextualTypingMode, InferTablesContext,
};
use crate::analyze::module::GlobalMergeCategory;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    AnchoredGlobalNodeId, Argument, BindingKind, Constraint, Declaration, DependencyItem,
    DynamicKey, EnumFieldValue, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId,
    InferOrigin, InferScope, InferTable, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId,
    Mutability, NodeTree, ScalarLiteral, StaticArgument, StaticExpression, StaticKey,
    StaticParameter, StaticParameterKind, StaticProperty, StringId, SymbolTable, SymbolType, Type,
    TypeElement, TypeField, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Inherited static arguments and substitutions for a type reference.
#[derive(Debug, Clone)]
pub(crate) struct InheritedStaticArguments {
    /// Static arguments inherited from the receiver.
    pub(crate) arguments: Vec<StaticArgument>,
    /// Substitutions for type parameters in inherited arguments.
    pub(crate) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a static parameter symbol for a reference expression.
    pub(crate) fn static_parameter_symbol_for_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_symbols_or_local_at_stage(
            module,
            profile,
            target_symbol.module_id,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_symbols| {
                self.static_parameter_symbol_for_reference_in_symbols(
                    owner_module,
                    profile,
                    *target_symbol,
                    owner_symbols,
                    types,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve a static parameter symbol within a symbol table.
    fn static_parameter_symbol_for_reference_in_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            return Some(target_symbol);
        }
        None
    }

    /// Run logic with the module and tree that own a static argument node.
    pub(crate) fn with_static_argument_owner<T>(
        &self,
        profile: ProfileId,
        argument_node: GlobalNodeIdAny,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        f: impl FnOnce(&Module, &NodeTree, &SymbolTable, LocalNodeId<Argument>) -> AnalyzeResult<T>,
    ) -> AnalyzeResult<Option<T>> {
        // static arguments should always point at argument nodes
        let argument_id = match argument_node.try_into_local_typed::<Argument>() {
            Ok(argument_id) => argument_id,
            Err(_) => {
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: argument_node.into_anchored(Some(profile)),
                    message: "static argument does not resolve to an argument node".to_string(),
                });
                return Ok(None);
            }
        };

        // prefer the call site tree when it owns the argument node
        if argument_node.module_id == module.id && tree.has_node_id(argument_node.local_id.id) {
            return f(module, tree, symbols, argument_id).map(Some);
        }

        // otherwise, resolve the owning module and ensure the node exists there
        let argument_module = self.program.modules.get(argument_node.module_id);
        let argument_module = argument_module.read();
        let argument_tree = argument_module.dir(profile).tree.read();
        if !argument_tree.has_node_id(argument_node.local_id.id) {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: argument_node.into_anchored(Some(profile)),
                message: "static argument node is missing".to_string(),
            });
            return Ok(None);
        }
        let argument_symbols = argument_module.dir(profile).symbols.read();
        f(
            &argument_module,
            &argument_tree,
            &argument_symbols,
            argument_id,
        )
        .map(Some)
    }

    /// Map static argument values to parameters by name and position.
    pub(crate) fn assign_static_argument_values(
        &self,
        module: &Module,
        profile_id: ProfileId,
        node_id: LocalNodeIdAny,
        static_arguments: &[StaticArgument],
        parameters: &[StaticParameter],
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Vec<Option<StaticArgument>> {
        // precompute argument names and detect mixed styles
        let mut has_named_arguments = false;
        let mut argument_infos = Vec::with_capacity(static_arguments.len());
        for argument in static_arguments {
            let (argument_name, is_spread) = match argument {
                StaticArgument::Evaluated { name, .. } => (*name, false),
                StaticArgument::Unevaluated { node } => {
                    let mut info = None;
                    let _ = self.with_static_argument_owner(
                        profile_id,
                        *node,
                        module,
                        tree,
                        symbols,
                        |_, owner_tree, _, argument_id| {
                            let argument = owner_tree.get(argument_id);
                            info = Some(match argument {
                                Argument::Named { name, .. } => {
                                    has_named_arguments = true;
                                    (Some(*name), false)
                                }
                                Argument::Spread { .. } => (None, true),
                                _ => (None, false),
                            });
                            Ok(())
                        },
                    );
                    info.unwrap_or((None, false))
                }
            };

            // reject spread arguments
            if is_spread {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => *node,
                    _ => node_id.into_global(module.id),
                };
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: error_node.into_anchored(Some(profile_id)),
                    message: "static argument spread is not supported".to_string(),
                });
                return vec![None; parameters.len()];
            }

            argument_infos.push((argument, argument_name));
        }

        // reject named static arguments while we only support positional syntax
        if has_named_arguments {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: node_id
                    .into_global(module.id)
                    .into_anchored(Some(profile_id)),
                message: "static arguments must be positional".to_string(),
            });
            return vec![None; parameters.len()];
        }

        // track assignments by parameter index
        let mut assigned: Vec<Option<StaticArgument>> = vec![None; parameters.len()];
        let mut next_index = 0;

        for (argument, argument_name) in argument_infos {
            // select the target parameter index
            let target_index = match argument_name {
                Some(name) => parameters
                    .iter()
                    .position(|parameter| parameter.name == Some(name)),
                None => {
                    let mut index = next_index;
                    while index < parameters.len() && assigned[index].is_some() {
                        index += 1;
                    }
                    next_index = index + 1;
                    if index < parameters.len() {
                        Some(index)
                    } else {
                        None
                    }
                }
            };

            // report unknown or overflowed argument positions
            let Some(target_index) = target_index else {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => *node,
                    _ => node_id.into_global(module.id),
                };
                let message = if argument_name.is_some() {
                    "unknown static argument name".to_string()
                } else {
                    "too many static arguments".to_string()
                };
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: error_node.into_anchored(Some(profile_id)),
                    message,
                });
                continue;
            };

            // reject duplicate assignments
            if assigned[target_index].is_some() {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => *node,
                    _ => node_id.into_global(module.id),
                };
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: error_node.into_anchored(Some(profile_id)),
                    message: "duplicate static argument".to_string(),
                });
                continue;
            }

            assigned[target_index] = Some(argument.clone());
        }

        assigned
    }

    /// Resolve inherited static arguments and substitutions for a receiver type.
    pub(crate) fn resolve_inherited_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        receiver_ty: &Type,
        infer: &InferTable,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<InheritedStaticArguments> {
        // resolve the receiver into a symbol and static arguments
        let reference = if let Some(reference) =
            self.receiver_reference_for_inherited_arguments(receiver_ty, types)
        {
            Some(reference)
        } else {
            match self.well_known_type(profile, receiver_ty, types) {
                Some(Type::Reference {
                    symbol,
                    static_arguments,
                }) => {
                    let symbol = self
                        .remap_typevalue_symbol_to_type_space(module, profile, symbol)
                        .map_err(AnalyzeError::from)?;
                    Some((symbol, static_arguments))
                }
                _ => None,
            }
        };
        let mut reference = reference;
        let has_usable_reference_arguments =
            |reference: &Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)>| {
                let Some((_, Some(arguments))) = reference.as_ref() else {
                    return false;
                };
                if arguments.is_empty() {
                    return false;
                }

                arguments.iter().all(|argument| match argument {
                    StaticArgument::Evaluated { value, .. } => {
                        self.static_value_argument_is_static(value, types)
                    }
                    StaticArgument::Unevaluated { .. } => false,
                })
            };

        // re-read declared reference arguments when inference widened away static arguments
        if !has_usable_reference_arguments(&reference)
            && let Some(declared_reference) = self.receiver_reference_for_declaration_symbol(
                module,
                receiver_id,
                tree,
                symbols,
                types,
            )
        {
            reference = Some(declared_reference);
        }
        if !has_usable_reference_arguments(&reference)
            && let Some(receiver_ty_id) = receiver_ty_id
            && let Some(source_reference) =
                self.receiver_reference_for_type_source(module, receiver_ty_id, infer, types)
        {
            reference = Some(source_reference);
        }

        // fall back to instance arguments when the receiver type does not preserve them (#Suspicious?)
        let instance_reference = || {
            // prefer instances registered on the receiver expression
            let receiver_global_id = receiver_id.into_global(module.id);
            if let Some(instance_reference) = self.query_instance_symbol_arguments_for_node_infer(
                receiver_global_id,
                infer,
                types,
            ) {
                return Some(instance_reference);
            }

            // fall back to the source node for the inferred type
            if let Some(receiver_ty_id) = receiver_ty_id {
                let source_id = types.get_type_source(receiver_ty_id);
                let source_global_id = source_id.into_global(module.id);
                if let Some(instance_reference) = self
                    .query_instance_symbol_arguments_for_node_infer(source_global_id, infer, types)
                {
                    return Some(instance_reference);
                }
            }

            None
        };
        // use instance arguments when reference arguments are missing
        if !has_usable_reference_arguments(&reference)
            && let Some(instance_reference) = instance_reference()
        {
            reference = Some((instance_reference.0, Some(instance_reference.1)));
        }

        let Some((symbol, static_arguments)) = reference else {
            return Ok(InheritedStaticArguments {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            });
        };

        // skip resolution when no explicit static arguments exist
        let has_arguments = static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty());
        if !has_arguments {
            return Ok(InheritedStaticArguments {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            });
        }

        // prefer static argument nodes or type sources to avoid node instance collisions
        let argument_node = static_arguments.as_ref().and_then(|arguments| {
            arguments.iter().find_map(|argument| match argument {
                StaticArgument::Unevaluated { node } if node.module_id == module.id => {
                    Some(node.local_id)
                }
                _ => None,
            })
        });
        let resolution_node_id = argument_node
            .or_else(|| receiver_ty_id.map(|ty_id| types.get_type_source(ty_id)))
            .unwrap_or(receiver_id);

        // resolve static arguments for the type reference
        let resolved: Option<Vec<StaticArgument>> = self.resolve_type_reference_static_arguments(
            module,
            profile,
            resolution_node_id,
            symbol,
            static_arguments.as_deref(),
            true,
            options,
            tree,
            symbols,
            types,
        )?;
        let mut resolved_arguments = match resolved {
            Some(resolved) => resolved,
            None => {
                let Some(static_arguments) = static_arguments.as_ref() else {
                    return Ok(InheritedStaticArguments {
                        arguments: Vec::new(),
                        substitutions: HashMap::new(),
                    });
                };
                if static_arguments.is_empty() {
                    return Ok(InheritedStaticArguments {
                        arguments: Vec::new(),
                        substitutions: HashMap::new(),
                    });
                }

                if symbol.module_id == module.id {
                    self.materialize_static_arguments_for_reference(
                        module,
                        profile,
                        symbol,
                        resolution_node_id,
                        static_arguments,
                        tree,
                        symbols,
                        types,
                    )
                } else {
                    let reference_module = self.program.modules.get(symbol.module_id);
                    let reference_module = reference_module.read();
                    let reference_tree = reference_module.dir(profile).tree.read();
                    let reference_symbols = reference_module.dir(profile).symbols.read();
                    self.materialize_static_arguments_for_reference(
                        &reference_module,
                        profile,
                        symbol,
                        resolution_node_id,
                        static_arguments,
                        &reference_tree,
                        &reference_symbols,
                        types,
                    )
                }
            }
        };

        if resolved_arguments
            .iter()
            .any(|argument| matches!(argument, StaticArgument::Unevaluated { .. }))
        {
            resolved_arguments = if symbol.module_id == module.id {
                self.materialize_static_arguments_for_reference(
                    module,
                    profile,
                    symbol,
                    receiver_id,
                    &resolved_arguments,
                    tree,
                    symbols,
                    types,
                )
            } else {
                let reference_module = self.program.modules.get(symbol.module_id);
                let reference_module = reference_module.read();
                let reference_tree = reference_module.dir(profile).tree.read();
                let reference_symbols = reference_module.dir(profile).symbols.read();
                self.materialize_static_arguments_for_reference(
                    &reference_module,
                    profile,
                    symbol,
                    receiver_id,
                    &resolved_arguments,
                    &reference_tree,
                    &reference_symbols,
                    types,
                )
            };
        }

        // build type parameter substitutions
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            symbol,
            resolution_node_id,
            &resolved_arguments,
            tree,
            symbols,
            types,
        );

        Ok(InheritedStaticArguments {
            arguments: resolved_arguments,
            substitutions,
        })
    }

    /// Infer static arguments for a generic return type from an expected return type.
    pub(crate) fn static_arguments_from_expected_return_type(
        &self,
        module: &Module,
        profile: ProfileId,
        return_type: LocalTypeId,
        expected_return_type: LocalTypeId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, StaticArgument>>> {
        // unwrap type value wrappers
        let return_type = self.unwrap_type_value(return_type, types);
        let expected_return_type = self.unwrap_type_value(expected_return_type, types);

        // extract reference metadata from the return types
        let (return_symbol, mut return_arguments) = match types.get_type(return_type) {
            Type::Reference {
                symbol,
                static_arguments,
            } => (*symbol, static_arguments.clone()),
            _ => return Ok(None),
        };
        let (expected_symbol, mut expected_arguments) = match types.get_type(expected_return_type) {
            Type::Reference {
                symbol,
                static_arguments: Some(arguments),
            } => (*symbol, arguments.clone()),
            _ => return Ok(None),
        };

        // resolve return arguments when possible
        if let Some(arguments) = return_arguments.as_deref() {
            let resolved = self.resolve_type_reference_static_arguments(
                module,
                profile,
                types.get_type_source(return_type),
                return_symbol,
                Some(arguments),
                false,
                options,
                tree,
                symbols,
                types,
            )?;
            if let Some(resolved) = resolved {
                return_arguments = Some(resolved);
            }
        }

        // resolve expected arguments when possible
        if !expected_arguments.is_empty() {
            let resolved = self.resolve_type_reference_static_arguments(
                module,
                profile,
                types.get_type_source(expected_return_type),
                expected_symbol,
                Some(expected_arguments.as_slice()),
                false,
                options,
                tree,
                symbols,
                types,
            )?;
            if let Some(resolved) = resolved {
                expected_arguments = resolved;
            }
        }

        // require a shared canonical target for inference
        let canonical_return = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            return_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let canonical_expected = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            expected_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        if canonical_return != canonical_expected {
            return Ok(None);
        }

        let mut mapping = HashMap::new();
        // map parameter references in the return type to expected arguments
        if let Some(return_arguments) = return_arguments
            && !return_arguments.is_empty()
        {
            // require a one to one argument mapping
            if return_arguments.len() != expected_arguments.len() {
                return Ok(None);
            }

            for (return_argument, expected_argument) in
                return_arguments.iter().zip(expected_arguments.iter())
            {
                let StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } = return_argument
                else {
                    continue;
                };

                let Type::Reference {
                    symbol: parameter_symbol,
                    ..
                } = types.get_type(*ty)
                else {
                    continue;
                };

                if !self.symbol_is_static_parameter(
                    module,
                    profile,
                    *parameter_symbol,
                    symbols,
                    types,
                ) {
                    continue;
                }

                mapping
                    .entry(*parameter_symbol)
                    .or_insert_with(|| expected_argument.clone());
            }
        } else {
            // fall back to parameter order when return arguments are absent
            let tree = module.dir(profile).tree.read();
            let Some(parameter_symbols) = self.collect_static_parameter_symbols(
                module,
                return_symbol,
                profile,
                &tree,
                symbols,
                types,
            ) else {
                return Ok(None);
            };
            if parameter_symbols.len() != expected_arguments.len() {
                return Ok(None);
            }

            for (parameter_symbol, expected_argument) in
                parameter_symbols.iter().zip(expected_arguments.iter())
            {
                mapping
                    .entry(*parameter_symbol)
                    .or_insert_with(|| expected_argument.clone());
            }
        }

        // bail when the return type does not expose parameters
        if mapping.is_empty() {
            return Ok(None);
        }

        // return the inferred mapping
        Ok(Some(mapping))
    }

    /// Extract a reference symbol from receiver types that preserve static arguments.
    fn receiver_reference_for_inherited_arguments(
        &self,
        receiver_ty: &Type,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)> {
        // resolve direct references
        let mut candidate = match receiver_ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => Some((*symbol, static_arguments.clone())),
            Type::Value { value } => {
                let value_ty = types.get_type(*value);
                return self.receiver_reference_for_inherited_arguments(value_ty, types);
            }
            _ => None,
        };

        // scan intersection and union members for a unique reference
        if let Type::Intersection { elements } | Type::Union { elements } = receiver_ty {
            for element_id in elements {
                let element_ty = types.get_type(*element_id);
                let Some(found) =
                    self.receiver_reference_for_inherited_arguments(element_ty, types)
                else {
                    continue;
                };
                if let Some(existing) = candidate.as_ref() {
                    if existing.0 != found.0 || existing.1 != found.1 {
                        return None;
                    }
                } else {
                    candidate = Some(found);
                }
            }
        }

        candidate
    }

    /// Extract a reference symbol from the receiver declaration symbol type when available.
    fn receiver_reference_for_declaration_symbol(
        &self,
        module: &Module,
        receiver_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)> {
        let receiver_expression_id = receiver_id.into_typed::<Expression>();
        let receiver_symbol = tree.get(receiver_expression_id).target_symbol()?;
        if receiver_symbol.module_id != module.id {
            return None;
        }

        let receiver_type_id = types.get_type_id_for_symbol(symbols, receiver_symbol)?;
        let declared_type = types.get_type(receiver_type_id);

        self.receiver_reference_for_inherited_arguments(declared_type, types)
    }

    /// Extract a reference symbol from the source node attached to one receiver type id.
    fn receiver_reference_for_type_source(
        &self,
        module: &Module,
        receiver_ty_id: LocalTypeId,
        infer: &InferTable,
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)> {
        let source_id = types.get_type_source(receiver_ty_id);
        let source_global_id = source_id.into_global(module.id);
        let source_ty_id = infer
            .inferred_type_for_node(source_global_id)
            .or_else(|| types.get_declared_or_inferred_type_id(source_global_id))?;
        let source_ty = types.get_type(source_ty_id);

        self.receiver_reference_for_inherited_arguments(source_ty, types)
    }

    /// Infer a dynamic argument value with contextual typing.
    pub(crate) fn infer_argument(
        &self,
        tables: &mut InferTablesContext<'_>,
        argument_id: LocalNodeId<Argument>,
        expected_ty_id: Option<LocalTypeId>,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let argument = tables.tree.get(argument_id);

        // apply the expected type to the argument value
        let mut argument_ctx = ctx
            .nested_expression_context()
            .with_expected_type(expected_ty_id);
        if expected_ty_id.is_some()
            && matches!(
                argument_ctx.contextual_typing,
                ContextualTypingMode::Satisfies
            )
        {
            // allow contextual typing for argument inference under satisfies
            argument_ctx = argument_ctx.with_contextual_typing_mode(ContextualTypingMode::Default);
        }

        match argument {
            Argument::Positional { value, .. } => {
                self.infer_expression(&mut tables.reborrow(), *value, &mut argument_ctx)?;
            }
            Argument::Named { name: _, value, .. } => {
                self.infer_expression(&mut tables.reborrow(), *value, &mut argument_ctx)?;
            }
            Argument::Labeled {
                label: _, value, ..
            } => {
                self.infer_expression(&mut tables.reborrow(), *value, &mut argument_ctx)?;
            }
            Argument::Spread {
                label: _, value, ..
            } => {
                self.infer_expression(&mut tables.reborrow(), *value, &mut argument_ctx)?;
            }
        }

        Ok(())
    }

    /// Resolve a static argument for a parameter.
    /// Returns `None` when no argument is provided and no default exists.
    pub(crate) fn resolve_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        assigned_argument: Option<StaticArgument>,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // resolve explicit argument when provided
        if let Some(argument) = assigned_argument {
            let resolved_argument = self.resolve_explicit_static_argument(
                module,
                profile,
                static_parameter,
                argument,
                treat_type_arguments_as_types,
                tree,
                symbols,
                types,
            )?;

            // normalize value arguments back into value expressions
            let resolved_argument = if static_parameter.kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved_argument, types)
            } else {
                resolved_argument
            };

            return Ok(Some(resolved_argument));
        }

        // default expression
        if let Some(default_expression) = static_parameter.default_expression.as_ref() {
            let resolved_argument = self.resolve_default_static_argument(
                module,
                profile,
                static_parameter,
                default_expression,
                treat_type_arguments_as_types,
                tree,
                symbols,
                types,
            )?;

            // normalize value defaults back into value expressions
            let resolved_argument = if static_parameter.kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved_argument, types)
            } else {
                resolved_argument
            };

            return Ok(Some(resolved_argument));
        }

        // no argument and no default, caller handles unresolved slots
        Ok(None)
    }

    /// Resolve an explicit static argument for a parameter.
    fn resolve_explicit_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        argument: StaticArgument,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // resolve explicit arguments based on parameter kind
        let resolved_argument = match (static_parameter.kind, argument) {
            (StaticParameterKind::Type, StaticArgument::Unevaluated { node }) => {
                // prefer value literals when type arguments stay unconverted
                if !treat_type_arguments_as_types
                    && let Some(value) = self.evaluate_static_argument_as_value(
                        module, profile, node, tree, symbols, types,
                    )?
                {
                    return Ok(value);
                }

                let resolved = self.evaluate_static_argument_as_type(
                    module, profile, node, tree, symbols, types,
                )?;
                resolved.unwrap_or(StaticArgument::Unevaluated { node })
            }
            (StaticParameterKind::Value, StaticArgument::Unevaluated { node }) => {
                self.resolve_value_static_argument(module, profile, node, tree, symbols, types)?
            }
            (StaticParameterKind::Type, StaticArgument::Evaluated { name, value }) => {
                // preserve explicit values when type arguments stay unconverted
                if !treat_type_arguments_as_types {
                    StaticArgument::Evaluated {
                        name,
                        value: value.clone(),
                    }
                } else {
                    let argument = StaticArgument::Evaluated {
                        name,
                        value: value.clone(),
                    };
                    let ty_id = self.convert_static_argument_type(
                        &argument,
                        types.get_type_source(static_parameter.declared_type_id),
                        types,
                    );

                    StaticArgument::Evaluated {
                        name,
                        value: StaticExpression::Type { ty: ty_id },
                    }
                }
            }
            (_, argument) => argument,
        };

        Ok(resolved_argument)
    }

    /// Resolve a value-kind static argument to a concrete value or reference.
    fn resolve_value_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        node: GlobalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // prefer static value evaluation first
        if let Some(value) = self
            .evaluate_static_argument_as_value_inner(module, profile, node, tree, symbols, types)?
        {
            return Ok(value);
        }

        // preserve static parameter references in value slots
        if let Some(StaticArgument::Evaluated { name, value }) =
            self.evaluate_static_argument_as_type(module, profile, node, tree, symbols, types)?
            && let StaticExpression::Type { ty } = value
            && let Type::Reference { symbol, .. } = types.get_type(ty)
            && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        {
            return Ok(StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty },
            });
        }

        // reject non-static values when no reference is available
        Ok(StaticArgument::Unevaluated { node })
    }

    /// Coerce value static arguments into literal value expressions when possible.
    pub(crate) fn normalize_value_static_argument(
        &self,
        argument: StaticArgument,
        types: &TypeTable,
    ) -> StaticArgument {
        // normalize type-backed literal arguments into value expressions
        match argument {
            StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty },
            } => self
                .static_expression_from_value_type(ty, types)
                .map(|value| StaticArgument::Evaluated { name, value })
                .unwrap_or(StaticArgument::Evaluated {
                    name,
                    value: StaticExpression::Type { ty },
                }),
            StaticArgument::Evaluated {
                name,
                value:
                    StaticExpression::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    },
            } => StaticArgument::Evaluated {
                name,
                value: StaticExpression::ScalarLiteral {
                    value: value.clone(),
                },
            },
            _ => argument,
        }
    }

    /// Build a static value expression from a literal value type.
    fn static_expression_from_value_type(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<StaticExpression> {
        // unwrap value wrappers before matching
        let ty_id = self.unwrap_type_value(ty_id, types);
        let ty = types.get_type(ty_id);

        // map scalar and type literals directly
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            } => Some(StaticExpression::ScalarLiteral {
                value: value.clone(),
            }),
            Type::TypeLiteral { value } => Some(StaticExpression::TypeLiteral {
                value: value.clone(),
            }),
            _ => self.static_expression_from_value_shape(ty_id, types),
        }
    }

    /// Build a static value expression from tuple or object literal types.
    fn static_expression_from_value_shape(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<StaticExpression> {
        // convert tuple element types into static expressions
        if let Type::Tuple { elements, .. } = types.get_type(ty_id) {
            let mut mapped = Vec::with_capacity(elements.len());
            for element in elements {
                let value = self.static_expression_from_value_type(element.ty, types)?;
                mapped.push(value);
            }
            return Some(StaticExpression::TupleExpression { elements: mapped });
        }

        // convert object fields into static expressions when keys are static
        if let Type::Object { fields, .. } = types.get_type(ty_id) {
            let mut properties = Vec::with_capacity(fields.len());
            for field in fields {
                let key = match field.key {
                    StaticKey::Name(name) => Some(DynamicKey::Name(name)),
                    StaticKey::Number(value) => Some(DynamicKey::Number(value)),
                    _ => None,
                }?;

                let value = self.static_expression_from_value_type(field.ty, types)?;
                properties.push(StaticProperty::Field {
                    modifiers: None,
                    key: Some(key),
                    value,
                    default: None,
                    symbol: LocalSymbolId::new(0),
                });
            }
            return Some(StaticExpression::ObjectExpression { properties });
        }

        None
    }

    /// Replace value defaults that reference earlier value parameters.
    fn substitute_value_parameter_reference(
        &self,
        argument: StaticArgument,
        resolved_arguments: &HashMap<GlobalSymbolId, StaticArgument>,
        types: &TypeTable,
        error_node: AnchoredGlobalNodeId,
    ) -> AnalyzeResult<StaticArgument> {
        // walk through value references until a concrete value appears
        let mut current = argument;
        let mut visited = HashSet::new();

        while let StaticArgument::Evaluated { name, value } = current {
            let StaticExpression::Type { ty } = value else {
                // preserve non-reference values as-is
                current = StaticArgument::Evaluated { name, value };
                break;
            };
            let Type::Reference { symbol, .. } = types.get_type(ty) else {
                // stop once the value is not a static parameter reference
                current = StaticArgument::Evaluated { name, value };
                break;
            };
            // guard against circular static defaults
            if !visited.insert(*symbol) {
                return Err(AnalyzeError::CircularStaticArgument { node: error_node });
            }
            let Some(StaticArgument::Evaluated { value, .. }) = resolved_arguments.get(symbol)
            else {
                // keep the reference if no replacement is available yet
                current = StaticArgument::Evaluated { name, value };
                break;
            };

            current = StaticArgument::Evaluated {
                name,
                value: value.clone(),
            };
        }

        Ok(self.normalize_value_static_argument(current, types))
    }

    /// Resolve a default static argument for a parameter.
    fn resolve_default_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        default_expression: &GlobalNodeId<Expression>,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // select the module context for the default expression
        if default_expression.module_id == module.id {
            return self.evaluate_static_default_argument(
                module,
                profile,
                static_parameter.kind,
                static_parameter.name,
                default_expression.local_id,
                treat_type_arguments_as_types,
                tree,
                symbols,
                types,
            );
        }

        // load the remote module context for the default expression
        let default_module = self.program.modules.get(default_expression.module_id);
        let default_module = default_module.read();
        let default_tree = default_module.dir(profile).tree.read();
        let default_symbols = default_module.dir(profile).symbols.read();
        self.evaluate_static_default_argument(
            &default_module,
            profile,
            static_parameter.kind,
            static_parameter.name,
            default_expression.local_id,
            treat_type_arguments_as_types,
            &default_tree,
            &default_symbols,
            types,
        )
    }

    /// Materialize a static type argument for validation.
    pub(crate) fn materialize_static_type_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // pre-evaluate local type aliases used as bounds
        if let Type::Reference { symbol, .. } = types.get_type(static_parameter.declared_type_id)
            && symbol.ty() == SymbolType::TypeAlias
        {
            self.unwrap_type_alias_reference(
                module,
                profile,
                static_parameter.declared_type_id,
                tree,
                symbols,
                types,
            )?;
        }

        // evaluate the declared bound when needed
        self.ensure_static_parameter_bound_evaluated(
            module,
            profile,
            static_parameter.declared_type_id,
            tree,
            symbols,
            types,
        )?;

        // re-evaluate unevaluated arguments before building the substitution type
        let resolved_argument =
            if let StaticArgument::Unevaluated { node } = resolved_static_argument {
                self.evaluate_static_argument_as_type(module, profile, *node, tree, symbols, types)?
                    .unwrap_or_else(|| resolved_static_argument.clone())
            } else {
                resolved_static_argument.clone()
            };

        // build the substitution type from the argument
        let substitution_ty_id =
            self.convert_static_argument_type(&resolved_argument, error_node.local_id, types);

        // ensure referenced instance types are available for validation
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            error_node.local_id,
            static_parameter.declared_type_id,
            types,
        )?;
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            error_node.local_id,
            substitution_ty_id,
            types,
        )?;

        Ok(substitution_ty_id)
    }

    /// Resolve the type of a static value argument for validation.
    fn static_value_argument_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        value: &StaticExpression,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer static parameter constraints for referenced type expressions
        if let StaticExpression::Type { ty } = value
            && let Type::Reference { symbol, .. } = types.get_type(*ty)
            && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
            && let Some(constraint_id) = self.static_parameter_constraint_type(
                module,
                profile,
                *symbol,
                error_node.local_id,
                symbols,
                types,
            )
        {
            self.ensure_static_parameter_bound_evaluated(
                module,
                profile,
                constraint_id,
                tree,
                symbols,
                types,
            )?;
            return Ok(constraint_id);
        }

        // normalize static expression shapes into value types
        self.static_expression_value_type(module, profile, error_node, value, tree, symbols, types)
    }

    /// Check whether a scalar literal matches an enum constraint.
    fn enum_constraint_accepts_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        constraint_ty_id: LocalTypeId,
        literal: &ScalarLiteral,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<bool> {
        // resolve the enum symbol for the constraint when possible
        let enum_symbol = self.enum_symbol_for_type(types.get_type(constraint_ty_id), types);
        let Some(enum_symbol) = enum_symbol else {
            return Ok(false);
        };

        // select the module context for the enum
        let matches = if enum_symbol.module_id == module.id {
            let _ = self.enum_backing_type_for_symbol(module, profile, enum_symbol, types)?;
            self.enum_literal_matches_symbol(enum_symbol, literal, tree, symbols, types)
        } else {
            self.with_module_tree_symbols_types_by_id_at_stage(
                profile,
                enum_symbol.module_id,
                tree,
                symbols,
                types,
                AnalyzeDependencyStage::Declare,
                |owner_tree, owner_symbols, owner_types| {
                    self.enum_literal_matches_symbol(
                        enum_symbol,
                        literal,
                        owner_tree,
                        owner_symbols,
                        owner_types,
                    )
                },
            )
            .map_err(AnalyzeError::from)?
        };

        Ok(matches)
    }

    /// Evaluate a static parameter bound when it is still unevaluated.
    fn ensure_static_parameter_bound_evaluated(
        &self,
        module: &Module,
        profile: ProfileId,
        bound_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        if matches!(types.get_type(bound_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(module, profile, bound_id, tree, symbols, types)?;
        }
        Ok(())
    }

    /// Check whether a scalar literal matches an enum field value.
    fn enum_literal_matches_symbol(
        &self,
        enum_symbol: GlobalSymbolId,
        literal: &ScalarLiteral,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // ensure the symbol refers to an enum declaration
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
        if symbol_entry.ty != SymbolType::Enum {
            return false;
        }

        // collect all enum declarations for the symbol
        let mut declaration_ids = Vec::new();
        if let Some(primary) = symbol_entry.primary_declaration {
            declaration_ids.push(primary);
        }
        if let Some(secondary) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary.iter().copied());
        }

        // scan enum fields for a matching literal value
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let Declaration::Enum { fields, .. } = tree.get(declaration_id) else {
                continue;
            };
            for field_id in fields {
                let field = tree.get(*field_id);
                let field_symbol = field.symbol.into_global(enum_symbol.module_id);
                let Some(value) = types.get_enum_field_value(field_symbol) else {
                    continue;
                };
                if self.enum_field_value_matches_literal(value, literal) {
                    return true;
                }
            }
        }

        false
    }

    /// Check whether an enum field value matches a scalar literal.
    fn enum_field_value_matches_literal(
        &self,
        value: EnumFieldValue,
        literal: &ScalarLiteral,
    ) -> bool {
        match (value, literal) {
            (EnumFieldValue::Int(value), ScalarLiteral::Integer(literal)) => value == *literal,
            (EnumFieldValue::String(value), ScalarLiteral::String(literal)) => value == *literal,
            _ => false,
        }
    }

    /// Convert a static expression into a value type for validation.
    fn static_expression_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        value: &StaticExpression,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // map scalar and type literal values directly
        let ty = match value {
            StaticExpression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            StaticExpression::TypeLiteral { value } => Type::TypeLiteral {
                value: value.clone(),
            },
            StaticExpression::Type { ty } => {
                return Ok(*ty);
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => self.static_expression_tuple_type(
                module, profile, error_node, elements, tree, symbols, types,
            )?,
            StaticExpression::ObjectExpression { properties } => self
                .static_expression_object_type(
                    module, profile, error_node, properties, tree, symbols, types,
                )?,
            StaticExpression::Declaration { .. } | StaticExpression::Unevaluated { .. } => {
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            }
        };

        Ok(types.insert_type_from_any(ty, error_node.local_id))
    }

    /// Convert a static tuple/array expression into a tuple type.
    fn static_expression_tuple_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        elements: &[StaticExpression],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // map tuple literal expressions into tuple types
        let mut element_types = Vec::with_capacity(elements.len());
        for element in elements {
            let element_ty_id = self.static_expression_value_type(
                module, profile, error_node, element, tree, symbols, types,
            )?;
            element_types.push(TypeElement::new(element_ty_id));
        }
        Ok(Type::Tuple {
            elements: element_types,
            is_readonly: false,
        })
    }

    /// Convert a static object expression into an object type.
    fn static_expression_object_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        properties: &[StaticProperty],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // map object literal expressions into structural object types
        let mut fields = Vec::new();
        for property in properties {
            let StaticProperty::Field {
                modifiers,
                key,
                value,
                default: _,
                symbol: _,
            } = property
            else {
                continue;
            };
            let key = match *key {
                Some(DynamicKey::Name(name)) => Some(StaticKey::Name(name)),
                Some(DynamicKey::Number(name)) => Some(StaticKey::Number(name)),
                _ => None,
            };
            let Some(key) = key else {
                continue;
            };
            let field_ty_id = self.static_expression_value_type(
                module, profile, error_node, value, tree, symbols, types,
            )?;
            let is_optional = modifiers.is_some_and(|modifiers| {
                modifiers
                    .kind
                    .is_some_and(|kind| kind == BindingKind::Maybe)
            });
            let is_readonly = modifiers.is_some_and(|modifiers| {
                modifiers
                    .mutability
                    .is_some_and(|mutability| mutability == Mutability::Immutable)
            });
            fields.push(TypeField {
                key,
                ty: field_ty_id,
                is_optional,
                is_readonly,
            });
        }

        Ok(Type::Object {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        })
    }

    /// Validate a static argument against its declared type.
    pub(crate) fn validate_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        prepared_substitution: Option<LocalTypeId>,
        bound_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: Option<&mut InferTable>,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // validate type arguments against the declared bound
        if static_parameter.kind == StaticParameterKind::Type {
            // coerce the argument into a type
            let substitution_ty_id = prepared_substitution.unwrap_or_else(|| {
                self.convert_static_argument_type(
                    resolved_static_argument,
                    error_node.local_id,
                    types,
                )
            });

            // skip bound validation in declaration modules
            if module.language_type.is_declaration() {
                return Ok(Some(substitution_ty_id));
            }

            // resolve bounds via constraint lookup when the declared slot is not concrete
            let mut declared_bound_id = static_parameter.declared_type_id;
            let declared_bound_needs_constraint = matches!(
                types.get_type(declared_bound_id),
                Type::Unevaluated(_)
                    | Type::InferVar { .. }
                    | Type::TypeLiteral {
                        value: TypeLiteral::Unknown | TypeLiteral::Any,
                    }
            );
            if declared_bound_needs_constraint
                && self.symbol_is_static_parameter(
                    module,
                    profile,
                    static_parameter.symbol,
                    symbols,
                    types,
                )
                && let Some(constraint_id) = self.static_parameter_constraint_type(
                    module,
                    profile,
                    static_parameter.symbol,
                    error_node.local_id,
                    symbols,
                    types,
                )
            {
                declared_bound_id = constraint_id;
            }

            // substitute the argument into self referential bounds
            let mut substitutions = bound_substitutions.clone();
            substitutions.insert(static_parameter.symbol, substitution_ty_id);
            let mut cache = HashMap::new();
            let mut expected_ty_id = self.substitute_static_parameters(
                declared_bound_id,
                &substitutions,
                types,
                &mut cache,
            );

            // fall back to resolved constraint types when bounds are unknown
            if matches!(
                types.get_type(expected_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any
                }
            ) && self.symbol_is_static_parameter(
                module,
                profile,
                static_parameter.symbol,
                symbols,
                types,
            ) && let Some(constraint_id) = self.static_parameter_constraint_type(
                module,
                profile,
                static_parameter.symbol,
                error_node.local_id,
                symbols,
                types,
            ) {
                let mut cache = HashMap::new();
                let substituted = self.substitute_static_parameters(
                    constraint_id,
                    &substitutions,
                    types,
                    &mut cache,
                );
                if !matches!(
                    types.get_type(substituted),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown | TypeLiteral::Any
                    }
                ) {
                    expected_ty_id = substituted;
                }
            }

            // accept all arguments when the declared bound is unknown or any
            if matches!(
                types.get_type(expected_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any
                }
            ) {
                return Ok(Some(substitution_ty_id));
            }

            // register an inference constraint for the declared bound
            if let Some(infer) = infer {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: substitution_ty_id,
                    super_type: expected_ty_id,
                    variance: None,
                });
            }

            // report unassignable type
            if !self.is_infer_var_type(declared_bound_id, types)
                && !self.is_infer_var_type(substitution_ty_id, types)
                && self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    expected_ty_id,
                    substitution_ty_id,
                    types,
                    options,
                ) == Assignability::NotAssignable
            {
                // accept static parameter arguments when their constraints satisfy the bound
                if let Type::Reference { symbol, .. } = types.get_type(substitution_ty_id)
                    && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
                    && let Some(constraint_ty_id) = self.static_parameter_constraint_type(
                        module,
                        profile,
                        *symbol,
                        error_node.local_id,
                        symbols,
                        types,
                    )
                    && self.constraint_satisfies_bound(
                        module,
                        profile,
                        expected_ty_id,
                        constraint_ty_id,
                        symbols,
                        types,
                        options,
                    )
                {
                    return Ok(Some(substitution_ty_id));
                }

                // allow type parameters that satisfy the expected bound via their constraints
                if let Some(constraint_ty_id) = self.materialize_static_argument_constraint_type(
                    module,
                    profile,
                    error_node,
                    substitution_ty_id,
                    symbols,
                    types,
                ) && self.constraint_satisfies_bound(
                    module,
                    profile,
                    expected_ty_id,
                    constraint_ty_id,
                    symbols,
                    types,
                    options,
                ) {
                    return Ok(Some(substitution_ty_id));
                }

                // report failed bound validation
                self.emit_unassignable_type_for_types(
                    module,
                    profile,
                    error_node.local_id,
                    expected_ty_id,
                    substitution_ty_id,
                    types,
                );
                return Ok(Some(
                    types.insert_type_from_any(Type::Error, error_node.local_id),
                ));
            }

            // accept validated type arguments
            return Ok(Some(substitution_ty_id));
        }

        // skip value validation in declaration modules
        if module.language_type.is_declaration() {
            return Ok(None);
        }

        // validate value arguments against the declared type
        if static_parameter.kind == StaticParameterKind::Value
            && matches!(resolved_static_argument, StaticArgument::Unevaluated { .. })
        {
            self.error(AnalyzeError::NonStaticArgument {
                node: error_node.into_anchored(Some(profile)),
            });
            return Ok(Some(
                types.insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        let StaticArgument::Evaluated { value, .. } = resolved_static_argument else {
            return Ok(None);
        };

        // reject non static value arguments
        if static_parameter.kind == StaticParameterKind::Value
            && !self.static_value_argument_is_static(value, types)
        {
            self.error(AnalyzeError::NonStaticArgument {
                node: error_node.into_anchored(Some(profile)),
            });
            return Ok(Some(
                types.insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        // reject not assignable value arguments
        let value_ty_id = self
            .static_value_argument_type(module, profile, error_node, value, tree, symbols, types)?;

        // accept enum literal values that match enum constraints
        if let StaticExpression::ScalarLiteral { value: literal } = value
            && self.enum_constraint_accepts_literal(
                module,
                profile,
                static_parameter.declared_type_id,
                literal,
                tree,
                symbols,
                types,
            )?
        {
            return Ok(Some(value_ty_id));
        }

        if !self.is_infer_var_type(static_parameter.declared_type_id, types)
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                static_parameter.declared_type_id,
                value_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            // report unassignable value arguments
            self.emit_unassignable_type_for_types(
                module,
                profile,
                error_node.local_id,
                static_parameter.declared_type_id,
                value_ty_id,
                types,
            );
            return Ok(Some(
                types.insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        Ok(Some(value_ty_id))
    }

    /// Check whether a static value argument is a static expression.
    fn static_value_argument_is_static(&self, value: &StaticExpression, types: &TypeTable) -> bool {
        // classify static expressions by evaluation state
        match value {
            StaticExpression::Unevaluated { .. } => false,
            StaticExpression::ScalarLiteral { .. } => true,
            StaticExpression::TypeLiteral { value } => !matches!(value, TypeLiteral::Unknown),
            StaticExpression::Type { ty } => !matches!(
                types.get_type(*ty),
                Type::Unevaluated(_)
                    | Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
            ),
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_none_or(|arguments| {
                arguments.iter().all(|argument| match argument {
                    StaticArgument::Unevaluated { .. } => false,
                    StaticArgument::Evaluated { value, .. } => {
                        self.static_value_argument_is_static(value, types)
                    }
                })
            }),
            StaticExpression::ArrayExpression { elements } => elements
                .iter()
                .all(|element| self.static_value_argument_is_static(element, types)),
            StaticExpression::TupleExpression { elements } => elements
                .iter()
                .all(|element| self.static_value_argument_is_static(element, types)),
            StaticExpression::ObjectExpression { properties } => properties
                .iter()
                .all(|property| self.static_property_is_static(property, types)),
        }
    }

    /// Check whether a static property is fully static.
    fn static_property_is_static(&self, property: &StaticProperty, types: &TypeTable) -> bool {
        // accept property values only when they are fully static
        match property {
            StaticProperty::Unevaluated { .. } => false,
            StaticProperty::Field { value, default, .. } => {
                self.static_value_argument_is_static(value, types)
                    && default
                        .as_ref()
                        .is_none_or(|value| self.static_value_argument_is_static(value, types))
            }
            StaticProperty::Method { body, .. } => {
                self.static_value_argument_is_static(body, types)
            }
        }
    }

    /// Resolve the target argument mapping for an extension declaration.
    pub(crate) fn extension_target_argument_mapping(
        &self,
        extension_symbol: GlobalSymbolId,
        extension_parameters: &[GlobalSymbolId],
        profile: ProfileId,
    ) -> Option<Vec<usize>> {
        // load the extension declaration
        let module = self.program.modules.get(extension_symbol.module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        let symbol_entry = symbols.get_symbol(extension_symbol.local_id);
        let declaration_id = symbol_entry
            .primary_declaration?
            .try_into_local_typed::<Declaration>()
            .ok()?;
        let declaration = tree.get(declaration_id);
        let Declaration::Extension { target_type, .. } = declaration else {
            return None;
        };

        // read the target type arguments
        let target_expression = tree.get(*target_type);
        let static_arguments = match target_expression {
            Expression::LocalReference {
                static_arguments, ..
            }
            | Expression::ModuleReference {
                static_arguments, ..
            }
            | Expression::GlobalReference {
                static_arguments, ..
            } => static_arguments.as_ref(),
            _ => None,
        }?;

        // map target arguments to extension parameter indices
        let mut mapping = Vec::with_capacity(static_arguments.len());
        for argument_id in static_arguments {
            let argument = tree.get(*argument_id);
            let expression_id = argument.value();
            let expression = tree.get(expression_id);
            let target_symbol = match expression {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            }?;

            let parameter_index = extension_parameters
                .iter()
                .position(|parameter_symbol| *parameter_symbol == target_symbol)?;
            mapping.push(parameter_index);
        }

        Some(mapping)
    }

    /// Resolve a static argument constraint for validation.
    fn materialize_static_argument_constraint_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        argument_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // collect referenced static parameters
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_type_reference_symbols(
            argument_ty_id,
            types,
            &mut referenced_symbols,
            &mut visited,
        );

        // materialize referenced constraints
        let mut substitutions = HashMap::new();
        let mut visiting_symbols = HashSet::new();
        for symbol in referenced_symbols {
            let constraint_id = self.materialize_static_parameter_constraint(
                module,
                profile,
                error_node,
                symbol,
                symbols,
                types,
                &mut visiting_symbols,
            );
            if let Some(constraint_id) = constraint_id {
                substitutions.insert(symbol, constraint_id);
            }
        }

        // skip when no substitutions are needed
        if substitutions.is_empty() {
            return None;
        }

        // apply nested constraint substitutions
        let mut cache = HashMap::new();
        Some(self.substitute_static_parameters(argument_ty_id, &substitutions, types, &mut cache))
    }

    /// Materialize a static parameter constraint by substituting nested constraints.
    fn materialize_static_parameter_constraint(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visiting: &mut HashSet<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        // avoid recursive constraint expansion
        if !visiting.insert(symbol) {
            return None;
        }

        // resolve the declared constraint type
        if !self.symbol_is_static_parameter(module, profile, symbol, symbols, types) {
            visiting.remove(&symbol);
            return None;
        }
        let constraint_id = self.static_parameter_constraint_type(
            module,
            profile,
            symbol,
            error_node.local_id,
            symbols,
            types,
        );
        let Some(constraint_id) = constraint_id else {
            visiting.remove(&symbol);
            return None;
        };

        // collect nested static parameters in the constraint
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_type_reference_symbols(
            constraint_id,
            types,
            &mut referenced_symbols,
            &mut visited,
        );

        // materialize nested constraints
        let mut substitutions = HashMap::new();
        for referenced_symbol in referenced_symbols {
            if referenced_symbol == symbol {
                continue;
            }
            let nested_constraint = self.materialize_static_parameter_constraint(
                module,
                profile,
                error_node,
                referenced_symbol,
                symbols,
                types,
                visiting,
            );
            if let Some(nested_constraint) = nested_constraint {
                substitutions.insert(referenced_symbol, nested_constraint);
            }
        }

        // stop recursive tracking for this symbol
        visiting.remove(&symbol);

        // return the raw constraint when nothing is substituted
        if substitutions.is_empty() {
            return Some(constraint_id);
        }

        // apply nested substitutions
        let mut cache = HashMap::new();
        Some(self.substitute_static_parameters(constraint_id, &substitutions, types, &mut cache))
    }

    /// Resolve static arguments for a type reference.
    pub(crate) fn resolve_type_reference_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_RESOLVE);
        // canonicalize import targets while preserving alias identity
        let symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let symbol = self.merged_type_symbol_id(module, symbols, profile, symbol);

        self.resolve_type_reference_static_arguments_with_bounds(
            module,
            profile,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            options,
            None,
            tree,
            symbols,
            types,
        )
    }

    /// Resolve static arguments for a canonicalized type reference in one substitution environment.
    pub(crate) fn resolve_type_reference_static_arguments_with_bounds(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        options: &AnalyzeOptions,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // treat type arguments as types for type references
        let treat_type_arguments_as_types = true;

        // check for cached resolved static arguments
        let options_cache_key = options.cache_key();
        let cache_key = if bound_substitutions.is_none() {
            self.static_argument_resolution_cache_key(
                symbol,
                static_arguments,
                validate_static_argument_bounds,
                treat_type_arguments_as_types,
                options_cache_key,
            )
        } else {
            None
        };
        if let Some(cache_key) = cache_key
            && let Some(cached) = types.get_static_argument_resolution_cache(cache_key)
        {
            return Ok(cached);
        }

        // skip non instantiable symbols
        if !self.query_symbol_is_instantiable(symbol) && symbol.ty() != SymbolType::Extension {
            return Ok(None);
        }

        // reuse resolved arguments when an instance is already registered for this node
        let has_explicit_arguments = static_arguments.is_some_and(|args| !args.is_empty());
        let node_global_id = node_id.into_global(module.id);
        if !has_explicit_arguments
            && let Some(arguments) =
                self.query_instance_arguments_for_node(node_global_id, Some(symbol), types)
        {
            return Ok(Some(arguments));
        }

        // guard against recursive resolution on the same reference
        let argument_slice = static_arguments.unwrap_or(&[]);
        if types.is_static_argument_resolution_in_progress(symbol, argument_slice) {
            if !argument_slice.is_empty() {
                let resolved_arguments = if symbol.module_id == module.id {
                    self.materialize_static_arguments_for_reference(
                        module,
                        profile,
                        symbol,
                        node_id,
                        argument_slice,
                        tree,
                        symbols,
                        types,
                    )
                } else {
                    let reference_module = self.program.modules.get(symbol.module_id);
                    let reference_module = reference_module.read();
                    let reference_tree = reference_module.dir(profile).tree.read();
                    let reference_symbols = reference_module.dir(profile).symbols.read();
                    self.materialize_static_arguments_for_reference(
                        &reference_module,
                        profile,
                        symbol,
                        node_id,
                        argument_slice,
                        &reference_tree,
                        &reference_symbols,
                        types,
                    )
                };
                return Ok(Some(resolved_arguments));
            }
            return Ok(None);
        }

        // mark resolution as in progress for this argument set
        let argument_snapshot = argument_slice.to_vec();
        types.mark_static_argument_resolution_in_progress(symbol, argument_snapshot.clone());

        let result = self.resolve_type_reference_static_arguments_inner(
            module,
            profile,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            options,
            bound_substitutions,
            treat_type_arguments_as_types,
            tree,
            symbols,
            types,
        );

        // clear the in progress marker, resolve cached arguments when needed
        types.clear_static_argument_resolution_in_progress(symbol, &argument_snapshot);
        if let Some(cache_key) = cache_key
            && let Ok(resolved) = &result
        {
            let should_cache = resolved
                .as_ref()
                .map(|arguments| arguments.iter().all(StaticArgument::is_evaluated))
                .unwrap_or(true);
            if should_cache {
                types.set_static_argument_resolution_cache(cache_key, resolved.clone());
            }
        }

        result
    }

    /// Resolve static arguments for a type reference.
    fn resolve_type_reference_static_arguments_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        options: &AnalyzeOptions,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // ensure remote declarations are resolved before reading defaults
        if symbol.module_id != module.id {
            self.require_resolve_module_direct(symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
        }

        let mut resolve_with = |argument_module: &Module,
                                argument_tree: &NodeTree,
                                argument_symbols: &SymbolTable|
         -> AnalyzeResult<Option<Vec<StaticArgument>>> {
            // collect parameter symbols for the declaration
            let parameter_symbols = self.collect_static_parameter_symbols(
                argument_module,
                symbol,
                profile,
                argument_tree,
                argument_symbols,
                types,
            );
            let parameter_symbols = match parameter_symbols {
                Some(parameter_symbols) => parameter_symbols,
                None => {
                    // keep explicit arguments when the declaration is unavailable
                    if let Some(static_arguments) = static_arguments
                        && !static_arguments.is_empty()
                    {
                        return Ok(Some(static_arguments.to_vec()));
                    }
                    return Ok(None);
                }
            };

            if parameter_symbols.is_empty() {
                // keep explicit arguments when no parameters exist
                if let Some(static_arguments) = static_arguments
                    && !static_arguments.is_empty()
                {
                    return Ok(Some(static_arguments.to_vec()));
                }

                return Ok(None);
            }

            // gather static parameter metadata for the declaration
            let static_parameters: Vec<_> = parameter_symbols
                .iter()
                .map(|symbol_id| {
                    self.resolve_static_parameter(
                        argument_module,
                        *symbol_id,
                        node_id,
                        profile,
                        argument_tree,
                        argument_symbols,
                        types,
                    )
                })
                .collect();

            // map arguments to parameter slots
            let argument_values = static_arguments.unwrap_or(&[]);
            let assigned_arguments = self.assign_static_argument_values(
                module,
                profile,
                node_id,
                argument_values,
                &static_parameters,
                tree,
                symbols,
            );
            // resolve arguments with defaults and error-type recovery
            let mut resolved_arguments = Vec::with_capacity(static_parameters.len());
            let mut resolved_argument_map = HashMap::new();
            for (index, static_parameter) in static_parameters.iter().enumerate() {
                let assigned_argument = assigned_arguments.get(index).cloned().flatten();
                let error_node = if let Some(argument) = &assigned_argument {
                    match argument {
                        StaticArgument::Unevaluated { node } => *node,
                        StaticArgument::Evaluated { .. } => node_id.into_global(module.id),
                    }
                } else if let Some(default_expression) =
                    static_parameter.default_expression.as_ref()
                {
                    default_expression
                        .local_id
                        .into_global_any(default_expression.module_id)
                } else {
                    node_id.into_global(module.id)
                };

                // resolve the argument value or synthesize error recovery when unresolved
                let mut resolved_argument = self
                    .resolve_static_argument(
                        module,
                        profile,
                        static_parameter,
                        assigned_argument,
                        treat_type_arguments_as_types,
                        tree,
                        symbols,
                        types,
                    )?
                    .unwrap_or_else(|| {
                        // unresolved references synthesize error recovery values by parameter kind
                        let error_ty_id = types.insert_type_from_any(Type::Error, node_id);
                        let synthesized_value = match static_parameter.kind {
                            StaticParameterKind::Type => StaticExpression::Type { ty: error_ty_id },
                            StaticParameterKind::Value => node_id
                                .try_into_typed::<Expression>()
                                .map(|expression_id| StaticExpression::Unevaluated {
                                    node: expression_id,
                                })
                                .unwrap_or(StaticExpression::Type { ty: error_ty_id }),
                        };
                        StaticArgument::Evaluated {
                            name: static_parameter.name,
                            value: synthesized_value,
                        }
                    });

                // substitute earlier value parameters in defaults
                if static_parameter.kind == StaticParameterKind::Value {
                    resolved_argument = self.substitute_value_parameter_reference(
                        resolved_argument,
                        &resolved_argument_map,
                        types,
                        error_node.into_anchored(Some(profile)),
                    )?;
                }

                // inherit value constraints when passing a static parameter through
                if static_parameter.kind == StaticParameterKind::Value {
                    let referenced_symbol = match &resolved_argument {
                        StaticArgument::Evaluated {
                            value: StaticExpression::Type { ty },
                            ..
                        } => self.unwrap_type_value_symbol(types, *ty),
                        _ => None,
                    };

                    if let Some(referenced_symbol) = referenced_symbol {
                        let constraint_id = if self.symbol_is_static_parameter(
                            argument_module,
                            profile,
                            referenced_symbol,
                            argument_symbols,
                            types,
                        ) {
                            self.static_parameter_constraint_type(
                                argument_module,
                                profile,
                                referenced_symbol,
                                error_node.local_id,
                                argument_symbols,
                                types,
                            )
                        } else {
                            None
                        };

                        if let Some(constraint_id) = constraint_id
                            && matches!(
                                types.get_type(constraint_id),
                                Type::TypeLiteral {
                                    value: TypeLiteral::Unknown
                                }
                            )
                        {
                            if matches!(
                                types.get_type(static_parameter.declared_type_id),
                                Type::Unevaluated(_)
                            ) {
                                self.resolve_declared_type(
                                    argument_module,
                                    profile,
                                    static_parameter.declared_type_id,
                                    argument_tree,
                                    argument_symbols,
                                    types,
                                )?;
                            }

                            if !matches!(
                                types.get_type(static_parameter.declared_type_id),
                                Type::TypeLiteral {
                                    value: TypeLiteral::Unknown
                                }
                            ) {
                                types.set_static_parameter_constraint_type(
                                    referenced_symbol,
                                    static_parameter.declared_type_id,
                                );
                            }
                        }
                    }
                }

                // materialized type argument validation when needed
                let materialized_substitution = if validate_static_argument_bounds
                    && static_parameter.kind == StaticParameterKind::Type
                {
                    Some(self.materialize_static_type_argument(
                        argument_module,
                        profile,
                        error_node,
                        static_parameter,
                        &resolved_argument,
                        argument_tree,
                        argument_symbols,
                        types,
                    )?)
                } else {
                    None
                };

                // collect resolved substitutions for prior static parameters
                let local_bound_substitutions = self.static_argument_substitutions_for_bounds(
                    &static_parameters[..resolved_arguments.len()],
                    &resolved_arguments,
                );
                let mut bound_substitutions =
                    bound_substitutions.cloned().unwrap_or_else(HashMap::new);
                bound_substitutions.extend(local_bound_substitutions);

                // validate type and value arguments against declared bounds
                let validated_type = if validate_static_argument_bounds {
                    self.validate_static_argument(
                        module,
                        profile,
                        error_node,
                        static_parameter,
                        &resolved_argument,
                        materialized_substitution,
                        &bound_substitutions,
                        tree,
                        symbols,
                        types,
                        None,
                        options,
                    )?
                } else {
                    None
                };

                // update resolved arguments with validated substitutions
                if let Some(substitution_ty_id) = validated_type {
                    let argument_name = match &resolved_argument {
                        StaticArgument::Evaluated { name, .. } => *name,
                        StaticArgument::Unevaluated { .. } => None,
                    };
                    let preserves_reference = match &resolved_argument {
                        StaticArgument::Evaluated {
                            value: StaticExpression::Type { ty },
                            ..
                        } => match types.get_type(*ty) {
                            Type::Reference { symbol, .. } => self.symbol_is_static_parameter(
                                argument_module,
                                profile,
                                *symbol,
                                argument_symbols,
                                types,
                            ),
                            _ => false,
                        },
                        _ => false,
                    };
                    if matches!(types.get_type(substitution_ty_id), Type::Error)
                        || static_parameter.kind == StaticParameterKind::Type
                    {
                        resolved_argument = StaticArgument::Evaluated {
                            name: argument_name,
                            value: StaticExpression::Type {
                                ty: substitution_ty_id,
                            },
                        };
                    } else if static_parameter.kind == StaticParameterKind::Value
                        && !preserves_reference
                    {
                        let replacement = StaticArgument::Evaluated {
                            name: argument_name,
                            value: StaticExpression::Type {
                                ty: substitution_ty_id,
                            },
                        };
                        resolved_argument =
                            self.normalize_value_static_argument(replacement, types);
                    }
                }

                resolved_argument_map.insert(static_parameter.symbol, resolved_argument.clone());
                resolved_arguments.push(resolved_argument);
            }

            Ok(Some(resolved_arguments))
        };

        if symbol.module_id == module.id {
            return resolve_with(module, tree, symbols);
        }

        let reference_module = self.program.modules.get(symbol.module_id);
        let reference_module = reference_module.read();
        let reference_tree = reference_module.dir(profile).tree.read();
        let reference_symbols = reference_module.dir(profile).symbols.read();
        resolve_with(&reference_module, &reference_tree, &reference_symbols)
    }

    /// Collect static parameter substitutions for bound evaluation.
    fn static_argument_substitutions_for_bounds(
        &self,
        static_parameters: &[StaticParameter],
        resolved_arguments: &[StaticArgument],
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        // collect resolved type substitutions for prior parameters
        let mut substitutions = HashMap::new();
        for (parameter, argument) in static_parameters.iter().zip(resolved_arguments.iter()) {
            let resolved_type = match argument {
                StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } => Some(*ty),
                _ => None,
            };
            let Some(resolved_type) = resolved_type else {
                continue;
            };

            substitutions.insert(parameter.symbol, resolved_type);
        }

        substitutions
    }

    /// Build type parameter substitutions for a type symbol.
    pub(crate) fn build_type_parameter_substitutions_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        resolved_arguments: &[StaticArgument],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        // collect static parameter symbols for the declaration
        let parameter_symbols =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols, types);
        let Some(parameter_symbols) = parameter_symbols else {
            return HashMap::new();
        };
        if parameter_symbols.is_empty() {
            return HashMap::new();
        }

        // collect static parameters for the declaration
        let static_parameters: Vec<_> = parameter_symbols
            .iter()
            .map(|symbol_id| {
                self.resolve_static_parameter(
                    module, *symbol_id, source_id, profile, tree, symbols, types,
                )
            })
            .collect();

        // build substitutions for type and value parameters
        let mut substitutions = HashMap::new();
        for (static_parameter, argument) in static_parameters.iter().zip(resolved_arguments.iter())
        {
            let ty_id = self.convert_static_argument_type(
                argument,
                types.get_type_source(static_parameter.declared_type_id),
                types,
            );
            substitutions.insert(static_parameter.symbol, ty_id);
        }

        let symbol_info = self
            .with_module_symbols_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                symbols,
                AnalyzeDependencyStage::Declare,
                |_, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                    (symbol_entry.key, symbol_entry.space)
                },
            )
            .ok();
        if let Some((symbol_key, symbol_space)) = symbol_info
            && let Some(symbol_key) = symbol_key
        {
            let resolved_parameter_types = parameter_symbols
                .iter()
                .map(|symbol| substitutions.get(symbol).copied())
                .collect::<Vec<_>>();

            if resolved_parameter_types.iter().any(|ty| ty.is_some()) {
                // propagate substitutions across all merge peers by position
                let merge_symbols = self.collect_global_merge_sources_for_key(
                    module,
                    profile,
                    symbol_key,
                    symbol_space,
                    GlobalMergeCategory::Instance,
                );

                for merge_symbol in merge_symbols {
                    if merge_symbol == symbol {
                        continue;
                    }

                    let Some(other_parameters) = self.collect_static_parameter_symbols(
                        module,
                        merge_symbol,
                        profile,
                        tree,
                        symbols,
                        types,
                    ) else {
                        continue;
                    };

                    for (index, parameter_symbol) in other_parameters.iter().enumerate() {
                        let Some(Some(mapped)) = resolved_parameter_types.get(index) else {
                            continue;
                        };
                        substitutions.entry(*parameter_symbol).or_insert(*mapped);
                    }
                }
            }
        }

        substitutions
    }

    /// Evaluate a static argument as a type.
    pub(crate) fn evaluate_static_argument_as_type(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_node: GlobalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        let mut evaluated = None;
        let _ = self.with_static_argument_owner(
            profile,
            argument_node,
            module,
            tree,
            symbols,
            |argument_module, argument_tree, argument_symbols, argument_id| {
                let argument = argument_tree.get(argument_id);
                let argument_name = match argument {
                    Argument::Named { name, .. } => Some(*name),
                    _ => None,
                };

                let expression_id = argument.value();

                // preserve static parameter references in type arguments
                if let Some(parameter_symbol) = self.static_parameter_symbol_for_reference(
                    argument_module,
                    profile,
                    expression_id,
                    argument_tree,
                    argument_symbols,
                    types,
                )? {
                    let reference_ty = Type::Reference {
                        symbol: parameter_symbol,
                        static_arguments: None,
                    };
                    let ty_id = types.insert_type_from_any(reference_ty, expression_id.into_any());
                    evaluated = Some(StaticArgument::Evaluated {
                        name: argument_name,
                        value: StaticExpression::Type { ty: ty_id },
                    });
                    return Ok(());
                }

                // try evaluate expression as a type
                let ty_id = self.resolve_declared_type_expression(
                    argument_module,
                    profile,
                    expression_id,
                    argument_tree,
                    argument_symbols,
                    types,
                    true,
                    true,
                )?;
                if matches!(types.get_type(ty_id), Type::Unevaluated { .. }) {
                    return Ok(());
                }

                evaluated = Some(StaticArgument::Evaluated {
                    name: argument_name,
                    value: StaticExpression::Type { ty: ty_id },
                });
                Ok(())
            },
        )?;

        Ok(evaluated)
    }

    /// Evaluate a static argument as a value.
    pub(crate) fn evaluate_static_argument_as_value(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_node: GlobalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        self.evaluate_static_argument_as_value_inner(
            module,
            profile,
            argument_node,
            tree,
            symbols,
            types,
        )
    }

    /// Evaluate a static argument as a value.
    fn evaluate_static_argument_as_value_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_node: GlobalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        let mut evaluated = None;
        let _ = self.with_static_argument_owner(
            profile,
            argument_node,
            module,
            tree,
            symbols,
            |argument_module, argument_tree, argument_symbols, argument_id| {
                // capture argument name for reuse in evaluated form
                let argument = argument_tree.get(argument_id);
                let argument_name = match argument {
                    Argument::Named { name, .. } => Some(*name),
                    _ => None,
                };

                // evaluate the expression into a static value when possible
                let expression_id = argument.value();
                let value = self.evaluate_static_expression_value(
                    argument_module,
                    profile,
                    expression_id,
                    argument_tree,
                    argument_symbols,
                    types,
                    None,
                )?;
                let value = if let Some(value) = value {
                    value
                } else if let Some(enum_symbol) = self.enum_symbol_for_member_expression(
                    argument_module,
                    profile,
                    expression_id,
                    argument_tree,
                    argument_symbols,
                )? {
                    let ty = types.insert_type_from_any(
                        Type::Reference {
                            symbol: enum_symbol,
                            static_arguments: None,
                        },
                        expression_id.into_any(),
                    );
                    StaticExpression::Type { ty }
                } else {
                    return Ok(());
                };

                evaluated = Some(StaticArgument::Evaluated {
                    name: argument_name,
                    value,
                });
                Ok(())
            },
        )?;

        Ok(evaluated)
    }

    /// Resolve the enum symbol for an enum member expression.
    fn enum_symbol_for_member_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let Expression::Member {
            name,
            static_arguments,
            left,
        } = tree.get(expression_id)
        else {
            return Ok(None);
        };
        if static_arguments.is_some() {
            return Ok(None);
        }

        let left_expression = tree.get(*left);
        let mut enum_symbol = match left_expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            _ => return Ok(None),
        };
        // unwrap import/export dependency items for enum symbols
        if enum_symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
            let dependency_id = symbol_entry.primary_declaration.and_then(|primary| {
                self.dependency_item_for_symbol(tree, primary, enum_symbol.local_id)
            });
            if let Some(dependency_id) = dependency_id
                && let DependencyItem::Local { target_symbol, .. }
                | DependencyItem::Remote { target_symbol, .. } = tree.get(dependency_id)
            {
                enum_symbol = *target_symbol;
            }
        }

        // ensure enum declarations are available before scanning enum fields
        if enum_symbol.module_id != module.id {
            self.require_analyze_module_declare(enum_symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
        }

        if self
            .enum_field_symbol_for_name(module, profile, enum_symbol, *name, tree, symbols)?
            .is_some()
        {
            Ok(Some(enum_symbol))
        } else {
            Ok(None)
        }
    }

    /// Convert a static argument into a type id for substitution.
    pub(crate) fn convert_static_argument_type(
        &self,
        argument: &StaticArgument,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // convert evaluated static expressions into type ids
        let convert_value = |value: &StaticExpression,
                             source_id: LocalNodeIdAny,
                             types: &mut TypeTable,
                             compiler: &Compiler| {
            match value {
                StaticExpression::Type { ty } => *ty,
                StaticExpression::TypeLiteral { value } => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: value.clone(),
                    },
                    source_id,
                ),
                StaticExpression::ScalarLiteral { value } => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value.clone()),
                    },
                    source_id,
                ),
                StaticExpression::ArrayExpression { elements }
                | StaticExpression::TupleExpression { elements } => {
                    let mut element_types = Vec::with_capacity(elements.len());
                    for element in elements {
                        let element_ty = compiler.convert_static_argument_type(
                            &StaticArgument::Evaluated {
                                name: None,
                                value: element.clone(),
                            },
                            source_id,
                            types,
                        );
                        element_types.push(TypeElement::new(element_ty));
                    }
                    types.insert_type_from_any(
                        Type::Tuple {
                            elements: element_types,
                            is_readonly: false,
                        },
                        source_id,
                    )
                }
                _ => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    source_id,
                ),
            }
        };

        match argument {
            StaticArgument::Evaluated { value, .. } => convert_value(value, source_id, types, self),
            StaticArgument::Unevaluated { .. } => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            ),
        }
    }

    /// Synthesize a missing static argument for function instantiation.
    pub(crate) fn synthesize_missing_static_argument_for_function(
        &self,
        tables: &mut InferTablesContext<'_>,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_parameter: &StaticParameter,
    ) -> AnalyzeResult<StaticArgument> {
        match static_parameter.kind {
            StaticParameterKind::Type => {
                // build a fresh inference variable for this call site
                let scope_owner = owner_symbol.unwrap_or(static_parameter.symbol);
                let scope = InferScope {
                    owner: scope_owner,
                    function_id: Some(node_id.into_global(tables.module.id)),
                };
                let var_id = tables
                    .infer
                    .new_var(InferOrigin::TypeParameter(static_parameter.symbol), scope);
                let inferred_ty_id = tables
                    .types
                    .insert_type_from_any(Type::InferVar { id: var_id }, node_id);
                tables.infer.bind_type(var_id, inferred_ty_id);

                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
            StaticParameterKind::Value => {
                self.error(AnalyzeError::MissingStaticArgument {
                    node: node_id
                        .into_global(tables.module.id)
                        .into_anchored(Some(tables.profile)),
                });
                let inferred_ty_id = tables.types.insert_type_from_any(Type::Error, node_id);
                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
        }
    }

    /// Infer a static argument from matching dynamic arguments.
    pub(crate) fn infer_static_argument_from_dynamic_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        dynamic_parameters: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // infer direct type-parameter arguments from positional dynamic arguments
        if static_parameter.kind == StaticParameterKind::Type {
            for (param_ty_id, argument_id) in
                dynamic_parameters.iter().zip(dynamic_arguments.iter())
            {
                let param_ty_id = self.unwrap_type_value(*param_ty_id, types);
                let Type::Reference {
                    symbol,
                    static_arguments: None,
                } = types.get_type(param_ty_id)
                else {
                    continue;
                };
                if *symbol != static_parameter.symbol {
                    continue;
                }

                let Some(argument_ty_id) = self.argument_type_for_static_inference(
                    module,
                    profile,
                    *argument_id,
                    tree,
                    symbols,
                    types,
                    infer,
                ) else {
                    continue;
                };
                if !self.inferred_type_argument_is_committable(
                    module,
                    profile,
                    symbols,
                    argument_ty_id,
                    types,
                ) {
                    continue;
                }

                return Ok(Some(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: argument_ty_id },
                }));
            }
        }

        // value inference from dynamic arguments is value-only
        if static_parameter.kind != StaticParameterKind::Value {
            return Ok(None);
        }
        if dynamic_parameters.len() != dynamic_arguments.len() {
            return Ok(None);
        }

        // scan positional arguments for direct static parameter references
        for (param_ty_id, argument_id) in dynamic_parameters.iter().zip(dynamic_arguments.iter()) {
            let param_ty_id = self.unwrap_type_value(*param_ty_id, types);
            let param_ty = types.get_type(param_ty_id).clone();

            // evaluate literal argument values when possible
            let expression_id = tree.get(*argument_id).value();
            let value = self.evaluate_static_expression_value(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                None,
            )?;
            let value = if let Some(value) = value {
                value
            } else if let Some(enum_symbol) = self.enum_symbol_for_member_expression(
                module,
                profile,
                expression_id,
                tree,
                symbols,
            )? {
                let ty = types.insert_type_from_any(
                    Type::Reference {
                        symbol: enum_symbol,
                        static_arguments: None,
                    },
                    expression_id.into_any(),
                );
                StaticExpression::Type { ty }
            } else {
                continue;
            };

            if !self.static_value_argument_is_static(&value, types) {
                continue;
            }

            // handle direct static parameter references
            if let Type::Reference { symbol, .. } = &param_ty
                && *symbol == static_parameter.symbol
            {
                return Ok(Some(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value,
                }));
            }

            // infer array sizes from literal arguments
            if let Type::ArraySized { count, .. } = &param_ty
                && let Some(target_symbol) = self.unwrap_type_value_symbol(types, *count)
                && target_symbol == static_parameter.symbol
            {
                let elements = match &value {
                    StaticExpression::ArrayExpression { elements }
                    | StaticExpression::TupleExpression { elements } => elements,
                    _ => continue,
                };
                let count_value = StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(elements.len() as i64),
                };
                return Ok(Some(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: count_value,
                }));
            }

            // infer array sizes when indexed access types reference the static parameter
            if let Type::Index { index, .. } = &param_ty
                && let Type::Reference { symbol, .. } = types.get_type(*index)
                && *symbol == static_parameter.symbol
            {
                let elements = match &value {
                    StaticExpression::ArrayExpression { elements }
                    | StaticExpression::TupleExpression { elements } => elements,
                    _ => continue,
                };
                let count_value = StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(elements.len() as i64),
                };
                return Ok(Some(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: count_value,
                }));
            }

            // fall through when the literal did not match the parameter shape
        }

        // infer from argument reference types that carry explicit static arguments
        for (param_ty_id, argument_id) in dynamic_parameters.iter().zip(dynamic_arguments.iter()) {
            let Some(argument_ty_id) = self.argument_type_for_static_inference(
                module,
                profile,
                *argument_id,
                tree,
                symbols,
                types,
                infer,
            ) else {
                continue;
            };

            if let Some(argument) = self.infer_static_argument_from_argument_type(
                module,
                profile,
                static_parameter,
                *param_ty_id,
                argument_ty_id,
                tree,
                symbols,
                types,
            )? {
                return Ok(Some(argument));
            }
        }

        Ok(None)
    }

    /// Return true when one inferred type argument is stable enough for static argument commitment.
    fn inferred_type_argument_is_committable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        argument_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        // reject unresolved convergence state
        if self.type_requires_infer_convergence(module, profile, argument_ty_id, symbols, types) {
            return false;
        }

        // reject explicit error placeholders
        !matches!(types.get_type(argument_ty_id), Type::Error)
    }

    /// Resolve a value static argument from argument type metadata.
    fn infer_static_argument_from_argument_type(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        param_ty_id: LocalTypeId,
        argument_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // unwrap any value wrappers before matching
        let param_ty_id = self.unwrap_type_value(param_ty_id, types);
        let argument_ty_id = self.unwrap_type_value(argument_ty_id, types);

        // extract reference arguments without holding immutable borrows
        let (param_symbol, param_arguments) = match types.get_type(param_ty_id) {
            Type::Reference {
                symbol,
                static_arguments: Some(arguments),
            } => (*symbol, arguments.clone()),
            _ => return Ok(None),
        };
        let (argument_symbol, argument_arguments) = match types.get_type(argument_ty_id) {
            Type::Reference {
                symbol,
                static_arguments: Some(arguments),
            } => (*symbol, arguments.clone()),
            _ => return Ok(None),
        };
        if param_symbol != argument_symbol {
            return Ok(None);
        }

        // materialize referenced static arguments with the owning module
        let resolved_arguments = if argument_symbol.module_id == module.id {
            self.materialize_static_arguments_for_reference(
                module,
                profile,
                argument_symbol,
                types.get_type_source(argument_ty_id),
                &argument_arguments,
                tree,
                symbols,
                types,
            )
        } else {
            let argument_module = self.program.modules.get(argument_symbol.module_id);
            let argument_module = argument_module.read();
            let argument_tree = argument_module.dir(profile).tree.read();
            let argument_symbols = argument_module.dir(profile).symbols.read();
            self.materialize_static_arguments_for_reference(
                &argument_module,
                profile,
                argument_symbol,
                types.get_type_source(argument_ty_id),
                &argument_arguments,
                &argument_tree,
                &argument_symbols,
                types,
            )
        };

        // map explicit static arguments when the parameter and argument share a reference
        for (index, param_argument) in param_arguments.iter().enumerate() {
            if !self.static_argument_references_symbol(
                module,
                profile,
                param_argument,
                static_parameter.symbol,
                tree,
                symbols,
                types,
            ) {
                continue;
            }

            let Some(argument) = resolved_arguments.get(index) else {
                continue;
            };
            let StaticArgument::Evaluated { value, .. } = argument else {
                continue;
            };
            if !self.static_value_argument_is_static(value, types) {
                continue;
            }

            return Ok(Some(StaticArgument::Evaluated {
                name: static_parameter.name,
                value: value.clone(),
            }));
        }

        Ok(None)
    }

    /// Resolve a candidate argument type for static value inference.
    fn argument_type_for_static_inference(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        infer: &InferTable,
    ) -> Option<LocalTypeId> {
        // prefer stable inferred types from this call site
        let argument = tree.get(argument_id);
        let value_id = argument.value();
        if let Some(type_id) = infer.inferred_type_for_node(value_id.into_global_any(module.id))
            && self.inferred_type_argument_is_committable(module, profile, symbols, type_id, types)
        {
            return Some(type_id);
        }

        // then use symbol value types for direct references
        if let Some(symbol) = tree.get(value_id).target_symbol() {
            if let Some(type_id) = types.get_type_id_for_symbol(symbols, symbol) {
                return Some(type_id);
            }
        }

        // fall back to declaration owned types for direct references
        if let Some(symbol) = tree.get(value_id).target_symbol()
            && symbol.module_id == module.id
        {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && let Some(type_id) = infer
                    .inferred_type_for_node(primary_declaration)
                    .or_else(|| types.get_declared_or_inferred_type_id(primary_declaration))
            {
                return Some(type_id);
            }
        }

        None
    }

    /// Check whether a static argument expression references a target symbol.
    fn static_argument_references_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        argument: &StaticArgument,
        target_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // accept evaluated references to the target symbol
        if let StaticArgument::Evaluated {
            value: StaticExpression::Type { ty },
            ..
        } = argument
            && let Type::Reference { symbol, .. } = types.get_type(*ty)
        {
            return *symbol == target_symbol;
        }

        // only unevaluated arguments carry expression nodes
        let StaticArgument::Unevaluated { node } = argument else {
            return false;
        };

        // resolve the owning tree before checking the argument expression
        let mut matches = false;
        let _ = self.with_static_argument_owner(
            profile,
            *node,
            module,
            tree,
            symbols,
            |owner_module, owner_tree, owner_symbols, argument_id| {
                let expression_id = owner_tree.get(argument_id).value();
                if let Some(symbol) = self.reference_symbol_for_expression(
                    owner_module,
                    expression_id,
                    profile,
                    owner_tree,
                    owner_symbols,
                ) {
                    matches = symbol == target_symbol;
                }
                Ok(())
            },
        );

        matches
    }

    /// Check whether a constraint satisfies a declared bound.
    fn constraint_satisfies_bound(
        &self,
        module: &Module,
        profile: ProfileId,
        expected_ty_id: LocalTypeId,
        constraint_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // skip validation when the declared bound still depends on static parameters
        let mut visited = HashSet::new();
        let expected_contains_static = self.type_contains_static_parameters(
            module,
            profile,
            expected_ty_id,
            symbols,
            types,
            &mut visited,
        );
        if expected_contains_static {
            return true;
        }

        // reject constraints that still depend on static parameters when the bound is concrete
        visited.clear();
        if self.type_contains_static_parameters(
            module,
            profile,
            constraint_ty_id,
            symbols,
            types,
            &mut visited,
        ) {
            return false;
        }

        // accept arguments whose constraints satisfy the expected bound
        self.is_type_assignable(
            module,
            profile,
            symbols,
            expected_ty_id,
            constraint_ty_id,
            types,
            options,
        )
        .is_assignable()
    }

    /// Evaluate a static default expression for a parameter.
    pub(crate) fn evaluate_static_default_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        parameter_kind: StaticParameterKind,
        name: Option<StringId>,
        default_expression: LocalNodeId<Expression>,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // prefer value defaults when type arguments stay unconverted
        if !treat_type_arguments_as_types
            && let Some(value) = self.evaluate_static_expression_value(
                module,
                profile,
                default_expression,
                tree,
                symbols,
                types,
                None,
            )?
        {
            return Ok(StaticArgument::Evaluated { name, value });
        }

        // evaluate default value based on the parameter kind
        let value = match parameter_kind {
            StaticParameterKind::Type => {
                let resolved = self.resolve_declared_type_expression_value(
                    module,
                    profile,
                    default_expression,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                    true,
                    true,
                )?;

                let resolved = match resolved {
                    Type::Unevaluated(_) => {
                        return Ok(StaticArgument::Evaluated {
                            name,
                            value: StaticExpression::Unevaluated {
                                node: default_expression,
                            },
                        });
                    }
                    resolved => resolved,
                };

                let ty_id = types.insert_type_from(resolved, default_expression);
                StaticExpression::Type { ty: ty_id }
            }
            StaticParameterKind::Value => {
                if let Some(value) = self.evaluate_static_expression_value(
                    module,
                    profile,
                    default_expression,
                    tree,
                    symbols,
                    types,
                    None,
                )? {
                    value
                } else if let Some(enum_symbol) = self.enum_symbol_for_member_expression(
                    module,
                    profile,
                    default_expression,
                    tree,
                    symbols,
                )? {
                    let ty = types.insert_type_from_any(
                        Type::Reference {
                            symbol: enum_symbol,
                            static_arguments: None,
                        },
                        default_expression.into_any(),
                    );
                    StaticExpression::Type { ty }
                } else {
                    StaticExpression::Unevaluated {
                        node: default_expression,
                    }
                }
            }
        };

        Ok(StaticArgument::Evaluated { name, value })
    }
}
