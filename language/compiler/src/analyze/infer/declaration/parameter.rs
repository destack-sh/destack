use crate::analyze::common::{TreeSymbolView, TypeContext, TypeView};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, FunctionSignature, GenericParameter, GlobalSymbolId, LocalNodeIdAny, LocalTypeId,
    Member, NodeType, StaticArgument, StaticExpression, StaticParameter, StaticParameterKind,
    StaticProperty, Type, TypeLiteral, TypeTable, VarianceModifier,
};
use std::collections::HashSet;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Query one declare-published static-parameter constraint entry.
    pub(crate) fn query_declared_static_parameter_constraint(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        if symbol.module_id == ctx.module.id && ctx.types.module_id == ctx.module.id {
            return ctx
                .types
                .query_artifact_static_parameter_constraint_type(symbol);
        }

        let remote_constraint = self
            .with_module_types_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.types,
                destack_artifact::ArtifactKey::dir_declared,
                |_owner_module, owner_types| {
                    let owner_constraint_type_id =
                        owner_types.query_artifact_static_parameter_constraint_type(symbol)?;
                    let owner_constraint_type = owner_types.get_type(owner_constraint_type_id);
                    let owner_snapshot = owner_types.clone();
                    Some((owner_constraint_type.clone(), owner_snapshot))
                },
            )
            .ok()
            .flatten()?;

        let (owner_constraint_type, owner_snapshot) = remote_constraint;
        Some(self.import_remote_type_for_node(
            source_id,
            &owner_constraint_type,
            &owner_snapshot,
            ctx.types,
        ))
    }

    /// Build and cache one static-parameter-constraint cycle error type.
    fn static_parameter_constraint_cycle_error_type(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        if let Some(cached_type_id) = ctx.types.get_static_parameter_constraint_type(symbol) {
            return cached_type_id;
        }

        self.error(AnalyzeError::CircularStaticArgument {
            node: source_id.into_anchored(ctx.module.id, Some(ctx.profile)),
        });

        let error_type_id = ctx.types.insert_type_from_any(Type::Error, source_id);
        ctx.types
            .set_static_parameter_constraint_type(symbol, error_type_id);

        error_type_id
    }

    /// Resolve local static parameter metadata from one module tree symbol.
    pub(crate) fn static_parameter_metadata_for_symbol_in_module(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> (StaticParameterKind, Option<VarianceModifier>) {
        let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
        if !symbol_entry.is_static_parameter() {
            return (StaticParameterKind::Type, None);
        }

        let Some(primary) = symbol_entry.primary_declaration else {
            return (StaticParameterKind::Type, None);
        };
        let Ok(parameter_id) = primary.local_id.try_into_typed::<GenericParameter>() else {
            return (StaticParameterKind::Type, None);
        };
        let parameter = ctx.tree.get(parameter_id);
        let kind = self.static_parameter_kind_for_generic_parameter(parameter);
        let variance = match parameter {
            GenericParameter::Type { variance, .. } => *variance,
            GenericParameter::Value { .. } | GenericParameter::Error { .. } => None,
        };

        (kind, variance)
    }

    /// Resolve the static parameter kind for a symbol.
    pub(crate) fn static_parameter_kind_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
    ) -> StaticParameterKind {
        // reuse cached kinds when available
        if let Some(kind) = ctx.types.get_static_parameter_kind(symbol) {
            return kind;
        }

        // resolve from published declare entries first
        if let Ok(Some(kind)) = self.with_module_types_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            symbol.module_id,
            ctx.types,
            destack_artifact::ArtifactKey::dir_declared,
            |_owner_module, owner_types| owner_types.query_artifact_static_parameter_kind(symbol),
        ) {
            ctx.types.set_static_parameter_kind(symbol, kind);
            return kind;
        }

        // remote symbols consume only published declare entries
        if symbol.module_id != ctx.module.id || ctx.types.module_id != ctx.module.id {
            return StaticParameterKind::Type;
        }

        // derive from local declaration metadata
        let (kind, _) =
            self.static_parameter_metadata_for_symbol_in_module(ctx.tree_symbol_view(), symbol);

        // cache resolved kinds
        ctx.types.set_static_parameter_kind(symbol, kind);
        kind
    }

    /// Resolve the static parameter variance for a symbol.
    pub(crate) fn static_parameter_variance_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<VarianceModifier> {
        // reuse cached variance when available
        if let Some(variance) = ctx.types.get_static_parameter_variance(symbol) {
            return variance;
        }

        // resolve from published declare entries first
        if let Ok(Some(variance)) = self.with_module_types_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            symbol.module_id,
            ctx.types,
            destack_artifact::ArtifactKey::dir_declared,
            |_owner_module, owner_types| {
                owner_types
                    .query_artifact_static_parameter_variance(symbol)
                    .flatten()
            },
        ) {
            ctx.types
                .set_static_parameter_variance(symbol, Some(variance));
            return Some(variance);
        }

        // remote symbols consume only published declare entries
        if symbol.module_id != ctx.module.id || ctx.types.module_id != ctx.module.id {
            return None;
        }

        // derive from local declaration metadata
        let (_, variance) =
            self.static_parameter_metadata_for_symbol_in_module(ctx.tree_symbol_view(), symbol);

        // cache resolved variance
        ctx.types.set_static_parameter_variance(symbol, variance);
        variance
    }

    /// Resolve static parameter kind and variance for one symbol.
    pub(crate) fn static_parameter_metadata_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
    ) -> (Option<StaticParameterKind>, Option<VarianceModifier>) {
        let kind = self.static_parameter_kind_for_symbol(&mut ctx.reborrow(), symbol);
        let variance = self.static_parameter_variance_for_symbol(&mut ctx.reborrow(), symbol);
        (Some(kind), variance)
    }

    /// Index static parameter symbols and metadata declared in one module.
    pub(crate) fn index_static_parameter_metadata(&self, state: &mut TypeContext<'_>) {
        for symbol_id in state.symbols.active_symbol_ids() {
            let symbol = symbol_id.into_global(state.module.id);
            let Some(parameters) =
                self.collect_static_parameter_symbols_in_module(state.tree_symbol_view(), symbol)
            else {
                continue;
            };

            state
                .types
                .set_artifact_static_parameter_symbols(symbol, parameters.clone());
            for parameter_symbol in parameters {
                let (kind, variance) = self.static_parameter_metadata_for_symbol_in_module(
                    state.tree_symbol_view(),
                    parameter_symbol,
                );

                state
                    .types
                    .set_artifact_static_parameter_kind(parameter_symbol, kind);
                state
                    .types
                    .set_artifact_static_parameter_variance(parameter_symbol, variance);
            }
        }
    }

    /// Publish declared static parameter constraints for one module.
    pub(crate) fn record_artifact_static_parameter_constraints(
        &self,
        ctx: &mut TypeContext<'_>,
    ) -> AnalyzeResult<()> {
        for symbol_id in ctx.symbols.active_symbol_ids() {
            let symbol = symbol_id.into_global(ctx.module.id);
            let symbol_entry = ctx.symbols.get_symbol(symbol_id);
            if !symbol_entry.is_static_parameter() {
                continue;
            }

            let source_id = symbol_entry
                .primary_declaration
                .map(|declaration| declaration.local_id)
                .unwrap_or(ctx.anchor_node());
            let artifact_constraint_type_id = if let Some(primary_declaration) =
                symbol_entry.primary_declaration
                && let Some(declared_type_id) = ctx.types.get_declared_type_id(primary_declaration)
            {
                if matches!(ctx.types.get_type(declared_type_id), Type::Unevaluated(_)) {
                    self.evaluate_static_parameter_constraint_type(
                        &mut ctx.reborrow(),
                        declared_type_id,
                    )?;
                }

                if matches!(ctx.types.get_type(declared_type_id), Type::Unevaluated(_)) {
                    ctx.types.insert_type_from_any(Type::Error, source_id)
                } else {
                    declared_type_id
                }
            } else {
                self.synthesize_implicit_static_parameter_constraint(source_id, ctx.types)
            };
            ctx.types
                .set_artifact_static_parameter_constraint_type(symbol, artifact_constraint_type_id);
        }

        Ok(())
    }

    /// Resolve the static parameter kind from one generic parameter node.
    fn static_parameter_kind_for_generic_parameter(
        &self,
        parameter: &GenericParameter,
    ) -> StaticParameterKind {
        match parameter {
            GenericParameter::Type { .. } | GenericParameter::Error { .. } => {
                StaticParameterKind::Type
            }
            GenericParameter::Value { .. } => StaticParameterKind::Value,
        }
    }

    /// Build static parameter placeholders for a function signature.
    pub(crate) fn static_parameter_placeholders_for_signature(
        &self,
        ctx: &mut TypeContext<'_>,
        signature: &FunctionSignature,
    ) -> Vec<LocalTypeId> {
        // stop when the signature has no generic parameters
        if signature.generic_parameters.is_empty() {
            return Vec::new();
        }

        // map static parameters to reference placeholders
        let mut placeholders = Vec::with_capacity(signature.generic_parameters.len());
        for parameter_id in &signature.generic_parameters {
            let symbol = ctx
                .tree
                .get(*parameter_id)
                .symbol()
                .into_global(ctx.module.id);
            let ty = Type::Reference {
                symbol,
                static_arguments: None,
            };
            let type_id = ctx.types.insert_type_from(ty, *parameter_id);
            placeholders.push(type_id);
        }

        placeholders
    }

    /// Collect static parameter symbols for a declaration symbol.
    pub(crate) fn collect_static_parameter_symbols(
        &self,
        ctx: TypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<Vec<GlobalSymbolId>> {
        if let Some(cached) = self
            .with_module_types_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.types,
                destack_artifact::ArtifactKey::dir_declared,
                |_, owner_types| owner_types.query_artifact_static_parameter_symbols(symbol),
            )
            .ok()
            .flatten()
        {
            return Some(cached);
        }

        // follow imports until a declaration provides static parameters
        let mut current = symbol;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                break None;
            }

            if current.module_id == ctx.module.id {
                if let Some(cached) = ctx.types.query_artifact_static_parameter_symbols(current) {
                    break Some(cached);
                }

                if let Some(parameters) =
                    self.collect_static_parameter_symbols_in_module(ctx.tree_symbol_view(), current)
                {
                    break Some(parameters);
                }

                let symbol_entry = ctx.symbols.get_symbol(current.local_id);
                let next = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
                let next = match next {
                    Some(next) => next,
                    None => break None,
                };
                current = next;
                continue;
            }

            let Ok(owner_dir) = self.require_artifact_dir_declared(
                ctx.compiler_context.revision(),
                current.module_id,
                ctx.profile,
            ) else {
                break None;
            };
            let owner_module = ctx.compiler_context.module(current.module_id);
            let owner_module = owner_module.as_ref();
            let (parameters, next) = if let Some(cached) = owner_dir
                .types
                .query_artifact_static_parameter_symbols(current)
            {
                (Some(cached), None)
            } else if let Some(parameters) = self.collect_static_parameter_symbols_in_module(
                TreeSymbolView::new(
                    ctx.compiler_context,
                    owner_module,
                    ctx.profile,
                    &owner_dir.tree,
                    &owner_dir.symbols,
                ),
                current,
            ) {
                (Some(parameters), None)
            } else {
                let symbol_entry = owner_dir.symbols.get_symbol(current.local_id);
                (
                    None,
                    symbol_entry.target_symbol.or(symbol_entry.canonical_symbol),
                )
            };
            if let Some(parameters) = parameters {
                break Some(parameters);
            }

            let next = match next {
                Some(next) => next,
                None => break None,
            };
            current = next;
        }
    }

    /// Collect static parameter symbols for a declaration in a module.
    fn collect_static_parameter_symbols_in_module(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<Vec<GlobalSymbolId>> {
        // extract the declaration for the symbol
        let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let parameters = match primary_declaration.local_id.ty {
            // declaration static parameters
            NodeType::Declaration => {
                let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                let declaration = ctx.tree.get(declaration_id);
                match declaration {
                    Declaration::Namespace(declaration) => {
                        declaration.generic_parameters.as_slice()
                    }
                    Declaration::Type(declaration) => declaration.generic_parameters.as_slice(),
                    Declaration::Struct(declaration) => declaration.generic_parameters.as_slice(),
                    Declaration::Class(declaration) => declaration.generic_parameters.as_slice(),
                    Declaration::Enum(declaration) => declaration.generic_parameters.as_slice(),
                    Declaration::Interface(declaration) => {
                        declaration.generic_parameters.as_slice()
                    }
                    Declaration::Extension(declaration) => {
                        declaration.generic_parameters.as_slice()
                    }
                    Declaration::Function(declaration) => {
                        declaration.signature.generic_parameters.as_slice()
                    }
                    Declaration::Global(_) | Declaration::ImportAlias(_) => return None,
                }
            }
            // associated type static parameters
            NodeType::Member => {
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                let member = ctx.tree.get(member_id);
                match member {
                    Member::AssociatedType {
                        generic_parameters, ..
                    } => generic_parameters.as_slice(),
                    Member::Method { signature, .. } => signature.generic_parameters.as_slice(),
                    _ => return None,
                }
            }
            _ => return None,
        };

        // map parameter nodes to global symbols
        let symbols = parameters
            .iter()
            .map(|parameter_id| {
                let parameter: &GenericParameter = ctx.tree.get(*parameter_id);
                let symbol_id = parameter.symbol();
                let symbol_entry = ctx.symbols.get_symbol(symbol_id);
                GlobalSymbolId::new(ctx.module.id, symbol_id.with_type(symbol_entry.ty))
            })
            .collect();

        Some(symbols)
    }

    /// Resolve static parameter metadata for a symbol (in this or another module).
    pub(crate) fn resolve_static_parameter(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol_id: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> StaticParameter {
        // prefer parameter metadata from the owning module
        let parameter = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol_id.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| {
                    let owner_options = view
                        .compiler_context
                        .analyze_context_options_for_module(view.module.id);
                    let mut ctx = TypeContext::new(
                        ctx.compiler_context,
                        view.module,
                        ctx.profile,
                        &owner_options,
                        view.tree,
                        view.symbols,
                        ctx.types,
                        ctx.index.clone(),
                    );
                    self.resolve_static_parameter_in_module(&mut ctx, symbol_id, source_id)
                },
            )
            .ok()
            .flatten();

        // recover with error metadata when declaration lookup does not resolve
        parameter.unwrap_or_else(|| {
            let kind = self.static_parameter_kind_for_symbol(&mut ctx.reborrow(), symbol_id);
            self.synthesize_error_static_parameter(symbol_id, kind, source_id, ctx.types)
        })
    }

    /// Synthesize an error static parameter when internal metadata resolution fails.
    pub(crate) fn synthesize_error_static_parameter(
        &self,
        symbol: GlobalSymbolId,
        kind: StaticParameterKind,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> StaticParameter {
        let error_ty_id = self.synthesize_semantic_error_type_for_source(source_id, types);

        StaticParameter {
            symbol,
            name: None,
            declared_type_id: error_ty_id,
            default_expression: None,
            kind,
        }
    }

    /// Synthesize the implicit unconstrained static-parameter bound (`unknown`) for one source.
    fn synthesize_implicit_static_parameter_constraint(
        &self,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        )
    }

    /// Synthesize the semantic `error` type id for one source node.
    fn synthesize_semantic_error_type_for_source(
        &self,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        types.insert_type_from_any(Type::Error, source_id)
    }

    /// Evaluate a static parameter constraint while preserving symbolic bounds.
    fn evaluate_static_parameter_constraint_type(
        &self,
        ctx: &mut TypeContext<'_>,
        declared_type_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        // skip when the declared type is already evaluated
        let expression_id = match ctx.types.get_type(declared_type_id) {
            Type::Unevaluated(expression_id) => *expression_id,
            _ => return Ok(()),
        };

        // evaluate once without resolving static arguments to keep symbolic structure
        let raw_evaluated = self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            expression_id,
            false,
            true,
            false,
            false,
            true,
        )?;

        // stage the raw evaluation result in the declared type slot
        ctx.types.update_type(declared_type_id, raw_evaluated);

        // stop here when the bound still depends on static parameters
        let mut visited = HashSet::new();
        if self.type_contains_static_parameters(ctx.type_view(), declared_type_id, &mut visited) {
            return Ok(());
        }

        // otherwise resolve static arguments now that the constraint is independent
        let resolved = self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            expression_id,
            false,
            true,
            true,
            false,
            true,
        )?;
        ctx.types.update_type(declared_type_id, resolved);

        Ok(())
    }

    /// Resolve a static parameter constraint into the local type table.
    pub(crate) fn static_parameter_constraint_type(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        // reuse cached constraints when available
        if let Some(cached) = ctx.types.get_static_parameter_constraint_type(symbol) {
            return Some(cached);
        }

        // resolve local constraints with local on-demand evaluation
        let resolved = if symbol.module_id == ctx.module.id && ctx.types.module_id == ctx.module.id
        {
            if ctx.types.is_static_parameter_constraint_in_progress(symbol) {
                return Some(self.static_parameter_constraint_cycle_error_type(
                    &mut ctx.reborrow(),
                    symbol,
                    source_id,
                ));
            }
            ctx.types
                .mark_static_parameter_constraint_in_progress(symbol);

            let local_resolved = {
                let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
                if !symbol_entry.is_static_parameter() {
                    Some(self.synthesize_semantic_error_type_for_source(source_id, ctx.types))
                } else if let Some(primary_declaration) = symbol_entry.primary_declaration {
                    let declared_type_id = if let Some(declared_type_id) =
                        ctx.types.get_declared_type_id(primary_declaration)
                    {
                        declared_type_id
                    } else {
                        self.synthesize_implicit_static_parameter_constraint(source_id, ctx.types)
                    };
                    if matches!(ctx.types.get_type(declared_type_id), Type::Unevaluated(_)) {
                        let mut ctx = ctx.reborrow();
                        if let Err(error) = self
                            .evaluate_static_parameter_constraint_type(&mut ctx, declared_type_id)
                        {
                            self.error(error);
                            Some(
                                self.synthesize_semantic_error_type_for_source(
                                    source_id, ctx.types,
                                ),
                            )
                        } else if matches!(
                            ctx.types.get_type(declared_type_id),
                            Type::Unevaluated(_)
                        ) {
                            Some(
                                self.synthesize_semantic_error_type_for_source(
                                    source_id, ctx.types,
                                ),
                            )
                        } else {
                            Some(declared_type_id)
                        }
                    } else {
                        Some(declared_type_id)
                    }
                } else {
                    Some(self.synthesize_semantic_error_type_for_source(source_id, ctx.types))
                }
            };
            ctx.types
                .clear_static_parameter_constraint_in_progress(symbol);

            // preserve cycle-error diagnostics emitted during recursive evaluation
            if let Some(cached_type_id) = ctx.types.get_static_parameter_constraint_type(symbol)
                && matches!(ctx.types.get_type(cached_type_id), Type::Error)
            {
                return Some(cached_type_id);
            }

            local_resolved
        } else {
            self.query_declared_static_parameter_constraint(&mut ctx.reborrow(), symbol, source_id)
        };
        if let Some(resolved_constraint_type_id) = resolved {
            ctx.types
                .set_static_parameter_constraint_type(symbol, resolved_constraint_type_id);
            return Some(resolved_constraint_type_id);
        }

        let error_type_id = self.synthesize_semantic_error_type_for_source(source_id, ctx.types);
        ctx.types
            .set_static_parameter_constraint_type(symbol, error_type_id);
        Some(error_type_id)
    }

    /// Resolve static parameter metadata from a module.
    fn resolve_static_parameter_in_module(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol_id: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> Option<StaticParameter> {
        // read the parameter declaration
        let symbol = ctx.symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        let parameter_id = primary_declaration
            .try_into_local_typed::<GenericParameter>()
            .ok()?;
        let parameter = ctx.tree.get(parameter_id);

        // resolve the declared type for the parameter
        let declared_type_id = if ctx.module.id == ctx.types.module_id {
            ctx.types
                .get_declared_type_id(primary_declaration)
                .unwrap_or_else(|| {
                    self.synthesize_implicit_static_parameter_constraint(
                        parameter_id.into_any(),
                        ctx.types,
                    )
                })
        } else {
            // import the declared type for remote parameters when possible
            // remote modules are read-only here: do not force declaration evaluation
            let remote_declared = {
                let remote_dir = self
                    .require_indexed_dir_declared(
                        &ctx.index,
                        ctx.compiler_context.revision(),
                        primary_declaration.module_id,
                        ctx.profile,
                    )
                    .ok()?;
                let remote_types = &remote_dir.types;
                remote_types.get_declared_type_id(primary_declaration).map(
                    |remote_declared_type_id| {
                        if matches!(
                            remote_types.get_type(remote_declared_type_id),
                            Type::Unevaluated(_)
                        ) {
                            None
                        } else {
                            let remote_declared_type =
                                remote_types.get_type(remote_declared_type_id).clone();
                            let remote_snapshot = remote_types.clone();
                            Some((remote_declared_type, remote_snapshot))
                        }
                    },
                )
            };

            match remote_declared {
                Some(Some((remote_declared_type, remote_snapshot))) => self
                    .import_remote_type_for_node(
                        source_id,
                        &remote_declared_type,
                        &remote_snapshot,
                        ctx.types,
                    ),
                Some(None) => self.synthesize_semantic_error_type_for_source(source_id, ctx.types),
                None => self.synthesize_implicit_static_parameter_constraint(source_id, ctx.types),
            }
        };

        // resolve the static parameter kind from the generic parameter
        let kind = self.static_parameter_kind_for_generic_parameter(parameter);

        // derive the parameter name for mapping
        let name = match parameter {
            GenericParameter::Type { name, .. } | GenericParameter::Value { name, .. } => {
                Some(*name)
            }
            GenericParameter::Error { .. } => None,
        };

        // resolve the default expression for value parameters
        let default_expression = match parameter {
            GenericParameter::Value { default, .. } => {
                default.map(|expression_id| expression_id.into_global(ctx.module.id))
            }
            GenericParameter::Type { .. } | GenericParameter::Error { .. } => None,
        };

        Some(StaticParameter {
            symbol: symbol_id,
            name,
            declared_type_id,
            default_expression,
            kind,
        })
    }

    /// Collect static parameter symbols referenced in a type.
    pub(crate) fn collect_type_reference_symbols(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        if !visited.insert(ty_id) {
            return;
        }
        match types.get_type(ty_id) {
            Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::Error | Type::This => {}
            Type::Value { value } => {
                self.collect_type_reference_symbols(*value, types, symbols, visited);
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                symbols.insert(*symbol);
                if let Some(static_arguments) = static_arguments {
                    for argument in static_arguments {
                        self.collect_type_reference_symbols_from_static_argument(
                            argument, types, symbols, visited,
                        );
                    }
                }
            }
            Type::Unevaluated(_) => {}
            Type::Readonly { target_type }
            | Type::KeyOf { target_type }
            | Type::Must { target_type }
            | Type::AsComptime { target_type }
            | Type::Not { target_type } => {
                self.collect_type_reference_symbols(*target_type, types, symbols, visited);
            }
            Type::In { left, right }
            | Type::Extends { left, right }
            | Type::Implements { left, right } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*right, types, symbols, visited);
                self.collect_type_reference_symbols(*then_type, types, symbols, visited);
                self.collect_type_reference_symbols(*else_type, types, symbols, visited);
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.collect_type_reference_symbols(parameter.constraint, types, symbols, visited);
                if let Some(key_remap) = parameter.key_remap {
                    self.collect_type_reference_symbols(key_remap, types, symbols, visited);
                }
                self.collect_type_reference_symbols(*value, types, symbols, visited);
            }
            Type::Index { left, index } => {
                self.collect_type_reference_symbols(*left, types, symbols, visited);
                self.collect_type_reference_symbols(*index, types, symbols, visited);
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.collect_type_reference_symbols(*span, types, symbols, visited);
                }
            }
            Type::Import { .. } => {}
            Type::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.collect_type_reference_symbols(*constraint, types, symbols, visited);
                }
            }
            Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.collect_type_reference_symbols(*target, types, symbols, visited);
                }
            }
            Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                self.collect_type_reference_symbols(*right, types, symbols, visited);
            }
            Type::ArraySized { element, .. } => {
                self.collect_type_reference_symbols(*element, types, symbols, visited);
            }
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    self.collect_type_reference_symbols(element.ty, types, symbols, visited);
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.collect_type_reference_symbols(field.ty, types, symbols, visited);
                }
                for signature in call_signatures {
                    self.collect_type_reference_symbols(*signature, types, symbols, visited);
                }
                for signature in construct_signatures {
                    self.collect_type_reference_symbols(*signature, types, symbols, visited);
                }
                for signature in index_signatures {
                    self.collect_type_reference_symbols(
                        signature.key_type,
                        types,
                        symbols,
                        visited,
                    );
                    self.collect_type_reference_symbols(
                        signature.value_type,
                        types,
                        symbols,
                        visited,
                    );
                }
            }
            Type::Function {
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                if let Some(this_parameter) = this_parameter {
                    self.collect_type_reference_symbols(*this_parameter, types, symbols, visited);
                }
                for parameter in dynamic_parameters {
                    self.collect_type_reference_symbols(*parameter, types, symbols, visited);
                }
                if let Some(return_type) = return_type {
                    self.collect_type_reference_symbols(*return_type, types, symbols, visited);
                }
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols(*element, types, symbols, visited);
                }
            }
        }
    }

    /// Collect static parameter symbols referenced in a static argument.
    pub(crate) fn collect_type_reference_symbols_from_static_argument(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match argument {
            StaticArgument::Unevaluated { .. } => {}
            StaticArgument::Evaluated { value, .. } => {
                self.collect_type_reference_symbols_from_static_expression(
                    value, types, symbols, visited,
                );
            }
        }
    }

    /// Collect static parameter symbols referenced in a static expression.
    pub(crate) fn collect_type_reference_symbols_from_static_expression(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match expression {
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. } => {}
            StaticExpression::Type { ty } => {
                self.collect_type_reference_symbols(*ty, types, symbols, visited);
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => {
                if let Some(static_arguments) = static_arguments {
                    for argument in static_arguments {
                        self.collect_type_reference_symbols_from_static_argument(
                            argument, types, symbols, visited,
                        );
                    }
                }
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => {
                for element in elements {
                    self.collect_type_reference_symbols_from_static_expression(
                        element, types, symbols, visited,
                    );
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                for property in properties {
                    self.collect_type_reference_symbols_from_static_property(
                        property, types, symbols, visited,
                    );
                }
            }
        }
    }

    /// Collect static parameter symbols referenced in a static property.
    pub(crate) fn collect_type_reference_symbols_from_static_property(
        &self,
        property: &StaticProperty,
        types: &TypeTable,
        symbols: &mut HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        match property {
            StaticProperty::Unevaluated { .. } => {}
            StaticProperty::Field { value, .. } => {
                self.collect_type_reference_symbols_from_static_expression(
                    value, types, symbols, visited,
                );
            }
            StaticProperty::Method { body, .. } => {
                self.collect_type_reference_symbols_from_static_expression(
                    body, types, symbols, visited,
                );
            }
            StaticProperty::Spread { value, .. } => {
                self.collect_type_reference_symbols_from_static_expression(
                    value, types, symbols, visited,
                );
            }
        }
    }
}
