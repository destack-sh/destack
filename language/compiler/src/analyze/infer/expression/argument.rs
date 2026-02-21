use std::collections::{HashMap, HashSet};

use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, ContextualTypingMode, MaterializationMode,
    REWRITER_TAG_STATIC_ARGUMENT, TypeRewriteCache, TypeWalkContext, rewrite_type_with_cache,
};
use crate::analyze::StaticMemberSymbolKind;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    AnchoredGlobalNodeId, Argument, BindingKind, Constraint, Declaration, DependencyItem,
    DynamicKey, EnumFieldValue, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId,
    InferOrigin, InferScope, InferTable, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId,
    Mutability, NodeTree, ScalarLiteral, StaticArgument, StaticExpression, StaticKey,
    StaticParameter, StaticParameterKind, StaticProperty, StringId, SymbolTable, SymbolType, Type,
    TypeElement, TypeField, TypeLiteral, TypeMappedParameter, TypeRewriter, TypeRewriterOptions,
    TypeTable, rewrite_type,
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

/// Rewrite static arguments inside types for substitution.
struct StaticArgumentMaterializer<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The module that owns the arguments.
    argument_module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The tree that owns the arguments.
    argument_tree: &'a NodeTree,
    /// The symbols that own the arguments.
    argument_symbols: &'a SymbolTable,
    /// The materialization mode.
    mode: MaterializationMode,
    /// The cached materializations.
    cache: TypeRewriteCache,
    /// The cache key for rewrites.
    cache_key: u64,
    /// The rewriter options.
    rewrite_options: TypeRewriterOptions,
}

impl<'a> StaticArgumentMaterializer<'a> {
    /// Create a materializer for static arguments.
    fn new(
        compiler: &'a Compiler,
        argument_module: &'a Module,
        profile: ProfileId,
        argument_tree: &'a NodeTree,
        argument_symbols: &'a SymbolTable,
        mode: MaterializationMode,
        cache: TypeRewriteCache,
    ) -> Self {
        // derive rewrite options from the materialization mode
        let walk_context = TypeWalkContext::for_materialization(mode)
            .with_rewriter_tag(REWRITER_TAG_STATIC_ARGUMENT);
        let context_key = static_argument_context_key(argument_module, profile);
        let walk_context = walk_context.with_context_key(context_key);
        let rewrite_options = walk_context.rewriter_options();
        let cache_key = rewrite_options.cache_key();

        // seed the materializer state
        Self {
            compiler,
            argument_module,
            profile,
            argument_tree,
            argument_symbols,
            mode,
            cache,
            cache_key,
            rewrite_options,
        }
    }

    /// Return the internal cache.
    fn into_cache(self) -> TypeRewriteCache {
        self.cache
    }

    /// Rewrite a list of static arguments.
    fn rewrite_static_arguments(
        &mut self,
        types: &mut TypeTable,
        arguments: &[StaticArgument],
    ) -> (Vec<StaticArgument>, bool) {
        // rewrite each argument and track changes
        let mut changed = false;
        let mut mapped = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let mapped_argument = self.rewrite_static_argument(types, argument);
            if mapped_argument != *argument {
                changed = true;
            }
            mapped.push(mapped_argument);
        }
        (mapped, changed)
    }
}

/// Return a cache key for static argument materialization.
fn static_argument_context_key(argument_module: &Module, profile: ProfileId) -> u64 {
    // base module key
    // TODO #Architecture: include substitution context in static argument cache keys
    let module_id = argument_module.id;
    let module_key = module_id.package_id.raw() ^ ((module_id.local_id as u64) << 32);

    // profile key mix
    let profile_key = (profile.raw() as u64).rotate_left(17);

    module_key ^ profile_key
}

impl TypeRewriter for StaticArgumentMaterializer<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.rewrite_options
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, id: LocalTypeId) -> LocalTypeId {
        // return cached rewrites when available
        if let Some(mapped) = self.cache.get(&(self.cache_key, id)).copied() {
            return mapped;
        }

        // evaluate unevaluated types before rewriting
        if matches!(types.get_type(id), Type::Unevaluated(_)) {
            let _ = self.compiler.resolve_declared_type(
                self.argument_module,
                self.profile,
                id,
                self.argument_tree,
                self.argument_symbols,
                types,
            );
        }

        // stop when evaluation still yields an unevaluated type
        if matches!(types.get_type(id), Type::Unevaluated(_)) {
            self.cache.insert((self.cache_key, id), id);
            return id;
        }

        // rewrite using the cached walker
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, id);
        self.cache = cache;
        mapped
    }

    fn rewrite_type(&mut self, types: &mut TypeTable, id: LocalTypeId, ty: &Type) -> LocalTypeId {
        // allow non surface modes to rewrite immediately
        match self.mode {
            MaterializationMode::Surface => {}
            MaterializationMode::Shape | MaterializationMode::Validation => {
                return rewrite_type(self, types, id, ty);
            }
        }

        // only materialize references with static arguments
        let Type::Reference {
            symbol,
            static_arguments,
        } = ty
        else {
            return rewrite_type(self, types, id, ty);
        };

        // normalize to the type space symbol for the reference
        let symbol = self.compiler.normalize_reference_symbol_id(
            self.argument_module,
            self.profile,
            *symbol,
        );

        // skip when no static arguments exist
        let Some(static_arguments) = static_arguments.as_ref() else {
            return id;
        };

        // resolve static arguments in the reference owner module
        let source_id = types.get_type_source(id);
        let resolved_arguments = if symbol.module_id == self.argument_module.id {
            self.compiler.materialize_static_arguments_for_reference(
                self.argument_module,
                self.profile,
                symbol,
                source_id,
                static_arguments,
                self.argument_tree,
                self.argument_symbols,
                types,
            )
        } else {
            let reference_module = self.compiler.program.modules.get(symbol.module_id);
            let reference_module = reference_module.read();
            let reference_tree = reference_module.dir(self.profile).tree.read();
            let reference_symbols = reference_module.dir(self.profile).symbols.read();
            self.compiler.materialize_static_arguments_for_reference(
                &reference_module,
                self.profile,
                symbol,
                source_id,
                static_arguments,
                &reference_tree,
                &reference_symbols,
                types,
            )
        };

        // rewrite nested static arguments
        let (mapped_arguments, nested_changed) =
            self.rewrite_static_arguments(types, &resolved_arguments);
        let changed = nested_changed || resolved_arguments != *static_arguments;

        // return the original type when nothing changed
        if !changed {
            return id;
        }

        // return a rewritten reference type
        types.insert_type_from_type(
            Type::Reference {
                symbol,
                static_arguments: Some(mapped_arguments),
            },
            id,
        )
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a static parameter symbol for a reference expression.
    fn static_parameter_symbol_for_reference(
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
    fn with_static_argument_owner<T>(
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
        let reference = self
            .receiver_reference_for_inherited_arguments(receiver_ty, types)
            .or_else(|| {
                self.well_known_type(profile, receiver_ty, types)
                    .and_then(|reference_ty| match reference_ty {
                        Type::Reference {
                            symbol,
                            static_arguments,
                        } => Some((symbol, static_arguments)),
                        _ => None,
                    })
            });
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

        // recover declared reference arguments for direct receiver symbols when inference widened away static args
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
                self.receiver_reference_for_type_source(module, receiver_ty_id, types)
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
        types: &TypeTable,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)> {
        let source_id = types.get_type_source(receiver_ty_id);
        let source_ty_id =
            types.get_declared_or_inferred_type_id(source_id.into_global(module.id))?;
        let source_ty = types.get_type(source_ty_id);

        self.receiver_reference_for_inherited_arguments(source_ty, types)
    }

    /// Infer a dynamic argument value with contextual typing.
    pub(crate) fn infer_argument(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        expected_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let argument = tree.get(argument_id);

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
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Named { name: _, value, .. } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Labeled {
                label: _, value, ..
            } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Spread {
                label: _, value, ..
            } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
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
            self.with_module_tree_symbols_at_stage(
                module,
                profile,
                enum_symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let mut owner_types = owner_module.dir(profile).types.write();
                    let _ = self.enum_backing_type_for_symbol_in_tables(
                        owner_module,
                        profile,
                        enum_symbol,
                        &mut owner_types,
                    );
                    self.enum_literal_matches_symbol(
                        enum_symbol,
                        literal,
                        owner_tree,
                        owner_symbols,
                        &owner_types,
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

        self.resolve_type_reference_static_arguments_for_symbol(
            module,
            profile,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            options,
            tree,
            symbols,
            types,
        )
    }

    /// Resolve static arguments for a canonicalized type reference.
    pub(crate) fn resolve_type_reference_static_arguments_for_symbol(
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
        self.resolve_type_reference_static_arguments_for_symbol_with_bound_substitutions(
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
    pub(crate) fn resolve_type_reference_static_arguments_for_symbol_with_bound_substitutions(
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
            // resolve arguments with defaults and unknown synthesis
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

                // resolve the argument value or synthesize unknown when unresolved
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
                        // unresolved type references synthesize unknown or error by kind
                        let synthesized_value = match static_parameter.kind {
                            StaticParameterKind::Type => {
                                let unknown_ty_id = types.insert_type_from_any(
                                    Type::TypeLiteral {
                                        value: TypeLiteral::Unknown,
                                    },
                                    node_id,
                                );
                                StaticExpression::Type { ty: unknown_ty_id }
                            }
                            StaticParameterKind::Value => node_id
                                .try_into_typed::<Expression>()
                                .map(|expression_id| StaticExpression::Unevaluated {
                                    node: expression_id,
                                })
                                .unwrap_or(StaticExpression::TypeLiteral {
                                    value: TypeLiteral::Unknown,
                                }),
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
                        } => match types.get_type(*ty) {
                            Type::Reference { symbol, .. } => Some(*symbol),
                            _ => None,
                        },
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
            && let Some(ambient_symbols) =
                self.get_ambient_lib_symbol_sources_for_merge(profile, symbol_key, symbol_space)
        {
            let resolved_parameter_types = parameter_symbols
                .iter()
                .map(|symbol| substitutions.get(symbol).copied())
                .collect::<Vec<_>>();

            if resolved_parameter_types.iter().any(|ty| ty.is_some()) {
                for ambient_symbol in ambient_symbols {
                    if ambient_symbol == symbol {
                        continue;
                    }

                    let Some(other_parameters) = self.collect_static_parameter_symbols(
                        module,
                        ambient_symbol,
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
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_parameter: &StaticParameter,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        match static_parameter.kind {
            StaticParameterKind::Type => {
                // build a fresh inference variable for this call site
                let scope_owner = owner_symbol.unwrap_or(static_parameter.symbol);
                let scope = InferScope {
                    owner: scope_owner,
                    function_id: Some(node_id.into_global(module.id)),
                };
                let var_id =
                    infer.new_var(InferOrigin::TypeParameter(static_parameter.symbol), scope);
                let inferred_ty_id =
                    types.insert_type_from_any(Type::InferVar { id: var_id }, node_id);
                infer.bind_type(var_id, inferred_ty_id);

                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
            StaticParameterKind::Value => {
                self.error(AnalyzeError::MissingStaticArgument {
                    node: node_id.into_global(module.id).into_anchored(Some(profile)),
                });
                let inferred_ty_id = types.insert_type_from_any(Type::Error, node_id);
                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
        }
    }

    /// Infer a value static argument from matching dynamic arguments.
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
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // require a value parameter and matching argument list
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
            let Some(argument_ty_id) =
                self.argument_type_for_static_inference(module, *argument_id, tree, symbols, types)
            else {
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
        argument_id: LocalNodeId<Argument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer declared types for direct references
        let argument = tree.get(argument_id);
        let value_id = argument.value();
        if let Some(symbol) = tree.get(value_id).target_symbol()
            && symbol.module_id == module.id
        {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && let Some(type_id) = types.get_declared_or_inferred_type_id(primary_declaration)
            {
                return Some(type_id);
            }
        }

        // fall back to inferred types for the argument expression
        if let Some(type_id) = types.get_inferred_type_id(value_id.into_global_any(module.id)) {
            return Some(type_id);
        }

        // fall back to symbol types for direct references
        let symbol = tree.get(value_id).target_symbol()?;
        types.get_type_id_for_symbol(symbols, symbol)
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

    /// Substitute static parameter references in a type.
    pub(crate) fn substitute_static_parameters(
        &self,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_SUBSTITUTE);

        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }
        cache.insert(ty_id, ty_id);

        let ty = types.get_type(ty_id).clone();
        let mapped = match ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if let Some(mapped) = self.substitution_type_id_for_symbol(symbol, substitutions) {
                    mapped
                } else if let Some(static_arguments) = static_arguments {
                    let mut changed = false;
                    let mapped_arguments = static_arguments
                        .iter()
                        .map(|argument| {
                            let mapped = self.substitute_static_argument(
                                argument,
                                substitutions,
                                types,
                                cache,
                            );
                            if mapped != *argument {
                                changed = true;
                            }
                            mapped
                        })
                        .collect::<Vec<_>>();

                    if changed {
                        types.insert_type_from_type(
                            Type::Reference {
                                symbol,
                                static_arguments: Some(mapped_arguments),
                            },
                            ty_id,
                        )
                    } else {
                        ty_id
                    }
                } else {
                    ty_id
                }
            }
            Type::This => ty_id,
            Type::Value { value } => {
                let mapped_value =
                    self.substitute_static_parameters(value, substitutions, types, cache);
                if mapped_value == value {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Unary {
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Binary {
                            left: mapped_left,
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let distributive_union = match distributive_symbol {
                    Some(symbol) => match types.get_type(left).clone() {
                        Type::Reference {
                            symbol: reference_symbol,
                            static_arguments: None,
                        } if reference_symbol == symbol => {
                            if let Some(substitution) = substitutions.get(&symbol) {
                                let mut union_source = *substitution;
                                if let Type::Reference {
                                    symbol: union_symbol,
                                    static_arguments: None,
                                } = types.get_type(union_source)
                                    && union_symbol.ty() == SymbolType::TypeAlias
                                    && let Some(instance_id) =
                                        types.get_instance_type_id(*union_symbol)
                                {
                                    union_source = instance_id;
                                }

                                match types.get_type(union_source).clone() {
                                    Type::Union { elements } => Some((symbol, elements)),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    None => None,
                };

                if let Some((symbol, elements)) = distributive_union {
                    let mut branches = Vec::with_capacity(elements.len());
                    for element in elements {
                        let mut branch_substitutions = substitutions.clone();
                        branch_substitutions.insert(symbol, element);
                        let mut branch_cache = HashMap::new();
                        let mapped_left = self.substitute_static_parameters(
                            left,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_right = self.substitute_static_parameters(
                            right,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_then = self.substitute_static_parameters(
                            then_type,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_else = self.substitute_static_parameters(
                            else_type,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let branch_id = types.insert_type_from_type(
                            Type::Conditional {
                                distributive_symbol: None,
                                left: mapped_left,
                                right: mapped_right,
                                then_type: mapped_then,
                                else_type: mapped_else,
                            },
                            ty_id,
                        );
                        branches.push(branch_id);
                    }

                    types.insert_type_from_type(Type::Union { elements: branches }, ty_id)
                } else {
                    let mapped_left =
                        self.substitute_static_parameters(left, substitutions, types, cache);
                    let mut branch_substitutions = substitutions.clone();
                    if let Some(symbol) = distributive_symbol {
                        branch_substitutions.remove(&symbol);
                    }
                    let mapped_right = self.substitute_static_parameters(
                        right,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    let mapped_then = self.substitute_static_parameters(
                        then_type,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    let mapped_else = self.substitute_static_parameters(
                        else_type,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    if mapped_left == left
                        && mapped_right == right
                        && mapped_then == then_type
                        && mapped_else == else_type
                    {
                        ty_id
                    } else {
                        types.insert_type_from_type(
                            Type::Conditional {
                                distributive_symbol,
                                left: mapped_left,
                                right: mapped_right,
                                then_type: mapped_then,
                                else_type: mapped_else,
                            },
                            ty_id,
                        )
                    }
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.substitute_static_parameters(
                    parameter.constraint,
                    substitutions,
                    types,
                    cache,
                );
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.substitute_static_parameters(key_remap, substitutions, types, cache)
                });
                let mapped_value =
                    self.substitute_static_parameters(value, substitutions, types, cache);
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = TypeMappedParameter {
                        name: parameter.name,
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Index { left, index } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_index =
                    self.substitute_static_parameters(index, substitutions, types, cache);
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        ty_id,
                    )
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mut changed = false;
                let mapped_spans = spans
                    .iter()
                    .map(|span| {
                        let mapped =
                            self.substitute_static_parameters(*span, substitutions, types, cache);
                        if mapped != *span {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::TemplateLiteral {
                            strings,
                            spans: mapped_spans,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                let Some(static_arguments) = static_arguments else {
                    return ty_id;
                };

                let mut changed = false;
                let mapped_arguments = static_arguments
                    .iter()
                    .map(|argument| {
                        let mapped =
                            self.substitute_static_argument(argument, substitutions, types, cache);
                        if mapped != *argument {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();

                if changed {
                    types.insert_type_from_type(
                        Type::Import {
                            target,
                            qualifier,
                            static_arguments: Some(mapped_arguments),
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Infer { name, constraint } => {
                let mapped_constraint = constraint.map(|constraint| {
                    self.substitute_static_parameters(constraint, substitutions, types, cache)
                });
                if mapped_constraint == constraint {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Infer {
                            name,
                            constraint: mapped_constraint,
                        },
                        ty_id,
                    )
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target.map(|target| {
                    self.substitute_static_parameters(target, substitutions, types, cache)
                });
                if mapped_target == target {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        ty_id,
                    )
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                // substitute the array element type
                let mapped_element =
                    self.substitute_static_parameters(element, substitutions, types, cache);
                let mapped_count =
                    self.substitute_static_parameters(count, substitutions, types, cache);

                // reuse the existing type if substitutions were no-ops
                if mapped_element == element && mapped_count == count {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count: mapped_count,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element.map(|element| {
                    self.substitute_static_parameters(element, substitutions, types, cache)
                });
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mut did_change = false;
                let mapped_elements = elements
                    .into_iter()
                    .map(|element| {
                        let TypeElement {
                            label,
                            ty,
                            is_optional,
                            is_readonly,
                            is_rest,
                        } = element;
                        let mapped_ty =
                            self.substitute_static_parameters(ty, substitutions, types, cache);
                        if mapped_ty != ty {
                            did_change = true;
                        }
                        TypeElement {
                            label,
                            ty: mapped_ty,
                            is_optional,
                            is_readonly,
                            is_rest,
                        }
                    })
                    .collect::<Vec<_>>();
                if did_change {
                    types.insert_type_from_type(
                        Type::Tuple {
                            elements: mapped_elements,
                            is_readonly,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut changed = false;
                let mapped_fields = fields
                    .iter()
                    .map(|field| {
                        let mapped = self.substitute_static_parameters(
                            field.ty,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != field.ty {
                            changed = true;
                        }
                        TypeField {
                            key: field.key,
                            ty: mapped,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect::<Vec<_>>();
                let mapped_call_signatures = call_signatures
                    .iter()
                    .map(|signature| {
                        let mapped = self.substitute_static_parameters(
                            *signature,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_construct_signatures = construct_signatures
                    .iter()
                    .map(|signature| {
                        let mapped = self.substitute_static_parameters(
                            *signature,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_index_signatures = index_signatures
                    .iter()
                    .map(|signature| {
                        let mapped_key = self.substitute_static_parameters(
                            signature.key_type,
                            substitutions,
                            types,
                            cache,
                        );
                        let mapped_value = self.substitute_static_parameters(
                            signature.value_type,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped_key != signature.key_type || mapped_value != signature.value_type
                        {
                            changed = true;
                        }
                        let mut signature = signature.clone();
                        signature.key_type = mapped_key;
                        signature.value_type = mapped_value;
                        signature
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Object {
                            fields: mapped_fields,
                            call_signatures: mapped_call_signatures,
                            construct_signatures: mapped_construct_signatures,
                            index_signatures: mapped_index_signatures,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let mut changed = false;
                let mapped_this = this_parameter.map(|this_parameter| {
                    let mapped = self.substitute_static_parameters(
                        this_parameter,
                        substitutions,
                        types,
                        cache,
                    );
                    if mapped != this_parameter {
                        changed = true;
                    }
                    mapped
                });
                let mapped_parameters = dynamic_parameters
                    .iter()
                    .map(|parameter| {
                        let mapped = self.substitute_static_parameters(
                            *parameter,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_return = return_type.map(|return_type| {
                    let mapped =
                        self.substitute_static_parameters(return_type, substitutions, types, cache);
                    if mapped != return_type {
                        changed = true;
                    }
                    mapped
                });
                if changed {
                    types.insert_type_from_type(
                        Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters,
                            this_parameter: mapped_this,
                            dynamic_parameters: mapped_parameters,
                            return_type: mapped_return,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Union { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_static_parameters(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Intersection { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_static_parameters(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Error => ty_id,
        };

        cache.insert(ty_id, mapped);
        mapped
    }

    /// Look up one substitution for a symbol id, tolerating placeholder symbol kinds.
    fn substitution_type_id_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<LocalTypeId> {
        if let Some(type_id) = substitutions.get(&symbol) {
            return Some(*type_id);
        }

        substitutions.iter().find_map(|(candidate, type_id)| {
            if candidate.module_id == symbol.module_id
                && candidate.local_id.id == symbol.local_id.id
            {
                Some(*type_id)
            } else {
                None
            }
        })
    }

    /// Resolve one projection receiver reference from the projection source expression.
    fn projection_receiver_reference_from_type_source(
        &self,
        module: &Module,
        profile: ProfileId,
        projection_source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
        let Ok(mut projection_expression_id) = projection_source_id.try_into_typed::<Expression>()
        else {
            return Ok(None);
        };
        if !tree.has_node_id(projection_expression_id.id) {
            return Ok(None);
        }

        if let Expression::Instantiation { left, .. } = tree.get(projection_expression_id) {
            projection_expression_id = *left;
        }
        let Expression::Member { left, .. } = tree.get(projection_expression_id) else {
            return Ok(None);
        };

        let receiver_expression_id = self.unwrap_parenthesized_expression(*left, tree);
        match tree.get(receiver_expression_id) {
            Expression::Instantiation {
                left,
                static_arguments,
            } => {
                let receiver_expression_id = self.unwrap_parenthesized_expression(*left, tree);
                let receiver_symbol = self
                    .reference_symbol_for_expression(
                        module,
                        receiver_expression_id,
                        profile,
                        tree,
                        symbols,
                    )
                    .or_else(|| tree.get(receiver_expression_id).target_symbol());
                let Some(receiver_symbol) = receiver_symbol else {
                    return Ok(None);
                };

                let receiver_arguments = self
                    .evaluate_static_arguments(
                        module,
                        profile,
                        Some(static_arguments.as_slice()),
                        tree,
                        symbols,
                        types,
                    )?
                    .unwrap_or_default();
                Ok(Some((receiver_symbol, receiver_arguments)))
            }
            _ => {
                let receiver_symbol = self
                    .reference_symbol_for_expression(
                        module,
                        receiver_expression_id,
                        profile,
                        tree,
                        symbols,
                    )
                    .or_else(|| tree.get(receiver_expression_id).target_symbol());
                Ok(receiver_symbol.map(|receiver_symbol| (receiver_symbol, Vec::new())))
            }
        }
    }

    /// Resolve one projection receiver reference from static-parameter substitutions and owner constraints.
    fn projection_receiver_reference_from_owner_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        let mut candidates = Vec::<(GlobalSymbolId, Vec<StaticArgument>)>::new();

        for (parameter_symbol, substitution_type_id) in substitutions {
            if !self.symbol_is_static_parameter(module, profile, *parameter_symbol, symbols, types)
            {
                continue;
            }

            let Some(constraint_type_id) = self.static_parameter_constraint_type(
                module,
                profile,
                *parameter_symbol,
                source_id,
                symbols,
                types,
            ) else {
                continue;
            };
            let constraint_symbol = self
                .unwrap_type_symbol(types, constraint_type_id)
                .map(|(symbol, _, _)| symbol)
                .or_else(|| match types.get_type(constraint_type_id) {
                    Type::Intersection { elements } | Type::Union { elements } => {
                        elements.iter().find_map(|element_id| {
                            self.unwrap_type_symbol(types, *element_id)
                                .map(|(symbol, _, _)| symbol)
                        })
                    }
                    _ => None,
                });
            let Some(constraint_symbol) = constraint_symbol else {
                continue;
            };
            let constraint_symbol = self
                .declaration_symbol_id(module, symbols, profile, constraint_symbol)
                .unwrap_or(constraint_symbol);
            if constraint_symbol != owner_symbol {
                continue;
            }

            let substitution_type_id = types.unwrap_value_type_id(*substitution_type_id);
            let Some((receiver_symbol, receiver_arguments, _)) =
                self.unwrap_type_symbol(types, substitution_type_id)
            else {
                continue;
            };
            let candidate = (receiver_symbol, receiver_arguments.unwrap_or_default());
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }

        if candidates.len() == 1 {
            return candidates.pop();
        }

        None
    }

    /// Materialize static arguments inside type references for substitution.
    pub(crate) fn materialize_static_arguments_in_type(
        &self,
        argument_module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        argument_tree: &NodeTree,
        argument_symbols: &SymbolTable,
        types: &mut TypeTable,
        cache: &mut TypeRewriteCache,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_MATERIALIZE);

        let local_cache = std::mem::take(cache);
        let mut materializer = StaticArgumentMaterializer::new(
            self,
            argument_module,
            profile,
            argument_tree,
            argument_symbols,
            MaterializationMode::Surface,
            local_cache,
        );
        let mapped = materializer.rewrite_type_id(types, ty_id);
        *cache = materializer.into_cache();
        mapped
    }

    /// Instantiate one signature type by materializing, substituting, and rewriting projections.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn instantiate_signature_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        materialize_cache: &mut TypeRewriteCache,
        substitute_cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        self.instantiate_type_with_substitutions(
            module,
            profile,
            source_id,
            owner_symbol,
            ty_id,
            substitutions,
            tree,
            symbols,
            types,
            materialize_cache,
            substitute_cache,
        )
    }

    /// Instantiate one type with a substitution environment, then normalize projections.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn instantiate_type_with_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        materialize_cache: &mut TypeRewriteCache,
        substitute_cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        // materialize source-level static arguments before substitution
        let materialized = self.materialize_static_arguments_in_type(
            module,
            profile,
            ty_id,
            tree,
            symbols,
            types,
            materialize_cache,
        );

        // apply static substitutions
        let substituted = if substitutions.is_empty() {
            materialized
        } else {
            self.substitute_static_parameters(materialized, substitutions, types, substitute_cache)
        };

        // normalize substituted static arguments
        let mut normalized = self.materialize_static_arguments_in_type(
            module,
            profile,
            substituted,
            tree,
            symbols,
            types,
            materialize_cache,
        );

        // resolve associated projections after substitution
        if let Some(projected) = self.instantiate_substituted_projection_type_from_source(
            module,
            profile,
            normalized,
            substitutions,
            tree,
            symbols,
            types,
        ) {
            normalized = projected;
        }

        // rewrite owner-scoped associated aliases
        if let Some(owner_symbol) = owner_symbol {
            normalized = self.rewrite_associated_aliases_for_owner(
                module,
                profile,
                source_id,
                owner_symbol,
                substitutions,
                normalized,
                tree,
                symbols,
                types,
            );
        }

        // substitute again after owner alias rewrites: alias materialization can expose static parameters
        if !substitutions.is_empty() {
            normalized = self.substitute_static_parameters(
                normalized,
                substitutions,
                types,
                substitute_cache,
            );
        }

        // rematerialize projections exposed by the second substitution pass
        if let Some(projected) = self.instantiate_substituted_projection_type_from_source(
            module,
            profile,
            normalized,
            substitutions,
            tree,
            symbols,
            types,
        ) {
            normalized = projected;
        }

        self.materialize_static_arguments_in_type(
            module,
            profile,
            normalized,
            tree,
            symbols,
            types,
            materialize_cache,
        )
    }

    /// Materialize one substituted associated projection from its source member expression.
    pub(crate) fn instantiate_substituted_projection_type_from_source(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // only projection-like references can be concretized in this pass
        let (projected_symbol, projected_arguments, projection_source_id) =
            self.unwrap_type_symbol(types, ty_id)?;
        if self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                projected_symbol,
                tree,
                symbols,
            )
            .ok()?
            != Some(StaticMemberSymbolKind::AssociatedType)
        {
            return None;
        }
        let projected_symbol = self
            .declaration_symbol_id(module, symbols, profile, projected_symbol)
            .unwrap_or(projected_symbol);
        let owner_symbol =
            self.owner_symbol_for_member_symbol(module, profile, projected_symbol, symbols);
        let owner_symbol = owner_symbol.map(|owner_symbol| {
            self.declaration_symbol_id(module, symbols, profile, owner_symbol)
                .unwrap_or(owner_symbol)
        });
        let projected_member_key = self
            .symbol_name_for_global(module, profile, projected_symbol)
            .map(StaticKey::Name);
        let Some(projected_member_key) = projected_member_key else {
            return None;
        };

        // use projected reference arguments directly
        let source_id = projection_source_id;
        let explicit_member_arguments = projected_arguments.clone();
        let projected_member_type = Type::Reference {
            symbol: projected_symbol,
            static_arguments: explicit_member_arguments.clone(),
        };

        // first materialize the projected member directly if it already resolves to a concrete alias
        if let Ok(direct_materialized_type) = self.materialize_associated_member_projection(
            module,
            profile,
            source_id,
            projected_symbol,
            None,
            &[],
            explicit_member_arguments.as_deref(),
            projected_member_type.clone(),
            tree,
            symbols,
            types,
        ) {
            if direct_materialized_type != projected_member_type {
                return Some(types.insert_type_from_any(direct_materialized_type, source_id));
            }
        }

        // resolve one projection receiver from source syntax and substitutions
        let receiver_from_source = self
            .projection_receiver_reference_from_type_source(
                module,
                profile,
                projection_source_id,
                tree,
                symbols,
                types,
            )
            .ok()
            .flatten();
        let receiver_from_owner = owner_symbol.and_then(|owner_symbol| {
            self.projection_receiver_reference_from_owner_substitutions(
                module,
                profile,
                source_id,
                owner_symbol,
                substitutions,
                symbols,
                types,
            )
        });
        let Some((projection_receiver_symbol, projection_receiver_arguments)) =
            receiver_from_source.or(receiver_from_owner)
        else {
            return None;
        };
        let receiver_substitution =
            self.substitution_type_id_for_symbol(projection_receiver_symbol, substitutions);
        let (mut receiver_symbol, receiver_arguments) =
            if let Some(receiver_substitution) = receiver_substitution {
                let receiver_substitution = types.unwrap_value_type_id(receiver_substitution);
                let Some((receiver_symbol, receiver_arguments, _)) =
                    self.unwrap_type_symbol(types, receiver_substitution)
                else {
                    return None;
                };
                (receiver_symbol, receiver_arguments.unwrap_or_default())
            } else {
                (projection_receiver_symbol, projection_receiver_arguments)
            };

        receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        receiver_symbol = self
            .declaration_symbol_id(module, symbols, profile, receiver_symbol)
            .unwrap_or(receiver_symbol);
        let target_symbol = self
            .resolve_associated_member_symbol_for_receiver(
                module,
                profile,
                receiver_symbol,
                projected_member_key,
                StaticMemberSymbolKind::AssociatedType,
                tree,
                symbols,
            )
            .ok()??;
        let member_type = Type::Reference {
            symbol: target_symbol,
            static_arguments: explicit_member_arguments.clone(),
        };
        let projected_type = self
            .materialize_associated_member_projection(
                module,
                profile,
                source_id,
                target_symbol,
                Some(receiver_symbol),
                &receiver_arguments,
                explicit_member_arguments.as_deref(),
                member_type,
                tree,
                symbols,
                types,
            )
            .ok()?;

        Some(types.insert_type_from_any(projected_type, source_id))
    }

    pub(crate) fn materialize_static_arguments_for_reference(
        &self,
        argument_module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        static_arguments: &[StaticArgument],
        argument_tree: &NodeTree,
        argument_symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Vec<StaticArgument> {
        // collect parameter symbols for the reference
        let Some(parameter_symbols) = self.collect_static_parameter_symbols(
            argument_module,
            symbol,
            profile,
            argument_tree,
            argument_symbols,
            types,
        ) else {
            return static_arguments.to_vec();
        };
        if parameter_symbols.is_empty() {
            return static_arguments.to_vec();
        }

        // select a source node for parameter inference
        let source_id = static_arguments
            .iter()
            .find_map(|argument| match argument {
                StaticArgument::Unevaluated { node }
                    if node.module_id == argument_module.id
                        && argument_tree.has_node_id(node.local_id.id) =>
                {
                    Some(node.local_id)
                }
                _ => None,
            })
            .unwrap_or(source_id);

        // map parameter names to their resolved kinds
        let mut parameter_kinds = Vec::with_capacity(parameter_symbols.len());
        let mut parameter_name_kinds = HashMap::new();
        for parameter_symbol in parameter_symbols {
            let parameter = self.resolve_static_parameter(
                argument_module,
                parameter_symbol,
                source_id,
                profile,
                argument_tree,
                argument_symbols,
                types,
            );
            let kind = parameter.kind;
            if let Some(name) = parameter.name {
                parameter_name_kinds.insert(name, kind);
            }
            parameter_kinds.push(kind);
        }

        // evaluate arguments based on the referenced parameter kinds
        let mut resolved_arguments = Vec::with_capacity(static_arguments.len());
        for (index, argument) in static_arguments.iter().enumerate() {
            let (argument_name, argument_node) = match argument {
                StaticArgument::Evaluated { name, .. } => (*name, None),
                StaticArgument::Unevaluated { node } => {
                    let mut name = None;
                    let _ = self.with_static_argument_owner(
                        profile,
                        *node,
                        argument_module,
                        argument_tree,
                        argument_symbols,
                        |_, owner_tree, _, argument_id| {
                            let argument_node = owner_tree.get(argument_id);
                            name = match argument_node {
                                Argument::Named { name, .. } => Some(*name),
                                _ => None,
                            };
                            Ok(())
                        },
                    );
                    (name, Some(*node))
                }
            };

            let parameter_kind = argument_name
                .and_then(|name| parameter_name_kinds.get(&name).copied())
                .or_else(|| parameter_kinds.get(index).copied());

            let Some(parameter_kind) = parameter_kind else {
                resolved_arguments.push(argument.clone());
                continue;
            };

            let Some(argument_node) = argument_node else {
                let resolved = if parameter_kind == StaticParameterKind::Value {
                    self.normalize_value_static_argument(argument.clone(), types)
                } else {
                    argument.clone()
                };
                resolved_arguments.push(resolved);
                continue;
            };

            let mut evaluated = None;
            let mut evaluated_name = argument_name;
            let _ = self.with_static_argument_owner(
                profile,
                argument_node,
                argument_module,
                argument_tree,
                argument_symbols,
                |owner_module, owner_tree, owner_symbols, argument_id| {
                    let argument = owner_tree.get(argument_id);
                    evaluated_name = match argument {
                        Argument::Named { name, .. } => Some(*name),
                        _ => None,
                    };
                    let expression_id = argument.value();
                    evaluated = match parameter_kind {
                        StaticParameterKind::Type => {
                            // preserve static parameter references during materialization
                            if let Some(parameter_symbol) = self
                                .static_parameter_symbol_for_reference(
                                    owner_module,
                                    profile,
                                    expression_id,
                                    owner_tree,
                                    owner_symbols,
                                    types,
                                )?
                            {
                                let reference_ty = Type::Reference {
                                    symbol: parameter_symbol,
                                    static_arguments: None,
                                };
                                let ty_id = types
                                    .insert_type_from_any(reference_ty, expression_id.into_any());
                                Some(StaticExpression::Type { ty: ty_id })
                            } else if let Ok(ty_id) = self.resolve_declared_type_expression(
                                owner_module,
                                profile,
                                expression_id,
                                owner_tree,
                                owner_symbols,
                                types,
                                false,
                                true,
                            ) && !matches!(types.get_type(ty_id), Type::Unevaluated(_))
                            {
                                Some(StaticExpression::Type { ty: ty_id })
                            } else {
                                None
                            }
                        }
                        StaticParameterKind::Value => self
                            .evaluate_static_expression_value(
                                owner_module,
                                profile,
                                expression_id,
                                owner_tree,
                                owner_symbols,
                                types,
                                None,
                            )
                            .ok()
                            .flatten(),
                    };
                    Ok(())
                },
            );

            let resolved = if let Some(value) = evaluated {
                StaticArgument::Evaluated {
                    name: evaluated_name,
                    value,
                }
            } else {
                StaticArgument::Unevaluated {
                    node: argument_node,
                }
            };
            let resolved = if parameter_kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved, types)
            } else {
                resolved
            };
            resolved_arguments.push(resolved);
        }

        resolved_arguments
    }

    /// Substitute static parameters in a static argument.
    pub(crate) fn substitute_static_argument(
        &self,
        argument: &StaticArgument,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Substitute static parameters in a static expression.
    pub(crate) fn substitute_static_expression(
        &self,
        expression: &StaticExpression,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => StaticExpression::Type {
                ty: self.substitute_static_parameters(*ty, substitutions, types, cache),
            },
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                let mapped_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_static_argument(argument, substitutions, types, cache)
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: mapped_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_static_expression(element, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_static_expression(element, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.substitute_static_property(property, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Substitute static parameters in a static property.
    pub(crate) fn substitute_static_property(
        &self,
        property: &StaticProperty,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                let mapped_default = default.as_ref().map(|default| {
                    self.substitute_static_expression(default, substitutions, types, cache)
                });
                StaticProperty::Field {
                    modifiers: *modifiers,
                    key: *key,
                    value: mapped_value,
                    default: mapped_default,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body =
                    self.substitute_static_expression(body, substitutions, types, cache);
                StaticProperty::Method {
                    modifiers: *modifiers,
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
        }
    }
}
