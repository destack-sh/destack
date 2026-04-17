use std::collections::{HashMap, HashSet};

use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{
    CanonicalSymbolMode, ContextualTypingMode, InferContext, SymbolTypeView, TreeSymbolView,
    TypeContext, TypeView,
};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, CompilerContext,
    InferState,
};
use destack_dir::{
    AnchoredGlobalNodeId, Argument, Constraint, Declaration, DependencyItem, Expression, Freshness,
    GenericArgument, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, InferOrigin, InferScope,
    InferTable, Key, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId, Name, NodeTree,
    ScalarLiteral, StaticArgument, StaticExpression, StaticKey, StaticParameter,
    StaticParameterKind, StaticProperty, StringId, SymbolTable, SymbolType, Type, TypeElement,
    TypeExpression, TypeField, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

/// Inherited static arguments and substitutions for a type reference.
#[derive(Debug, Clone)]
pub(crate) struct InheritedStaticArguments {
    /// Static arguments inherited from the receiver.
    pub(crate) arguments: Vec<StaticArgument>,
    /// Substitutions for type parameters in inherited arguments.
    pub(crate) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// The validation mode for resolving static type arguments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StaticArgumentValidationMode {
    /// Full analyze-time validation against concrete apparent types.
    Analyze,
    /// Declare-time validation over declared structural facts only.
    Declare,
}

impl StaticArgumentValidationMode {
    /// Return true when validation should eagerly prepare concrete instance types.
    fn uses_eager_instance_types(self) -> bool {
        matches!(self, Self::Analyze)
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Run logic with one declared-owner type context for a remote module.
    fn with_declared_type_context_for_module<T>(
        &self,
        ctx: &mut TypeContext<'_>,
        module_id: ModuleId,
        handle: impl FnOnce(&mut TypeContext<'_>) -> AnalyzeResult<T>,
    ) -> AnalyzeResult<T> {
        // local declared reads should reuse the current context directly
        if module_id == ctx.module.id {
            let mut ctx = ctx.reborrow();
            return handle(&mut ctx);
        }

        let owner_module = ctx.compiler_context.module(module_id);
        let owner_module = owner_module.as_ref();
        let owner_options = ctx
            .compiler_context
            .analyze_context_options_for_module(owner_module.id);

        let owner_dir = self
            .require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        let mut owner_ctx = TypeContext::new(
            ctx.compiler_context,
            owner_module,
            ctx.profile,
            &owner_options,
            &owner_dir.tree,
            &owner_dir.symbols,
            ctx.types,
            ctx.index.clone(),
        );
        handle(&mut owner_ctx)
    }

    /// Resolve a static parameter symbol for a reference expression.
    pub(crate) fn static_parameter_symbol_for_reference(
        &self,
        ctx: TypeView<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = ctx.tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_symbols_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            target_symbol.module_id,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_resolved,
            |owner_module, owner_symbols| {
                if self.symbol_is_static_parameter(
                    SymbolTypeView::new(
                        ctx.compiler_context,
                        owner_module,
                        ctx.profile,
                        owner_symbols,
                        ctx.types,
                    ),
                    *target_symbol,
                ) {
                    Some(*target_symbol)
                } else {
                    None
                }
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Run logic with the module and tree that own a static argument node in read-only mode.
    fn with_static_argument_owner_read<T>(
        &self,
        view: TreeSymbolView<'_>,
        argument_node: GlobalNodeIdAny,
        f: impl FnOnce(TreeSymbolView<'_>, LocalNodeId<Argument>) -> AnalyzeResult<T>,
    ) -> AnalyzeResult<Option<T>> {
        // static arguments should always point at argument nodes
        let argument_id = match argument_node.try_into_local_typed::<Argument>() {
            Ok(argument_id) => argument_id,
            Err(_) => {
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: argument_node.into_anchored(Some(view.profile)),
                    message: "static argument does not resolve to an argument node".to_string(),
                });
                return Ok(None);
            }
        };

        // prefer the call site tree when it owns the argument node
        if argument_node.module_id == view.module.id
            && view.tree.has_node_id(argument_node.local_id.id)
        {
            return f(view, argument_id).map(Some);
        }

        // otherwise, resolve the owning module and ensure the node exists there
        let argument_snapshot = self.require_artifact_dir_declared(
            view.compiler_context.revision(),
            argument_node.module_id,
            view.profile,
        )?;
        if !argument_snapshot
            .tree
            .has_node_id(argument_node.local_id.id)
        {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: argument_node.into_anchored(Some(view.profile)),
                message: "static argument node is missing".to_string(),
            });
            return Ok(None);
        }

        let argument_module = view.compiler_context.module(argument_node.module_id);
        let argument_module = argument_module.as_ref();
        let view = TreeSymbolView::new(
            view.compiler_context,
            argument_module,
            view.profile,
            &argument_snapshot.tree,
            &argument_snapshot.symbols,
        );
        f(view, argument_id).map(Some)
    }

    // static argument owner resolution: mutable
    /// Run logic with the module and tree that own a static argument node.
    pub(crate) fn with_static_argument_owner<T>(
        &self,
        ctx: &mut TypeContext<'_>,
        argument_node: GlobalNodeIdAny,
        f: impl FnOnce(&mut TypeContext<'_>, LocalNodeId<Argument>) -> AnalyzeResult<T>,
    ) -> AnalyzeResult<Option<T>> {
        // static arguments should always point at argument nodes
        let argument_id = match argument_node.try_into_local_typed::<Argument>() {
            Ok(argument_id) => argument_id,
            Err(_) => {
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: argument_node.into_anchored(Some(ctx.profile)),
                    message: "static argument does not resolve to an argument node".to_string(),
                });
                return Ok(None);
            }
        };

        // prefer the call site tree when it owns the argument node
        if argument_node.module_id == ctx.module.id
            && ctx.tree.has_node_id(argument_node.local_id.id)
        {
            let mut ctx = ctx.reborrow();
            return f(&mut ctx, argument_id).map(Some);
        }

        // otherwise, resolve the owning module and ensure the node exists there
        let argument_snapshot = self.require_artifact_dir_resolved(
            ctx.compiler_context.revision(),
            argument_node.module_id,
            ctx.profile,
        )?;
        if !argument_snapshot
            .tree
            .has_node_id(argument_node.local_id.id)
        {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: argument_node.into_anchored(Some(ctx.profile)),
                message: "static argument node is missing".to_string(),
            });
            return Ok(None);
        }

        let argument_module = ctx.compiler_context.module(argument_node.module_id);
        let argument_module = argument_module.as_ref();
        let argument_options = ctx
            .compiler_context
            .analyze_context_options_for_module(argument_module.id);
        let mut ctx = TypeContext::new(
            ctx.compiler_context,
            argument_module,
            ctx.profile,
            &argument_options,
            &argument_snapshot.tree,
            &argument_snapshot.symbols,
            ctx.types,
            ctx.index.clone(),
        );
        f(&mut ctx, argument_id).map(Some)
    }

    /// Map static argument values to parameters by name and position.
    pub(crate) fn assign_static_argument_values(
        &self,
        call_site: TreeSymbolView<'_>,
        node_id: LocalNodeIdAny,
        static_arguments: &[StaticArgument],
        parameters: &[StaticParameter],
    ) -> Vec<Option<StaticArgument>> {
        // precompute argument names and detect mixed styles
        let mut has_named_arguments = false;
        let mut argument_infos = Vec::with_capacity(static_arguments.len());
        for argument in static_arguments {
            let (argument_name, is_spread) = match argument {
                StaticArgument::Evaluated { name, .. } => (*name, false),
                StaticArgument::Unevaluated { node } => {
                    let info = self.with_static_argument_owner_read(
                        call_site,
                        *node,
                        |view, argument_id| {
                            let argument = view.tree.get(argument_id);
                            Ok(match argument {
                                Argument::Named { name, .. } => {
                                    has_named_arguments = true;
                                    (Some(name.string()), false)
                                }
                                Argument::Spread { .. } => (None, true),
                                _ => (None, false),
                            })
                        },
                    );
                    info.ok().flatten().unwrap_or((None, false))
                }
            };

            // reject spread arguments
            if is_spread {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => *node,
                    _ => node_id.into_global(call_site.module.id),
                };
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: error_node.into_anchored(Some(call_site.profile)),
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
                    .into_global(call_site.module.id)
                    .into_anchored(Some(call_site.profile)),
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
                    _ => node_id.into_global(call_site.module.id),
                };
                let message = if argument_name.is_some() {
                    "unknown static argument name".to_string()
                } else {
                    "too many static arguments".to_string()
                };
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: error_node.into_anchored(Some(call_site.profile)),
                    message,
                });
                continue;
            };

            // reject duplicate assignments
            if assigned[target_index].is_some() {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => *node,
                    _ => node_id.into_global(call_site.module.id),
                };
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: error_node.into_anchored(Some(call_site.profile)),
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
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeIdAny,
        receiver_ty_id: Option<LocalTypeId>,
        receiver_ty: &Type,
    ) -> AnalyzeResult<InheritedStaticArguments> {
        // resolve the receiver into a symbol and static arguments
        let reference = if let Some(reference) =
            self.receiver_reference_for_inherited_arguments(receiver_ty, ctx.types)
        {
            Some(reference)
        } else {
            match self.well_known_type(ctx.profile, receiver_ty, ctx.types) {
                Some(Type::Reference {
                    symbol,
                    static_arguments,
                }) => {
                    let symbol = self.remap_typevalue_symbol_to_type_space(
                        ctx.module_symbol_view(),
                        &ctx.index,
                        symbol,
                    )?;
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
                        self.static_value_argument_is_static(value, ctx.type_view())
                    }
                    StaticArgument::Unevaluated { .. } => false,
                })
            };

        // re-read declared reference arguments when inference widened away static arguments
        if !has_usable_reference_arguments(&reference)
            && let Some(declared_reference) =
                self.receiver_reference_for_declaration_symbol(ctx, receiver_id)
        {
            reference = Some(declared_reference);
        }
        if !has_usable_reference_arguments(&reference)
            && let Some(receiver_ty_id) = receiver_ty_id
            && let Some(source_reference) =
                self.receiver_reference_for_type_source(ctx, receiver_ty_id)
        {
            reference = Some(source_reference);
        }

        // fall back to instance arguments when the receiver type does not preserve them
        let instance_reference = || {
            // prefer instances registered on the receiver expression
            let receiver_global_id = receiver_id.into_global(ctx.module.id);
            if let Some(instance_reference) = self.query_instance_symbol_arguments_for_node_infer(
                receiver_global_id,
                ctx.infer,
                ctx.types,
            ) {
                return Some(instance_reference);
            }

            // fall back to the source node for the inferred type
            if let Some(receiver_ty_id) = receiver_ty_id {
                let source_id = ctx.types.get_type_source(receiver_ty_id);
                let source_global_id = source_id.into_global(ctx.module.id);
                if let Some(instance_reference) = self
                    .query_instance_symbol_arguments_for_node_infer(
                        source_global_id,
                        ctx.infer,
                        ctx.types,
                    )
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
                StaticArgument::Unevaluated { node } if node.module_id == ctx.module.id => {
                    Some(node.local_id)
                }
                _ => None,
            })
        });
        let resolution_node_id = argument_node
            .or_else(|| receiver_ty_id.map(|ty_id| ctx.types.get_type_source(ty_id)))
            .unwrap_or(receiver_id);

        // resolve static arguments for the type reference
        let resolved: Option<Vec<StaticArgument>> = self.resolve_type_reference_static_arguments(
            &mut ctx.type_context_reborrow(),
            resolution_node_id,
            symbol,
            static_arguments.as_deref(),
            true,
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

                if symbol.module_id == ctx.module.id {
                    self.materialize_static_arguments_for_reference(
                        &mut ctx.type_context_reborrow(),
                        symbol,
                        resolution_node_id,
                        static_arguments,
                    )
                } else {
                    self.with_declared_type_context_for_module(
                        &mut ctx.type_context_reborrow(),
                        symbol.module_id,
                        |remote_ctx| {
                            Ok(self.materialize_static_arguments_for_reference(
                                remote_ctx,
                                symbol,
                                resolution_node_id,
                                static_arguments,
                            ))
                        },
                    )?
                }
            }
        };

        if resolved_arguments
            .iter()
            .any(|argument| matches!(argument, StaticArgument::Unevaluated { .. }))
        {
            resolved_arguments = if symbol.module_id == ctx.module.id {
                self.materialize_static_arguments_for_reference(
                    &mut ctx.type_context_reborrow(),
                    symbol,
                    receiver_id,
                    &resolved_arguments,
                )
            } else {
                self.with_declared_type_context_for_module(
                    &mut ctx.type_context_reborrow(),
                    symbol.module_id,
                    |remote_ctx| {
                        Ok(self.materialize_static_arguments_for_reference(
                            remote_ctx,
                            symbol,
                            receiver_id,
                            &resolved_arguments,
                        ))
                    },
                )?
            };
        }

        // build type parameter substitutions
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.type_context_reborrow(),
            symbol,
            resolution_node_id,
            &resolved_arguments,
        );

        Ok(InheritedStaticArguments {
            arguments: resolved_arguments,
            substitutions,
        })
    }

    /// Infer static arguments for a generic return type from an expected return type.
    pub(crate) fn static_arguments_from_expected_return_type(
        &self,
        ctx: &mut TypeContext<'_>,
        return_type: LocalTypeId,
        expected_return_type: LocalTypeId,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, StaticArgument>>> {
        // unwrap type value wrappers
        let return_type = self.unwrap_type_value(return_type, ctx.types);
        let expected_return_type = self.unwrap_type_value(expected_return_type, ctx.types);

        // extract reference metadata from the return types
        let (return_symbol, mut return_arguments) = match ctx.types.get_type(return_type) {
            Type::Reference {
                symbol,
                static_arguments,
            } => (*symbol, static_arguments.clone()),
            _ => return Ok(None),
        };
        let (expected_symbol, mut expected_arguments) =
            match ctx.types.get_type(expected_return_type) {
                Type::Reference {
                    symbol,
                    static_arguments: Some(arguments),
                } => (*symbol, arguments.clone()),
                _ => return Ok(None),
            };

        // resolve return arguments when possible
        if let Some(arguments) = return_arguments.as_deref() {
            let source_id = ctx.types.get_type_source(return_type);
            let resolved = self.resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                return_symbol,
                Some(arguments),
                false,
            )?;
            if let Some(resolved) = resolved {
                return_arguments = Some(resolved);
            }
        }

        // resolve expected arguments when possible
        if !expected_arguments.is_empty() {
            let source_id = ctx.types.get_type_source(expected_return_type);
            let resolved = self.resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                expected_symbol,
                Some(expected_arguments.as_slice()),
                false,
            )?;
            if let Some(resolved) = resolved {
                expected_arguments = resolved;
            }
        }

        // require a shared canonical target for inference
        let canonical_return = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            return_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let canonical_expected = self.canonical_symbol_id(
            ctx.module_symbol_view(),
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
                } = ctx.types.get_type(*ty)
                else {
                    continue;
                };

                if !self.symbol_is_static_parameter(ctx.symbol_type_view(), *parameter_symbol) {
                    continue;
                }

                mapping
                    .entry(*parameter_symbol)
                    .or_insert_with(|| expected_argument.clone());
            }
        } else {
            // fall back to parameter order when return arguments are absent
            let Some(parameter_symbols) =
                self.collect_static_parameter_symbols(ctx.type_view(), return_symbol)
            else {
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
        ctx: &InferContext<'_>,
        receiver_id: LocalNodeIdAny,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)> {
        let receiver_expression_id = receiver_id.into_typed::<Expression>();
        let receiver_symbol = ctx.tree.get(receiver_expression_id).target_symbol()?;
        if receiver_symbol.module_id != ctx.module.id {
            return None;
        }

        let receiver_type_id = ctx
            .types
            .get_type_id_for_symbol(ctx.symbols, receiver_symbol)?;
        let declared_type = ctx.types.get_type(receiver_type_id);

        self.receiver_reference_for_inherited_arguments(declared_type, ctx.types)
    }

    /// Extract a reference symbol from the source node attached to one receiver type id.
    fn receiver_reference_for_type_source(
        &self,
        ctx: &InferContext<'_>,
        receiver_ty_id: LocalTypeId,
    ) -> Option<(GlobalSymbolId, Option<Vec<StaticArgument>>)> {
        let source_id = ctx.types.get_type_source(receiver_ty_id);
        let source_global_id = source_id.into_global(ctx.module.id);
        let source_ty_id = ctx
            .infer
            .inferred_type_for_node(source_global_id)
            .or_else(|| ctx.types.get_declared_or_inferred_type_id(source_global_id))?;
        let source_ty = ctx.types.get_type(source_ty_id);

        self.receiver_reference_for_inherited_arguments(source_ty, ctx.types)
    }

    /// Infer a dynamic argument value with contextual typing.
    pub(crate) fn infer_argument(
        &self,
        ctx: &mut InferContext<'_>,
        argument_id: LocalNodeId<Argument>,
        expected_ty_id: Option<LocalTypeId>,
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        let argument = ctx.tree.get(argument_id);

        // apply the expected type to the argument value
        let mut arg_state = state
            .nested_expression_context()
            .with_expected_type(expected_ty_id);
        if expected_ty_id.is_some()
            && matches!(arg_state.contextual_typing, ContextualTypingMode::Satisfies)
        {
            // allow contextual typing for argument inference under satisfies
            arg_state = arg_state.with_contextual_typing_mode(ContextualTypingMode::Default);
        }

        match argument {
            Argument::Positional { value, .. } => {
                self.infer_expression(&mut ctx.reborrow(), *value, &mut arg_state)?;
            }
            Argument::Named { name: _, value, .. } => {
                self.infer_expression(&mut ctx.reborrow(), *value, &mut arg_state)?;
            }
            Argument::Labeled {
                label: _, value, ..
            } => {
                self.infer_expression(&mut ctx.reborrow(), *value, &mut arg_state)?;
            }
            Argument::Spread {
                label: _, value, ..
            } => {
                self.infer_expression(&mut ctx.reborrow(), *value, &mut arg_state)?;
            }
            Argument::Error { value } => {
                self.infer_expression(&mut ctx.reborrow(), *value, &mut arg_state)?;
            }
        }

        Ok(())
    }

    /// Resolve a static argument for a parameter.
    /// Returns `None` when no argument is provided and no default exists.
    pub(crate) fn resolve_static_argument(
        &self,
        ctx: &mut TypeContext<'_>,
        call_site: TreeSymbolView<'_>,
        call_site_options: &AnalyzeOptions,
        static_parameter: &StaticParameter,
        assigned_argument: Option<StaticArgument>,
        treat_type_arguments_as_types: bool,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // resolve explicit argument when provided
        if let Some(argument) = assigned_argument {
            let resolved_argument = self.resolve_explicit_static_argument(
                &mut ctx.reborrow(),
                call_site,
                call_site_options,
                static_parameter,
                argument,
                treat_type_arguments_as_types,
            )?;

            // normalize value arguments back into value expressions
            let resolved_argument = if static_parameter.kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved_argument, ctx.types)
            } else {
                resolved_argument
            };

            return Ok(Some(resolved_argument));
        }

        // default expression
        if let Some(default_expression) = static_parameter.default_expression.as_ref() {
            let resolved_argument = self.resolve_default_static_argument(
                &mut ctx.reborrow(),
                static_parameter,
                default_expression,
                treat_type_arguments_as_types,
            )?;

            // normalize value defaults back into value expressions
            let resolved_argument = if static_parameter.kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved_argument, ctx.types)
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
        ctx: &mut TypeContext<'_>,
        call_site: TreeSymbolView<'_>,
        call_site_options: &AnalyzeOptions,
        static_parameter: &StaticParameter,
        argument: StaticArgument,
        treat_type_arguments_as_types: bool,
    ) -> AnalyzeResult<StaticArgument> {
        // resolve explicit arguments based on parameter kind
        let resolved_argument = match (static_parameter.kind, argument) {
            (StaticParameterKind::Type, StaticArgument::Unevaluated { node }) => {
                let mut call_site_ctx = TypeContext::new(
                    ctx.compiler_context,
                    call_site.module,
                    call_site.profile,
                    call_site_options,
                    call_site.tree,
                    call_site.symbols,
                    ctx.types,
                    ctx.index.clone(),
                );

                // prefer value literals when type arguments stay unconverted
                if !treat_type_arguments_as_types
                    && let Some(value) =
                        self.evaluate_static_argument_as_value(&mut call_site_ctx.reborrow(), node)?
                {
                    return Ok(value);
                }

                let resolved =
                    self.evaluate_static_argument_as_type(&mut call_site_ctx.reborrow(), node)?;
                resolved.unwrap_or(StaticArgument::Unevaluated { node })
            }
            (StaticParameterKind::Value, StaticArgument::Unevaluated { node }) => {
                let mut call_site_ctx = TypeContext::new(
                    ctx.compiler_context,
                    call_site.module,
                    call_site.profile,
                    call_site_options,
                    call_site.tree,
                    call_site.symbols,
                    ctx.types,
                    ctx.index.clone(),
                );
                self.resolve_value_static_argument(&mut call_site_ctx.reborrow(), node)?
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
                        ctx.types.get_type_source(static_parameter.declared_type_id),
                        ctx.types,
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
        ctx: &mut TypeContext<'_>,
        node: GlobalNodeIdAny,
    ) -> AnalyzeResult<StaticArgument> {
        // prefer static value evaluation first
        if let Some(value) = self.evaluate_static_argument_as_value(&mut ctx.reborrow(), node)? {
            return Ok(value);
        }

        // preserve symbolic static references directly from expression symbols
        if let Some(argument) =
            self.evaluate_symbolic_static_reference_argument(&mut ctx.reborrow(), node)?
        {
            return Ok(argument);
        }

        // preserve symbolic static references in value slots
        if let Some(StaticArgument::Evaluated { name, value }) =
            self.evaluate_static_argument_as_type(&mut ctx.reborrow(), node)?
            && let StaticExpression::Type { ty } = value
            && let Type::Reference { symbol, .. } = ctx.types.get_type(ty)
        {
            let symbol = *symbol;
            if self.symbol_is_symbolic_static_value_reference(ctx.type_view(), symbol)? {
                return Ok(StaticArgument::Evaluated {
                    name,
                    value: StaticExpression::Type { ty },
                });
            }
        }

        // reject non-static values when no reference is available
        Ok(StaticArgument::Unevaluated { node })
    }

    /// Preserve symbolic static references directly from argument expression symbols.
    fn evaluate_symbolic_static_reference_argument(
        &self,
        ctx: &mut TypeContext<'_>,
        argument_node: GlobalNodeIdAny,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        let mut evaluated = None;
        let mut ctx = ctx.reborrow();
        let _ = self.with_static_argument_owner(&mut ctx, argument_node, |ctx, argument_id| {
            let argument = ctx.tree.get(argument_id);
            let argument_name = match argument {
                Argument::Named { name, .. } => Some(name.string()),
                _ => None,
            };

            let expression_id = argument.value();
            let symbol = self
                .symbolic_static_value_symbol_for_expression(&mut ctx.reborrow(), expression_id)?;
            let Some(mut symbol) = symbol else {
                return Ok(());
            };
            if self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), symbol)?
                == Some(StaticMemberSymbolKind::EnumField)
                && let Some(enum_symbol) =
                    self.enum_symbol_for_member_expression(&mut ctx.reborrow(), expression_id)?
            {
                symbol = enum_symbol;
            }

            let reference_ty = Type::Reference {
                symbol,
                static_arguments: None,
            };
            let ty_id = ctx
                .types
                .insert_type_from_any(reference_ty, expression_id.into_any());

            evaluated = Some(StaticArgument::Evaluated {
                name: argument_name,
                value: StaticExpression::Type { ty: ty_id },
            });
            Ok(())
        })?;

        Ok(evaluated)
    }

    /// Resolve one symbolic static value symbol from an expression when possible.
    fn symbolic_static_value_symbol_for_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let direct_symbol = self
            .reference_symbol_for_expression(ctx.tree_symbol_view(), expression_id)
            .or_else(|| ctx.tree.get(expression_id).target_symbol());
        if let Some(symbol) = direct_symbol
            && self.symbol_is_symbolic_static_value_reference(ctx.type_view(), symbol)?
        {
            return Ok(Some(symbol));
        }

        // unwrap parenthesized expressions before scope lookup
        if let Expression::Parenthesized { expression } = ctx.tree.get(expression_id) {
            return self
                .symbolic_static_value_symbol_for_expression(&mut ctx.reborrow(), *expression);
        }

        let Expression::UnresolvedPath { path, .. } = ctx.tree.get(expression_id) else {
            return Ok(None);
        };
        if path.segments.len() != 1 {
            return Ok(None);
        }

        let key = StaticKey::Name(path.segments[0]);
        let (_scope_id, scope, _mark) = ctx.symbols.get_scope(expression_id, ctx.tree);
        let mut scope_cursor = Some(scope);

        while let Some(scope) = scope_cursor {
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().rev() {
                if *candidate_key != key {
                    continue;
                }

                let candidate = ctx.symbols.get_symbol(*candidate_symbol_id);
                if !candidate.is_active {
                    continue;
                }

                let symbol = candidate_symbol_id.into_global(ctx.module.id);
                if self.symbol_is_symbolic_static_value_reference(ctx.type_view(), symbol)? {
                    return Ok(Some(symbol));
                }
            }

            scope_cursor = scope
                .parent
                .map(|(parent_id, _)| ctx.symbols.get_scope_by_id(parent_id));
        }

        Ok(None)
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
            } => {
                let ty = if let Type::Reference { symbol, .. } = types.get_type(ty) {
                    if symbol.ty() == SymbolType::Enum {
                        ty
                    } else {
                        types.get_value_type_id(*symbol).unwrap_or(ty)
                    }
                } else {
                    ty
                };

                self.static_expression_from_value_type(ty, types)
                    .map(|value| StaticArgument::Evaluated { name, value })
                    .unwrap_or(StaticArgument::Evaluated {
                        name,
                        value: StaticExpression::Type { ty },
                    })
            }
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
                    StaticKey::Name(name) => Some(Key::Name(Name::Identifier(name))),
                    StaticKey::Number(value) => Some(Key::Name(Name::Number(value))),
                    _ => None,
                }?;

                let value = self.static_expression_from_value_type(field.ty, types)?;
                properties.push(StaticProperty::Field {
                    key,
                    value,
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
        ctx: &mut TypeContext<'_>,
        static_parameter: &StaticParameter,
        default_expression: &GlobalNodeId<Expression>,
        treat_type_arguments_as_types: bool,
    ) -> AnalyzeResult<StaticArgument> {
        // select the module context for the default expression
        if default_expression.module_id == ctx.module.id {
            return self.evaluate_static_default_argument(
                &mut ctx.reborrow(),
                static_parameter.kind,
                static_parameter.name,
                default_expression.local_id,
                treat_type_arguments_as_types,
            );
        }

        // load the remote module context for the default expression
        self.with_declared_type_context_for_module(
            &mut ctx.reborrow(),
            default_expression.module_id,
            |ctx| {
                self.evaluate_static_default_argument(
                    ctx,
                    static_parameter.kind,
                    static_parameter.name,
                    default_expression.local_id,
                    treat_type_arguments_as_types,
                )
            },
        )
    }

    /// Materialize a static type argument for validation.
    pub(super) fn materialize_static_type_argument(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        validation_mode: StaticArgumentValidationMode,
    ) -> AnalyzeResult<LocalTypeId> {
        // pre-evaluate local type aliases used as bounds
        if let Type::Reference { symbol, .. } =
            ctx.types.get_type(static_parameter.declared_type_id)
            && symbol.ty() == SymbolType::TypeAlias
        {
            self.unwrap_type_alias_reference(
                &mut ctx.reborrow(),
                static_parameter.declared_type_id,
            )?;
        }

        // evaluate the declared bound when needed
        self.ensure_static_parameter_bound_evaluated(
            &mut ctx.reborrow(),
            static_parameter.declared_type_id,
        )?;

        // re-evaluate unevaluated arguments before building the substitution type
        let resolved_argument =
            if let StaticArgument::Unevaluated { node } = resolved_static_argument {
                self.evaluate_static_argument_as_type(&mut ctx.reborrow(), *node)?
                    .unwrap_or_else(|| resolved_static_argument.clone())
            } else {
                resolved_static_argument.clone()
            };

        // build the substitution type from the argument
        let substitution_ty_id =
            self.convert_static_argument_type(&resolved_argument, error_node.local_id, ctx.types);

        // ensure referenced instance types are available for validation
        if validation_mode.uses_eager_instance_types() {
            self.ensure_reference_instance_types_for_type(
                &mut ctx.reborrow(),
                error_node.local_id,
                static_parameter.declared_type_id,
            )?;
            self.ensure_reference_instance_types_for_type(
                &mut ctx.reborrow(),
                error_node.local_id,
                substitution_ty_id,
            )?;
        } else {
            self.ensure_local_reference_instance_types_for_type(
                &mut ctx.reborrow(),
                error_node.local_id,
                static_parameter.declared_type_id,
            )?;
            self.ensure_local_reference_instance_types_for_type(
                &mut ctx.reborrow(),
                error_node.local_id,
                substitution_ty_id,
            )?;
        }

        Ok(substitution_ty_id)
    }

    /// Resolve the type of a static value argument for validation.
    fn static_value_argument_type(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        value: &StaticExpression,
    ) -> AnalyzeResult<LocalTypeId> {
        // normalize enum-field references to their owning enum type
        if let StaticExpression::Type { ty } = value
            && let Type::Reference { symbol, .. } = ctx.types.get_type(*ty)
            && let Some(enum_symbol) =
                self.enum_symbol_for_field_symbol(ctx.symbol_type_view(), *symbol)?
        {
            let enum_type_id = ctx.types.insert_type_from_any(
                Type::Reference {
                    symbol: enum_symbol,
                    static_arguments: None,
                },
                error_node.local_id,
            );
            return Ok(enum_type_id);
        }

        // prefer static parameter constraints for referenced type expressions
        if let StaticExpression::Type { ty } = value {
            let symbol = match ctx.types.get_type(*ty) {
                Type::Reference { symbol, .. } => Some(*symbol),
                _ => None,
            };
            if let Some(symbol) = symbol
                && self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol)
                && let Some(constraint_id) = self.static_parameter_constraint_type(
                    &mut ctx.reborrow(),
                    symbol,
                    error_node.local_id,
                )
            {
                self.ensure_static_parameter_bound_evaluated(&mut ctx.reborrow(), constraint_id)?;
                return Ok(constraint_id);
            }
        }

        // normalize static expression shapes into value types
        self.static_expression_value_type(&mut ctx.reborrow(), error_node, value)
    }

    /// Check whether a scalar literal matches an enum constraint.
    fn enum_constraint_accepts_literal(
        &self,
        ctx: &mut TypeContext<'_>,
        constraint_ty_id: LocalTypeId,
        literal: &ScalarLiteral,
    ) -> AnalyzeResult<bool> {
        // resolve the enum symbol for the constraint when possible
        let enum_symbol =
            self.enum_symbol_for_type(ctx.types.get_type(constraint_ty_id), ctx.types);
        let Some(enum_symbol) = enum_symbol else {
            return Ok(false);
        };

        // select the module context for the enum
        let matches = if enum_symbol.module_id == ctx.module.id {
            let _ = self.enum_backing_type_for_symbol(&mut ctx.reborrow(), enum_symbol);
            self.enum_literal_matches_symbol(ctx.tree, ctx.symbols, ctx.types, enum_symbol, literal)
        } else {
            self.require_remote_artifact_dir(
                ctx.compiler_context,
                ctx.module.id,
                enum_symbol.module_id,
                ctx.profile,
                destack_artifact::ArtifactKey::dir_declared,
            )
            .map_err(AnalyzeError::from)?;
            let snapshot = self
                .require_artifact_dir_declared(
                    ctx.compiler_context.revision(),
                    enum_symbol.module_id,
                    ctx.profile,
                )
                .map_err(AnalyzeError::from)?;
            self.enum_literal_matches_symbol(
                &snapshot.tree,
                &snapshot.symbols,
                &snapshot.types,
                enum_symbol,
                literal,
            )
        };

        Ok(matches)
    }

    /// Evaluate a static parameter bound when it is still unevaluated.
    fn ensure_static_parameter_bound_evaluated(
        &self,
        ctx: &mut TypeContext<'_>,
        bound_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        if matches!(ctx.types.get_type(bound_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(&mut ctx.reborrow(), bound_id)?;
        }
        Ok(())
    }

    /// Check whether a scalar literal matches an enum field value.
    fn enum_literal_matches_symbol(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        _types: &TypeTable,
        _enum_symbol: GlobalSymbolId,
        literal: &ScalarLiteral,
    ) -> bool {
        // ensure the symbol refers to an enum declaration
        let symbol_entry = symbols.get_symbol(_enum_symbol.local_id);
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

        // scan enum fields in declaration order
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let Declaration::Enum(declaration) = tree.get(declaration_id) else {
                continue;
            };
            let mut next_integer = 0_i64;
            for field_id in &declaration.fields {
                let field = tree.get(*field_id);

                // explicit enum field values
                if let Some(value_id) = field.value
                    && let Expression::ScalarLiteral { value } = tree.get(value_id)
                {
                    if self.enum_field_value_matches_literal_from_scalar(value, literal) {
                        return true;
                    }

                    if let ScalarLiteral::Integer(value) = value {
                        next_integer = *value + 1;
                    }

                    continue;
                }

                // implicit numeric enum field values
                if let ScalarLiteral::Integer(value) = literal
                    && *value == next_integer
                {
                    return true;
                }

                next_integer += 1;
            }
        }

        false
    }

    /// Check whether an enum field value matches a scalar literal.
    fn enum_field_value_matches_literal_from_scalar(
        &self,
        value: &ScalarLiteral,
        literal: &ScalarLiteral,
    ) -> bool {
        match (value, literal) {
            (ScalarLiteral::Integer(value), ScalarLiteral::Integer(literal)) => value == literal,
            (ScalarLiteral::String(value), ScalarLiteral::String(literal)) => value == literal,
            _ => false,
        }
    }

    /// Convert a static expression into a value type for validation.
    fn static_expression_value_type(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        value: &StaticExpression,
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
            | StaticExpression::TupleExpression { elements } => {
                self.static_expression_tuple_type(&mut ctx.reborrow(), error_node, elements)?
            }
            StaticExpression::ObjectExpression { properties } => {
                self.static_expression_object_type(&mut ctx.reborrow(), error_node, properties)?
            }
            StaticExpression::Declaration { .. } | StaticExpression::Unevaluated { .. } => {
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            }
        };

        Ok(ctx.types.insert_type_from_any(ty, error_node.local_id))
    }

    /// Convert a static tuple/array expression into a tuple type.
    fn static_expression_tuple_type(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        elements: &[StaticExpression],
    ) -> AnalyzeResult<Type> {
        // map tuple literal expressions into tuple types
        let mut element_types = Vec::with_capacity(elements.len());
        for element in elements {
            let element_ty_id =
                self.static_expression_value_type(&mut ctx.reborrow(), error_node, element)?;
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
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        properties: &[StaticProperty],
    ) -> AnalyzeResult<Type> {
        // map object literal expressions into structural object types
        let mut fields = Vec::new();
        for property in properties {
            let StaticProperty::Field { key, value, .. } = property else {
                continue;
            };
            let key = match *key {
                Key::Name(Name::Identifier(name)) | Key::Name(Name::String(name)) => {
                    Some(StaticKey::Name(name))
                }
                Key::Name(Name::Number(name)) => Some(StaticKey::Number(name)),
                _ => None,
            };
            let Some(key) = key else {
                continue;
            };
            let field_ty_id =
                self.static_expression_value_type(&mut ctx.reborrow(), error_node, value)?;
            fields.push(TypeField {
                key,
                ty: field_ty_id,
                is_optional: false,
                is_readonly: false,
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
    pub(super) fn validate_static_argument(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        prepared_substitution: Option<LocalTypeId>,
        bound_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        infer: Option<&mut InferTable>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // validate type arguments against the declared bound
        if static_parameter.kind == StaticParameterKind::Type {
            // coerce the argument into a type
            let substitution_ty_id = prepared_substitution.unwrap_or_else(|| {
                self.convert_static_argument_type(
                    resolved_static_argument,
                    error_node.local_id,
                    ctx.types,
                )
            });

            // skip bound validation in declaration modules
            if ctx.module.language_type.is_declaration() {
                return Ok(Some(substitution_ty_id));
            }

            // resolve bounds via constraint lookup when the declared slot is not concrete
            let mut declared_bound_id = static_parameter.declared_type_id;
            let declared_bound_needs_constraint = matches!(
                ctx.types.get_type(declared_bound_id),
                Type::Unevaluated(_)
                    | Type::InferVar { .. }
                    | Type::TypeLiteral {
                        value: TypeLiteral::Unknown | TypeLiteral::Any,
                    }
            );
            if declared_bound_needs_constraint
                && self.symbol_is_static_parameter(ctx.symbol_type_view(), static_parameter.symbol)
                && let Some(constraint_id) = self.static_parameter_constraint_type(
                    &mut ctx.reborrow(),
                    static_parameter.symbol,
                    error_node.local_id,
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
                ctx.types,
                &mut cache,
            );

            // fall back to resolved constraint types when bounds are unknown
            if matches!(
                ctx.types.get_type(expected_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any
                }
            ) && self.symbol_is_static_parameter(ctx.symbol_type_view(), static_parameter.symbol)
                && let Some(constraint_id) = self.static_parameter_constraint_type(
                    &mut ctx.reborrow(),
                    static_parameter.symbol,
                    error_node.local_id,
                )
            {
                let mut cache = HashMap::new();
                let substituted = self.substitute_static_parameters(
                    constraint_id,
                    &substitutions,
                    ctx.types,
                    &mut cache,
                );
                if !matches!(
                    ctx.types.get_type(substituted),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown | TypeLiteral::Any
                    }
                ) {
                    expected_ty_id = substituted;
                }
            }

            // accept all arguments when the declared bound is unknown or any
            if matches!(
                ctx.types.get_type(expected_ty_id),
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
            if !self.is_infer_var_type(declared_bound_id, ctx.types)
                && !self.is_infer_var_type(substitution_ty_id, ctx.types)
                && self.is_type_assignable(&mut ctx.reborrow(), expected_ty_id, substitution_ty_id)
                    == Assignability::NotAssignable
            {
                // accept static parameter arguments when their constraints satisfy the bound
                let symbol = match ctx.types.get_type(substitution_ty_id) {
                    Type::Reference { symbol, .. } => Some(*symbol),
                    _ => None,
                };
                if let Some(symbol) = symbol
                    && self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol)
                    && let Some(constraint_ty_id) = self.static_parameter_constraint_type(
                        &mut ctx.reborrow(),
                        symbol,
                        error_node.local_id,
                    )
                    && self.constraint_satisfies_bound(
                        &mut ctx.reborrow(),
                        expected_ty_id,
                        constraint_ty_id,
                    )
                {
                    return Ok(Some(substitution_ty_id));
                }

                // allow type parameters that satisfy the expected bound via their constraints
                if let Some(constraint_ty_id) = self.materialize_static_argument_constraint_type(
                    &mut ctx.reborrow(),
                    error_node,
                    substitution_ty_id,
                ) && self.constraint_satisfies_bound(
                    &mut ctx.reborrow(),
                    expected_ty_id,
                    constraint_ty_id,
                ) {
                    return Ok(Some(substitution_ty_id));
                }

                // report failed bound validation
                self.emit_unassignable_type_for_types(
                    ctx.module_type_view(),
                    error_node.local_id,
                    expected_ty_id,
                    substitution_ty_id,
                );
                return Ok(Some(
                    ctx.types
                        .insert_type_from_any(Type::Error, error_node.local_id),
                ));
            }

            // accept validated type arguments
            return Ok(Some(substitution_ty_id));
        }

        // skip value validation in declaration modules
        if ctx.module.language_type.is_declaration() {
            return Ok(None);
        }

        // validate value arguments against the declared type
        if static_parameter.kind == StaticParameterKind::Value
            && let StaticArgument::Unevaluated { node } = resolved_static_argument
        {
            if self.unevaluated_static_value_argument_requires_convergence(
                &mut ctx.reborrow(),
                *node,
            )? {
                return Ok(None);
            }

            self.error(AnalyzeError::NonStaticArgument {
                node: error_node.into_anchored(Some(ctx.profile)),
            });
            return Ok(Some(
                ctx.types
                    .insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        let StaticArgument::Evaluated { value, .. } = resolved_static_argument else {
            return Ok(None);
        };

        // keep symbolic static values deferred until substitution convergence
        if static_parameter.kind == StaticParameterKind::Value
            && let StaticExpression::Unevaluated { node } = value
            && self.static_value_expression_requires_convergence(ctx.type_view(), *node)
        {
            return Ok(None);
        }

        // evaluated unevaluated values represent symbolic static expressions
        if static_parameter.kind == StaticParameterKind::Value
            && matches!(value, StaticExpression::Unevaluated { .. })
        {
            return Ok(None);
        }

        // reject non static value arguments
        if static_parameter.kind == StaticParameterKind::Value
            && !self.static_value_argument_is_static(value, ctx.type_view())
        {
            self.error(AnalyzeError::NonStaticArgument {
                node: error_node.into_anchored(Some(ctx.profile)),
            });
            return Ok(Some(
                ctx.types
                    .insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        // reject not assignable value arguments
        let value_ty_id =
            self.static_value_argument_type(&mut ctx.reborrow(), error_node, value)?;

        // accept enum literal values that match enum constraints
        if let StaticExpression::ScalarLiteral { value: literal } = value
            && self.enum_constraint_accepts_literal(
                &mut ctx.reborrow(),
                static_parameter.declared_type_id,
                literal,
            )?
        {
            return Ok(Some(value_ty_id));
        }

        if !self.is_infer_var_type(static_parameter.declared_type_id, ctx.types)
            && self.is_type_assignable(
                &mut ctx.reborrow(),
                static_parameter.declared_type_id,
                value_ty_id,
            ) == Assignability::NotAssignable
        {
            // report unassignable value arguments
            self.emit_unassignable_type_for_types(
                ctx.module_type_view(),
                error_node.local_id,
                static_parameter.declared_type_id,
                value_ty_id,
            );
            return Ok(Some(
                ctx.types
                    .insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        Ok(Some(value_ty_id))
    }

    /// Check whether a static value argument is a static expression.
    pub(crate) fn static_value_argument_is_static(
        &self,
        value: &StaticExpression,
        ctx: TypeView<'_>,
    ) -> bool {
        // classify static expressions by evaluation state
        match value {
            StaticExpression::Unevaluated { .. } => false,
            StaticExpression::ScalarLiteral { .. } => true,
            StaticExpression::TypeLiteral { value } => !matches!(value, TypeLiteral::Unknown),
            StaticExpression::Type { ty } => {
                if matches!(
                    ctx.types.get_type(*ty),
                    Type::Unevaluated(_)
                        | Type::TypeLiteral {
                            value: TypeLiteral::Unknown
                        }
                ) {
                    return false;
                }

                // associated type projections are type-space only and cannot be value arguments
                let mut visited = HashSet::new();
                !self.type_contains_associated_type_reference(
                    ctx.tree_symbol_view(),
                    *ty,
                    ctx.types,
                    &mut visited,
                )
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_none_or(|arguments| {
                arguments.iter().all(|argument| match argument {
                    StaticArgument::Unevaluated { .. } => false,
                    StaticArgument::Evaluated { value, .. } => {
                        self.static_value_argument_is_static(value, ctx)
                    }
                })
            }),
            StaticExpression::ArrayExpression { elements } => elements
                .iter()
                .all(|element| self.static_value_argument_is_static(element, ctx)),
            StaticExpression::TupleExpression { elements } => elements
                .iter()
                .all(|element| self.static_value_argument_is_static(element, ctx)),
            StaticExpression::ObjectExpression { properties } => properties
                .iter()
                .all(|property| self.static_property_is_static(property, ctx)),
        }
    }

    /// Check whether a static property is fully static.
    fn static_property_is_static(&self, property: &StaticProperty, ctx: TypeView<'_>) -> bool {
        // accept property values only when they are fully static
        match property {
            StaticProperty::Unevaluated { .. } => false,
            StaticProperty::Field { value, .. } => self.static_value_argument_is_static(value, ctx),
            StaticProperty::Method { body, .. } => self.static_value_argument_is_static(body, ctx),
            StaticProperty::Spread { value, .. } => {
                self.static_value_argument_is_static(value, ctx)
            }
        }
    }

    /// Resolve the target argument mapping for an extension declaration.
    pub(crate) fn extension_target_argument_mapping(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        extension_symbol: GlobalSymbolId,
        extension_parameters: &[GlobalSymbolId],
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<Vec<usize>>> {
        self.require_remote_artifact_dir(
            context,
            module.id,
            extension_symbol.module_id,
            profile,
            destack_artifact::ArtifactKey::dir_declared,
        )?;

        let remote_snapshot;

        // read local tables directly and fall back to the declared remote artifact when needed
        let (tree, symbols) = if extension_symbol.module_id == module.id {
            (tree, symbols)
        } else {
            remote_snapshot = self
                .require_artifact_dir_declared(
                    context.revision(),
                    extension_symbol.module_id,
                    profile,
                )
                .map_err(AnalyzeError::from)?;
            (
                remote_snapshot.tree.as_ref(),
                remote_snapshot.symbols.as_ref(),
            )
        };

        let symbol_entry = symbols.get_symbol(extension_symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(None);
        };
        let Ok(declaration_id) = primary_declaration.try_into_local_typed::<Declaration>() else {
            return Ok(None);
        };
        let declaration = tree.get(declaration_id);
        let Declaration::Extension(declaration) = declaration else {
            return Ok(None);
        };
        let target_type = declaration.target_type;

        // read the target type arguments
        let target_expression = tree.get(target_type);
        let generic_arguments = match target_expression {
            TypeExpression::LocalReference {
                generic_arguments, ..
            }
            | TypeExpression::ModuleReference {
                generic_arguments, ..
            }
            | TypeExpression::GlobalReference {
                generic_arguments, ..
            } => Some(generic_arguments.as_slice()),
            _ => None,
        };
        let Some(generic_arguments) = generic_arguments else {
            return Ok(None);
        };

        // map target arguments to extension parameter indices
        let mut mapping = Vec::with_capacity(generic_arguments.len());
        for argument_id in generic_arguments {
            let argument = tree.get(*argument_id);
            let target_symbol = match argument {
                GenericArgument::Type { value } => match tree.get(*value) {
                    TypeExpression::LocalReference { target_symbol, .. }
                    | TypeExpression::ModuleReference { target_symbol, .. }
                    | TypeExpression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
                    _ => None,
                },
                GenericArgument::Value { .. } | GenericArgument::Error => return Ok(None),
            };
            let Some(target_symbol) = target_symbol else {
                return Ok(None);
            };

            let Some(parameter_index) = extension_parameters
                .iter()
                .position(|parameter_symbol| *parameter_symbol == target_symbol)
            else {
                return Ok(None);
            };
            mapping.push(parameter_index);
        }

        Ok(Some(mapping))
    }

    /// Resolve a static argument constraint for validation.
    fn materialize_static_argument_constraint_type(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        argument_ty_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // collect referenced static parameters
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_type_reference_symbols(
            argument_ty_id,
            ctx.types,
            &mut referenced_symbols,
            &mut visited,
        );

        // materialize referenced constraints
        let mut substitutions = HashMap::new();
        let mut visiting_symbols = HashSet::new();
        for symbol in referenced_symbols {
            let constraint_id = self.materialize_static_parameter_constraint(
                &mut ctx.reborrow(),
                error_node,
                symbol,
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
        Some(self.substitute_static_parameters(
            argument_ty_id,
            &substitutions,
            ctx.types,
            &mut cache,
        ))
    }

    /// Materialize a static parameter constraint by substituting nested constraints.
    fn materialize_static_parameter_constraint(
        &self,
        ctx: &mut TypeContext<'_>,
        error_node: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
        visiting: &mut HashSet<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        // avoid recursive constraint expansion
        if !visiting.insert(symbol) {
            return None;
        }

        // resolve the declared constraint type
        if !self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
            visiting.remove(&symbol);
            return None;
        }
        let constraint_id =
            self.static_parameter_constraint_type(&mut ctx.reborrow(), symbol, error_node.local_id);
        let Some(constraint_id) = constraint_id else {
            visiting.remove(&symbol);
            return None;
        };

        // collect nested static parameters in the constraint
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_type_reference_symbols(
            constraint_id,
            ctx.types,
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
                &mut ctx.reborrow(),
                error_node,
                referenced_symbol,
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
        Some(self.substitute_static_parameters(
            constraint_id,
            &substitutions,
            ctx.types,
            &mut cache,
        ))
    }

    /// Resolve static arguments for a type reference.
    pub(crate) fn resolve_type_reference_static_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_RESOLVE);
        // canonicalize import targets while preserving alias identity
        let symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let symbol = self.merged_type_symbol_id(ctx.module_symbol_view(), symbol);

        self.resolve_type_reference_static_arguments_with_validation_mode(
            ctx,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            None,
            StaticArgumentValidationMode::Analyze,
        )
    }

    /// Resolve static arguments for one declared type reference.
    pub(crate) fn resolve_declared_type_reference_static_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_RESOLVE);
        let symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let symbol = self.merged_type_symbol_id(ctx.module_symbol_view(), symbol);

        self.resolve_type_reference_static_arguments_with_validation_mode(
            ctx,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            None,
            StaticArgumentValidationMode::Declare,
        )
    }

    /// Resolve static arguments for a canonicalized type reference under one validation mode.
    fn resolve_type_reference_static_arguments_with_validation_mode(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        validation_mode: StaticArgumentValidationMode,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        self.resolve_type_reference_static_arguments_with_bounds_and_validation_mode(
            ctx,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            bound_substitutions,
            validation_mode,
        )
    }

    /// Resolve static arguments for a canonicalized type reference in one substitution environment.
    pub(crate) fn resolve_type_reference_static_arguments_with_bounds(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        self.resolve_type_reference_static_arguments_with_bounds_and_validation_mode(
            ctx,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            bound_substitutions,
            StaticArgumentValidationMode::Analyze,
        )
    }

    /// Resolve static arguments for a canonicalized type reference in one substitution environment.
    fn resolve_type_reference_static_arguments_with_bounds_and_validation_mode(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        validation_mode: StaticArgumentValidationMode,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // treat type arguments as types for type references
        let treat_type_arguments_as_types = true;

        // check for cached resolved static arguments
        let options_cache_key = ctx.options.cache_key();
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
            && let Some(cached) = ctx.types.get_static_argument_resolution_cache(cache_key)
        {
            return Ok(cached);
        }

        // skip non instantiable symbols
        if !self.query_symbol_is_instantiable(symbol) && symbol.ty() != SymbolType::Extension {
            return Ok(None);
        }

        // reuse resolved arguments when an instance is already registered for this node
        let has_explicit_arguments = static_arguments.is_some_and(|args| !args.is_empty());
        let node_global_id = node_id.into_global(ctx.module.id);
        if !has_explicit_arguments
            && let Some(arguments) =
                self.query_instance_arguments_for_node(node_global_id, Some(symbol), ctx.types)
        {
            return Ok(Some(arguments));
        }

        // guard against recursive resolution on the same reference
        let argument_slice = static_arguments.unwrap_or(&[]);
        if ctx
            .types
            .is_static_argument_resolution_in_progress(symbol, argument_slice)
        {
            if !argument_slice.is_empty() {
                let resolved_arguments = if symbol.module_id == ctx.module.id {
                    self.materialize_static_arguments_for_reference(
                        &mut ctx.reborrow(),
                        symbol,
                        node_id,
                        argument_slice,
                    )
                } else {
                    self.with_declared_type_context_for_module(
                        &mut ctx.reborrow(),
                        symbol.module_id,
                        |remote_ctx| {
                            Ok(self.materialize_static_arguments_for_reference(
                                remote_ctx,
                                symbol,
                                node_id,
                                argument_slice,
                            ))
                        },
                    )?
                };
                return Ok(Some(resolved_arguments));
            }
            return Ok(None);
        }

        // mark resolution as in progress for this argument set
        let argument_snapshot = argument_slice.to_vec();
        ctx.types
            .mark_static_argument_resolution_in_progress(symbol, argument_snapshot.clone());

        let result = self.resolve_type_reference_static_arguments_inner(
            ctx,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            bound_substitutions,
            treat_type_arguments_as_types,
            validation_mode,
        );

        // clear the in progress marker, resolve cached arguments when needed
        ctx.types
            .clear_static_argument_resolution_in_progress(symbol, &argument_snapshot);
        if let Some(cache_key) = cache_key
            && let Ok(resolved) = &result
        {
            let should_cache = resolved
                .as_ref()
                .map(|arguments| arguments.iter().all(StaticArgument::is_evaluated))
                .unwrap_or(true);
            if should_cache {
                ctx.types
                    .set_static_argument_resolution_cache(cache_key, resolved.clone());
            }
        }

        result
    }

    /// Resolve static arguments for a type reference.
    fn resolve_type_reference_static_arguments_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        treat_type_arguments_as_types: bool,
        validation_mode: StaticArgumentValidationMode,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // preserve caller module context for bound validation
        let call_site = TreeSymbolView::new(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            ctx.tree,
            ctx.symbols,
        );
        let call_site_options = ctx.options;

        // ensure remote declarations are resolved before reading defaults
        if symbol.module_id != ctx.module.id {
            self.require_dir_resolved(
                ctx.compiler_context.revision(),
                symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        }

        if symbol.module_id == ctx.module.id {
            let mut ctx = ctx.reborrow();
            return self.resolve_type_reference_static_arguments_in_owner(
                &mut ctx,
                call_site,
                call_site_options,
                node_id,
                symbol,
                static_arguments,
                validate_static_argument_bounds,
                bound_substitutions,
                treat_type_arguments_as_types,
                validation_mode,
            );
        }

        self.with_declared_type_context_for_module(ctx, symbol.module_id, |owner_ctx| {
            self.resolve_type_reference_static_arguments_in_owner(
                owner_ctx,
                call_site,
                call_site_options,
                node_id,
                symbol,
                static_arguments,
                validate_static_argument_bounds,
                bound_substitutions,
                treat_type_arguments_as_types,
                validation_mode,
            )
        })
    }

    /// Resolve static arguments for one owner-module context.
    fn resolve_type_reference_static_arguments_in_owner(
        &self,
        ctx: &mut TypeContext<'_>,
        call_site: TreeSymbolView<'_>,
        call_site_options: &AnalyzeOptions,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        treat_type_arguments_as_types: bool,
        validation_mode: StaticArgumentValidationMode,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // collect parameter symbols for the declaration
        let parameter_symbols = self.collect_static_parameter_symbols(ctx.type_view(), symbol);
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

        // keep explicit arguments when no parameters exist
        if parameter_symbols.is_empty() {
            if let Some(static_arguments) = static_arguments
                && !static_arguments.is_empty()
            {
                return Ok(Some(static_arguments.to_vec()));
            }

            return Ok(None);
        }

        // gather static parameter metadata for the declaration
        let mut static_parameters = Vec::with_capacity(parameter_symbols.len());
        for symbol_id in &parameter_symbols {
            static_parameters.push(self.resolve_static_parameter(
                &mut ctx.reborrow(),
                *symbol_id,
                node_id,
            ));
        }

        // map arguments to parameter slots
        let argument_values = static_arguments.unwrap_or(&[]);
        let assigned_arguments = self.assign_static_argument_values(
            call_site,
            node_id,
            argument_values,
            &static_parameters,
        );

        // resolve arguments with defaults and error-type recovery
        let mut resolved_arguments = Vec::with_capacity(static_parameters.len());
        let mut resolved_argument_map = HashMap::new();
        for (index, static_parameter) in static_parameters.iter().enumerate() {
            let assigned_argument = assigned_arguments.get(index).cloned().flatten();
            let error_node = if let Some(argument) = &assigned_argument {
                match argument {
                    StaticArgument::Unevaluated { node } => *node,
                    StaticArgument::Evaluated { .. } => node_id.into_global(call_site.module.id),
                }
            } else if let Some(default_expression) = static_parameter.default_expression.as_ref() {
                default_expression
                    .local_id
                    .into_global_any(default_expression.module_id)
            } else {
                node_id.into_global(call_site.module.id)
            };

            // resolve the argument value or synthesize error recovery when unresolved
            let resolved_argument = self.resolve_static_argument(
                &mut ctx.reborrow(),
                call_site,
                call_site_options,
                static_parameter,
                assigned_argument,
                treat_type_arguments_as_types,
            )?;
            let mut resolved_argument = resolved_argument.unwrap_or_else(|| {
                // unresolved references synthesize error recovery values by parameter kind
                let error_ty_id = ctx.types.insert_type_from_any(Type::Error, node_id);
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
                    ctx.types,
                    error_node.into_anchored(Some(ctx.profile)),
                )?;
            }

            // inherit value constraints when passing a static parameter through
            if static_parameter.kind == StaticParameterKind::Value {
                let referenced_symbol = match &resolved_argument {
                    StaticArgument::Evaluated {
                        value: StaticExpression::Type { ty },
                        ..
                    } => self.unwrap_type_value_symbol(ctx.types, *ty),
                    _ => None,
                };

                if let Some(referenced_symbol) = referenced_symbol {
                    let constraint_id = if self
                        .symbol_is_static_parameter(ctx.symbol_type_view(), referenced_symbol)
                    {
                        self.static_parameter_constraint_type(
                            &mut ctx.reborrow(),
                            referenced_symbol,
                            error_node.local_id,
                        )
                    } else {
                        None
                    };

                    if let Some(constraint_id) = constraint_id
                        && matches!(
                            ctx.types.get_type(constraint_id),
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown
                            }
                        )
                    {
                        if matches!(
                            ctx.types.get_type(static_parameter.declared_type_id),
                            Type::Unevaluated(_)
                        ) {
                            self.resolve_declared_type(
                                &mut ctx.reborrow(),
                                static_parameter.declared_type_id,
                            )?;
                        }

                        if !matches!(
                            ctx.types.get_type(static_parameter.declared_type_id),
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown
                            }
                        ) {
                            ctx.types.set_static_parameter_constraint_type(
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
                let mut ctx = ctx.reborrow();
                Some(self.materialize_static_type_argument(
                    &mut ctx,
                    error_node,
                    static_parameter,
                    &resolved_argument,
                    validation_mode,
                )?)
            } else {
                None
            };

            // collect resolved substitutions for prior static parameters
            let local_bound_substitutions = self.static_argument_substitutions_for_bounds(
                &static_parameters[..resolved_arguments.len()],
                &resolved_arguments,
            );
            let mut bound_substitutions = bound_substitutions.cloned().unwrap_or_else(HashMap::new);
            bound_substitutions.extend(local_bound_substitutions);

            // validate type and value arguments against declared bounds
            let validated_type = if validate_static_argument_bounds {
                let mut ctx = TypeContext::new(
                    ctx.compiler_context,
                    call_site.module,
                    ctx.profile,
                    call_site_options,
                    call_site.tree,
                    call_site.symbols,
                    ctx.types,
                    ctx.index.clone(),
                );
                self.validate_static_argument(
                    &mut ctx,
                    error_node,
                    static_parameter,
                    &resolved_argument,
                    materialized_substitution,
                    &bound_substitutions,
                    None,
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
                    } => match ctx.types.get_type(*ty) {
                        Type::Reference { symbol, .. } => {
                            self.symbol_is_static_parameter(ctx.symbol_type_view(), *symbol)
                        }
                        _ => false,
                    },
                    _ => false,
                };
                if matches!(ctx.types.get_type(substitution_ty_id), Type::Error)
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
                        self.normalize_value_static_argument(replacement, ctx.types);
                }
            }

            resolved_argument_map.insert(static_parameter.symbol, resolved_argument.clone());
            resolved_arguments.push(resolved_argument);
        }

        Ok(Some(resolved_arguments))
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

    // substitution assembly
    /// Build type parameter substitutions for a type symbol.
    pub(crate) fn build_type_parameter_substitutions_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        resolved_arguments: &[StaticArgument],
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        // collect static parameter symbols for the declaration
        let parameter_symbols = self.collect_static_parameter_symbols(ctx.type_view(), symbol);
        let Some(parameter_symbols) = parameter_symbols else {
            return HashMap::new();
        };
        if parameter_symbols.is_empty() {
            return HashMap::new();
        }

        // collect static parameters for the declaration
        let mut static_parameters = Vec::with_capacity(parameter_symbols.len());
        for symbol_id in &parameter_symbols {
            static_parameters.push(self.resolve_static_parameter(
                &mut ctx.reborrow(),
                *symbol_id,
                source_id,
            ));
        }

        // build substitutions for type and value parameters
        let mut substitutions = HashMap::new();
        for (static_parameter, argument) in static_parameters.iter().zip(resolved_arguments.iter())
        {
            let ty_id = self.convert_static_argument_type(
                argument,
                ctx.types.get_type_source(static_parameter.declared_type_id),
                ctx.types,
            );
            substitutions.insert(static_parameter.symbol, ty_id);
        }

        substitutions
    }

    // static argument evaluation: type and value paths
    /// Evaluate a static argument as a type.
    pub(crate) fn evaluate_static_argument_as_type(
        &self,
        ctx: &mut TypeContext<'_>,
        argument_node: GlobalNodeIdAny,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        let mut evaluated = None;
        let mut ctx = ctx.reborrow();
        let _ = self.with_static_argument_owner(&mut ctx, argument_node, |ctx, argument_id| {
            let argument = ctx.tree.get(argument_id);
            let argument_name = match argument {
                Argument::Named { name, .. } => Some(name.string()),
                _ => None,
            };

            let expression_id = argument.value();

            // preserve static parameter references in type arguments
            if let Some(parameter_symbol) =
                self.static_parameter_symbol_for_reference(ctx.type_view(), expression_id)?
            {
                let reference_ty = Type::Reference {
                    symbol: parameter_symbol,
                    static_arguments: None,
                };
                let ty_id = ctx
                    .types
                    .insert_type_from_any(reference_ty, expression_id.into_any());
                evaluated = Some(StaticArgument::Evaluated {
                    name: argument_name,
                    value: StaticExpression::Type { ty: ty_id },
                });
                return Ok(());
            }

            // try evaluate expression as a type
            let value =
                self.evaluate_static_argument_as_type(&mut ctx.reborrow(), argument_node)?;
            evaluated = value;
            Ok(())
        })?;

        Ok(evaluated)
    }

    /// Evaluate a static argument as a value.
    pub(crate) fn evaluate_static_argument_as_value(
        &self,
        ctx: &mut TypeContext<'_>,
        argument_node: GlobalNodeIdAny,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        let mut evaluated = None;
        let mut ctx = ctx.reborrow();
        let _ = self.with_static_argument_owner(&mut ctx, argument_node, |ctx, argument_id| {
            // capture argument name for reuse in evaluated form
            let argument = ctx.tree.get(argument_id);
            let argument_name = match argument {
                Argument::Named { name, .. } => Some(name.string()),
                _ => None,
            };

            // evaluate the expression into a static value when possible
            let expression_id = argument.value();
            let value =
                self.evaluate_static_expression_value(&mut ctx.reborrow(), expression_id, None)?;
            let value = if let Some(value) = value {
                value
            } else if let Some(enum_symbol) =
                self.enum_symbol_for_member_expression(&mut ctx.reborrow(), expression_id)?
            {
                let ty = ctx.types.insert_type_from_any(
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
        })?;

        Ok(evaluated)
    }

    /// Resolve the enum symbol for an enum member expression.
    fn enum_symbol_for_member_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let Expression::Member {
            name,
            generic_arguments,
            left,
        } = ctx.tree.get(expression_id)
        else {
            return Ok(None);
        };
        if !generic_arguments.is_empty() {
            return Ok(None);
        }

        let left_expression = ctx.tree.get(*left);
        let mut enum_symbol = match left_expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            _ => return Ok(None),
        };
        // unwrap import/export dependency items for enum symbols
        if enum_symbol.module_id == ctx.module.id {
            let symbol_entry = ctx.symbols.get_symbol(enum_symbol.local_id);
            let dependency_id = symbol_entry.primary_declaration.and_then(|primary| {
                self.dependency_item_for_symbol(ctx.tree, primary, enum_symbol.local_id)
            });
            if let Some(dependency_id) = dependency_id
                && let DependencyItem::Local { target_symbol, .. }
                | DependencyItem::Remote { target_symbol, .. } = ctx.tree.get(dependency_id)
            {
                enum_symbol = *target_symbol;
            }
        }

        // ensure enum declarations are available before scanning enum fields
        if enum_symbol.module_id != ctx.module.id {
            self.require_dir_declared(
                ctx.compiler_context.revision(),
                enum_symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        }

        let Some(name) = *name else {
            return Ok(None);
        };

        if self
            .enum_field_symbol_for_name(ctx.type_view(), enum_symbol, name)?
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
        ctx: &mut InferContext<'_>,
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
                    function_id: Some(node_id.into_global(ctx.module.id)),
                };
                let var_id = ctx
                    .infer
                    .new_var(InferOrigin::TypeParameter(static_parameter.symbol), scope);
                let inferred_ty_id = ctx
                    .types
                    .insert_type_from_any(Type::InferVar { id: var_id }, node_id);
                ctx.infer.bind_type(var_id, inferred_ty_id);

                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
            StaticParameterKind::Value => {
                self.error(AnalyzeError::MissingStaticArgument {
                    node: node_id
                        .into_global(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
                let inferred_ty_id = ctx.types.insert_type_from_any(Type::Error, node_id);
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
        ctx: &mut InferContext<'_>,
        static_parameter: &StaticParameter,
        dynamic_parameters: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // infer direct type-parameter arguments from positional dynamic arguments
        if static_parameter.kind == StaticParameterKind::Type {
            for (param_ty_id, argument_id) in
                dynamic_parameters.iter().zip(dynamic_arguments.iter())
            {
                let param_ty_id = self.unwrap_type_value(*param_ty_id, ctx.types);
                let Type::Reference {
                    symbol,
                    static_arguments: None,
                } = ctx.types.get_type(param_ty_id)
                else {
                    continue;
                };
                if *symbol != static_parameter.symbol {
                    continue;
                }

                let Some(argument_ty_id) =
                    self.argument_type_for_static_inference(&ctx.reborrow(), *argument_id)
                else {
                    continue;
                };
                if !self.inferred_type_argument_is_committable(&ctx.reborrow(), argument_ty_id) {
                    continue;
                }

                let argument_source_id = ctx.tree.get(*argument_id).value().into_any();
                let argument_ty_id = self.regularize_constrained_type_argument_literal(
                    &mut ctx.type_context_reborrow(),
                    static_parameter,
                    argument_ty_id,
                    argument_source_id,
                );

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
            let param_ty_id = self.unwrap_type_value(*param_ty_id, ctx.types);
            let param_ty = ctx.types.get_type(param_ty_id).clone();

            // evaluate literal argument values when possible
            let expression_id = ctx.tree.get(*argument_id).value();
            let value = self.query_static_expression_value(
                &mut ctx.type_context_reborrow(),
                expression_id,
                None,
            )?;
            let value = if let Some(value) = value {
                value
            } else if let Some(enum_symbol) = self.enum_symbol_for_member_expression(
                &mut ctx.type_context_reborrow(),
                expression_id,
            )? {
                let ty = ctx.types.insert_type_from_any(
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

            if !self.static_value_argument_is_static(&value, ctx.type_view()) {
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
                && let Some(target_symbol) = self.unwrap_type_value_symbol(ctx.types, *count)
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
                && let Type::Reference { symbol, .. } = ctx.types.get_type(*index)
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
                self.argument_type_for_static_inference(&ctx.reborrow(), *argument_id)
            else {
                continue;
            };

            if let Some(argument) = self.infer_static_argument_from_argument_type(
                &mut ctx.type_context_reborrow(),
                static_parameter,
                *param_ty_id,
                argument_ty_id,
            )? {
                return Ok(Some(argument));
            }
        }

        Ok(None)
    }

    /// Return true when one unevaluated value static argument should wait for convergence.
    fn unevaluated_static_value_argument_requires_convergence(
        &self,
        ctx: &mut TypeContext<'_>,
        argument_node: GlobalNodeIdAny,
    ) -> AnalyzeResult<bool> {
        let Some(requires_convergence) = self.with_static_argument_owner(
            &mut ctx.reborrow(),
            argument_node,
            |ctx, argument_id| {
                let expression_id = ctx.tree.get(argument_id).value();
                let expression_node = expression_id.into_global_any(ctx.module.id);
                let argument_type_id = ctx.types.get_declared_or_inferred_type_id(expression_node);
                if let Some(argument_type_id) = argument_type_id {
                    let requires_convergence = self.type_requires_static_evaluation_convergence(
                        ctx.type_view(),
                        argument_type_id,
                    );
                    if !requires_convergence {
                        return Ok(false);
                    }

                    // associated type projections are type-space only and never become
                    // valid value static expressions through convergence
                    let mut visited = HashSet::new();
                    if self.type_contains_associated_type_reference(
                        ctx.tree_symbol_view(),
                        argument_type_id,
                        ctx.types,
                        &mut visited,
                    ) {
                        return Ok(false);
                    }

                    return Ok(true);
                }

                let symbolic_static_reference = self
                    .symbolic_static_value_symbol_for_expression(
                        &mut ctx.reborrow(),
                        expression_id,
                    )?
                    .is_some();
                if symbolic_static_reference {
                    return Ok(true);
                }

                Ok(false)
            },
        )?
        else {
            return Ok(false);
        };

        Ok(requires_convergence)
    }

    /// Return true when one unevaluated static value expression should wait for convergence.
    fn static_value_expression_requires_convergence(
        &self,
        ctx: TypeView<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if !ctx.tree.has_node_id(expression_id.id) {
            return false;
        }

        let expression_node = expression_id.into_global_any(ctx.module.id);
        let Some(expression_type_id) = ctx.types.get_declared_or_inferred_type_id(expression_node)
        else {
            return false;
        };

        let mut visited = HashSet::new();
        if self.type_contains_associated_type_reference(
            ctx.tree_symbol_view(),
            expression_type_id,
            ctx.types,
            &mut visited,
        ) {
            return false;
        }

        self.type_requires_static_evaluation_convergence(ctx, expression_type_id)
    }

    /// Regularize constrained static type arguments to non fresh literal precision.
    fn regularize_constrained_type_argument_literal(
        &self,
        ctx: &mut TypeContext<'_>,
        static_parameter: &StaticParameter,
        argument_ty_id: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        // resolve the constraint when the declared type is still symbolic
        let mut constraint_ty_id = static_parameter.declared_type_id;
        if matches!(
            ctx.types.get_type(constraint_ty_id),
            Type::Unevaluated(_)
                | Type::InferVar { .. }
                | Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any,
                }
        ) && self.symbol_is_static_parameter(ctx.symbol_type_view(), static_parameter.symbol)
        {
            let argument_source_id = ctx.types.get_type_source(argument_ty_id);
            if let Some(resolved_constraint_ty_id) = self.static_parameter_constraint_type(
                &mut ctx.reborrow(),
                static_parameter.symbol,
                argument_source_id,
            ) {
                constraint_ty_id = resolved_constraint_ty_id;
            }
        }

        // unconstrained type parameters should keep default widening behavior
        if matches!(
            ctx.types.get_type(constraint_ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown | TypeLiteral::Any,
            }
        ) {
            return argument_ty_id;
        }

        // detach from shared source slots before regularization
        let argument_ty_id = if ctx.types.get_type_source(argument_ty_id) == source_id {
            argument_ty_id
        } else {
            let ty = ctx.types.get_type(argument_ty_id).clone();
            ctx.types.insert_type_from_any(ty, source_id)
        };
        self.set_type_freshness(ctx.types, argument_ty_id, Freshness::Regular);

        argument_ty_id
    }

    /// Return true when one inferred type argument is stable enough for static argument commitment.
    fn inferred_type_argument_is_committable(
        &self,
        ctx: &InferContext<'_>,
        argument_ty_id: LocalTypeId,
    ) -> bool {
        // reject unresolved convergence state
        if self.type_requires_infer_convergence(ctx.type_view(), argument_ty_id) {
            return false;
        }

        // reject explicit error placeholders
        !matches!(ctx.types.get_type(argument_ty_id), Type::Error)
    }

    /// Resolve a value static argument from argument type metadata.
    fn infer_static_argument_from_argument_type(
        &self,
        ctx: &mut TypeContext<'_>,
        static_parameter: &StaticParameter,
        param_ty_id: LocalTypeId,
        argument_ty_id: LocalTypeId,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // unwrap any value wrappers before matching
        let param_ty_id = self.unwrap_type_value(param_ty_id, ctx.types);
        let argument_ty_id = self.unwrap_type_value(argument_ty_id, ctx.types);

        // extract reference arguments without holding immutable borrows
        let (param_symbol, param_arguments) = match ctx.types.get_type(param_ty_id) {
            Type::Reference {
                symbol,
                static_arguments: Some(arguments),
            } => (*symbol, arguments.clone()),
            _ => return Ok(None),
        };
        let (argument_symbol, argument_arguments) = match ctx.types.get_type(argument_ty_id) {
            Type::Reference {
                symbol,
                static_arguments: Some(arguments),
            } => (*symbol, arguments.clone()),
            _ => return Ok(None),
        };
        if param_symbol != argument_symbol {
            return Ok(None);
        }
        let argument_source_id = ctx.types.get_type_source(argument_ty_id);

        // materialize referenced static arguments with the owning module
        let resolved_arguments = if argument_symbol.module_id == ctx.module.id {
            self.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                argument_symbol,
                argument_source_id,
                &argument_arguments,
            )
        } else {
            self.with_declared_type_context_for_module(
                &mut ctx.reborrow(),
                argument_symbol.module_id,
                |remote_ctx| {
                    Ok(self.materialize_static_arguments_for_reference(
                        remote_ctx,
                        argument_symbol,
                        argument_source_id,
                        &argument_arguments,
                    ))
                },
            )?
        };

        // map explicit static arguments when the parameter and argument share a reference
        for (index, param_argument) in param_arguments.iter().enumerate() {
            if !self.static_argument_references_symbol(
                ctx.type_view(),
                param_argument,
                static_parameter.symbol,
            ) {
                continue;
            }

            let Some(argument) = resolved_arguments.get(index) else {
                continue;
            };
            let StaticArgument::Evaluated { value, .. } = argument else {
                continue;
            };
            if !self.static_value_argument_is_static(value, ctx.type_view()) {
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
        ctx: &InferContext<'_>,
        argument_id: LocalNodeId<Argument>,
    ) -> Option<LocalTypeId> {
        // prefer stable inferred types from this call site
        let argument = ctx.tree.get(argument_id);
        let value_id = argument.value();
        if let Some(type_id) = ctx
            .infer
            .inferred_type_for_node(value_id.into_global_any(ctx.module.id))
            && self.inferred_type_argument_is_committable(ctx, type_id)
        {
            return Some(type_id);
        }

        // then use symbol value types for direct references
        if let Some(symbol) = ctx.tree.get(value_id).target_symbol()
            && let Some(type_id) = ctx.types.get_type_id_for_symbol(ctx.symbols, symbol)
        {
            return Some(type_id);
        }

        // fall back to declaration owned types for direct references
        if let Some(symbol) = ctx.tree.get(value_id).target_symbol()
            && symbol.module_id == ctx.module.id
        {
            let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && let Some(type_id) = ctx
                    .infer
                    .inferred_type_for_node(primary_declaration)
                    .or_else(|| {
                        ctx.types
                            .get_declared_or_inferred_type_id(primary_declaration)
                    })
            {
                return Some(type_id);
            }
        }

        None
    }

    /// Check whether a static argument expression references a target symbol.
    fn static_argument_references_symbol(
        &self,
        ctx: TypeView<'_>,
        argument: &StaticArgument,
        target_symbol: GlobalSymbolId,
    ) -> bool {
        // accept evaluated references to the target symbol
        if let StaticArgument::Evaluated {
            value: StaticExpression::Type { ty },
            ..
        } = argument
            && let Type::Reference { symbol, .. } = ctx.types.get_type(*ty)
        {
            return *symbol == target_symbol;
        }

        // only unevaluated arguments carry expression nodes
        let StaticArgument::Unevaluated { node } = argument else {
            return false;
        };

        // resolve the owning tree before checking the argument expression
        let view = TreeSymbolView::new(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            ctx.tree,
            ctx.symbols,
        );
        self.with_static_argument_owner_read(view, *node, |view, argument_id| {
            let expression_id = view.tree.get(argument_id).value();
            Ok(self.reference_symbol_for_expression(view, expression_id) == Some(target_symbol))
        })
        .ok()
        .flatten()
        .unwrap_or(false)
    }

    /// Check whether a constraint satisfies a declared bound.
    fn constraint_satisfies_bound(
        &self,
        ctx: &mut TypeContext<'_>,
        expected_ty_id: LocalTypeId,
        constraint_ty_id: LocalTypeId,
    ) -> bool {
        // skip validation when the declared bound still depends on static parameters
        let mut visited = HashSet::new();
        let expected_contains_static =
            self.type_contains_static_parameters(ctx.type_view(), expected_ty_id, &mut visited);
        if expected_contains_static {
            return true;
        }

        // reject constraints that still depend on static parameters when the bound is concrete
        visited.clear();
        if self.type_contains_static_parameters(ctx.type_view(), constraint_ty_id, &mut visited) {
            return false;
        }

        // accept arguments whose constraints satisfy the expected bound
        self.is_type_assignable(&mut ctx.reborrow(), expected_ty_id, constraint_ty_id)
            .is_assignable()
    }

    /// Evaluate a static default expression for a parameter.
    pub(crate) fn evaluate_static_default_argument(
        &self,
        ctx: &mut TypeContext<'_>,
        parameter_kind: StaticParameterKind,
        name: Option<StringId>,
        default_expression: LocalNodeId<Expression>,
        treat_type_arguments_as_types: bool,
    ) -> AnalyzeResult<StaticArgument> {
        let mut default_expression =
            self.unwrap_parenthesized_expression(default_expression, ctx.tree);
        let mut is_explicit_comptime = false;
        loop {
            match ctx.tree.get(default_expression) {
                Expression::Comptime { body } => {
                    is_explicit_comptime = true;
                    default_expression = self.unwrap_parenthesized_expression(*body, ctx.tree);
                }
                _ => break,
            }
        }

        if parameter_kind == StaticParameterKind::Value && is_explicit_comptime {
            let error_type = ctx
                .types
                .insert_type_from_any(Type::Error, default_expression.into_any());
            return Ok(StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty: error_type },
            });
        }

        // prefer value defaults when type arguments stay unconverted
        if !treat_type_arguments_as_types
            && let Some(value) = self.evaluate_static_expression_value(
                &mut ctx.reborrow(),
                default_expression,
                None,
            )?
        {
            return Ok(StaticArgument::Evaluated { name, value });
        }

        // evaluate default value based on the parameter kind
        let value = match parameter_kind {
            StaticParameterKind::Type => {
                let Expression::Type {
                    value: type_expression_id,
                    ..
                } = ctx.tree.get(default_expression)
                else {
                    return Ok(StaticArgument::Evaluated {
                        name,
                        value: StaticExpression::Unevaluated {
                            node: default_expression,
                        },
                    });
                };
                let resolved = self.resolve_declared_type_expression_value(
                    &mut ctx.reborrow(),
                    *type_expression_id,
                    true,
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

                let ty_id = ctx.types.insert_type_from(resolved, default_expression);
                StaticExpression::Type { ty: ty_id }
            }
            StaticParameterKind::Value => {
                if let Some(value) = self.evaluate_static_expression_value(
                    &mut ctx.reborrow(),
                    default_expression,
                    None,
                )? {
                    value
                } else if let Some(enum_symbol) =
                    self.enum_symbol_for_member_expression(&mut ctx.reborrow(), default_expression)?
                {
                    let ty = ctx.types.insert_type_from_any(
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
