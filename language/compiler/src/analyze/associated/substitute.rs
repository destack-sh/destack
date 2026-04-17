use std::collections::{HashMap, HashSet};

use super::resolve::{AssociatedAliasProjectionRewriter, ProjectionEnvironment};
use crate::analyze::common::{
    CanonicalSymbolMode, RelationMode, TreeSymbolView, TypeContext, TypeRewriteCache,
};
use crate::analyze::declare::StaticConstantResolutionMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, GlobalNodeId, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId,
    MappedTypeParameter, Member, NodeType, NormalizationMode, StaticArgument, StaticExpression,
    StaticKey, SymbolType, TupleElement, Type, TypeExpression, TypeLiteral, TypeRewriter,
    TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn rewrite_associated_aliases_for_owner(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // skip when there is no owner receiver context to rewrite against
        if substitutions.is_empty() && receiver_symbol.is_none() {
            return type_id;
        }

        // rewrite associated aliases for the owner in one pass
        let mut rewriter = AssociatedAliasProjectionRewriter::new(
            self,
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            source_id,
            owner_symbol,
            receiver_symbol,
            receiver_arguments,
            substitutions,
            ctx.tree,
            ctx.symbols,
        );

        rewriter.rewrite_type_id(ctx.types, type_id)
    }

    /// Resolve receiver substitutions for an associated projection owner.
    pub(crate) fn receiver_projection_substitutions_for_owner(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        owner_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        let canonical_receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let canonical_receiver_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), canonical_receiver_symbol)
            .unwrap_or(canonical_receiver_symbol);
        let owner_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
            .unwrap_or(owner_symbol);
        let receiver_substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            canonical_receiver_symbol,
            source_id,
            receiver_arguments,
        );

        let mut visited_symbols = HashSet::new();
        self.associated_projection_receiver_substitutions_inner(
            &mut ctx.reborrow(),
            source_id,
            canonical_receiver_symbol,
            owner_symbol,
            receiver_substitutions,
            &mut visited_symbols,
        )
    }

    /// Resolve one static-parameter constraint for projection traversal.
    /// remote constraints must come from declare-published commitments
    /// local constraints may use local infer-owned cache/evaluation
    pub(super) fn projection_static_parameter_constraint_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        static_parameter_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        if let Some(constraint_type_id) = self.query_declared_static_parameter_constraint(
            &mut ctx.reborrow(),
            static_parameter_symbol,
            source_id,
        ) {
            return Ok(Some(constraint_type_id));
        }

        if static_parameter_symbol.module_id == ctx.module.id {
            let constraint_type_id = self.static_parameter_constraint_type(
                &mut ctx.reborrow(),
                static_parameter_symbol,
                source_id,
            );
            return Ok(constraint_type_id);
        }

        Ok(None)
    }

    /// Resolve receiver substitutions for associated projections along heritage edges.
    fn associated_projection_receiver_substitutions_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        current_symbol: GlobalSymbolId,
        owner_symbol: GlobalSymbolId,
        current_substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        // stop when we reached the associated member owner
        if current_symbol == owner_symbol {
            return Ok(Some(current_substitutions));
        }

        // stop recursive cycles in lineage traversal
        if !visited_symbols.insert(current_symbol) {
            return Ok(None);
        }
        // follow static parameter constraints before declaration heritage traversal
        if self.symbol_is_static_parameter(ctx.symbol_type_view(), current_symbol) {
            let constraint_type_id = self.projection_static_parameter_constraint_type(
                &mut ctx.reborrow(),
                source_id,
                current_symbol,
            )?;
            if let Some(mut constraint_type_id) = constraint_type_id {
                if !current_substitutions.is_empty() {
                    let mut substitution_cache = HashMap::new();
                    constraint_type_id = self.substitute_static_parameters(
                        constraint_type_id,
                        &current_substitutions,
                        ctx.types,
                        &mut substitution_cache,
                    );
                }

                let mut materialize_cache = TypeRewriteCache::new();
                constraint_type_id = self.materialize_static_arguments_in_type(
                    &mut ctx.reborrow(),
                    constraint_type_id,
                    &mut materialize_cache,
                );

                if let Some((mut next_symbol, next_arguments, _)) =
                    self.unwrap_type_symbol(ctx.types, constraint_type_id)
                {
                    next_symbol = self.canonical_symbol_id(
                        ctx.module_symbol_view(),
                        next_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    next_symbol = self
                        .declaration_symbol_id(ctx.module_symbol_view(), next_symbol)
                        .unwrap_or(next_symbol);

                    let next_substitutions = self.build_type_parameter_substitutions_for_symbol(
                        &mut ctx.reborrow(),
                        next_symbol,
                        source_id,
                        next_arguments.as_deref().unwrap_or_default(),
                    );
                    if let Some(substitutions) = self
                        .associated_projection_receiver_substitutions_inner(
                            &mut ctx.reborrow(),
                            source_id,
                            next_symbol,
                            owner_symbol,
                            next_substitutions,
                            visited_symbols,
                        )?
                    {
                        return Ok(Some(substitutions));
                    }
                }
            }
        }

        // collect direct heritage expressions for this symbol
        let heritage_expressions =
            self.heritage_expressions_for_associated_projection(&*ctx, current_symbol)?;
        for heritage_expression_id in heritage_expressions {
            // resolve the heritage target and applied arguments
            let resolved_heritage = self
                .with_module_tree_symbol_view_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    heritage_expression_id.module_id,
                    ctx.tree,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |view| -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
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
                        let expression_id = heritage_expression_id.local_id;
                        let expression = view.tree.get(expression_id);
                        let expression_has_generic_arguments = expression
                            .generic_arguments()
                            .is_some_and(|arguments| !arguments.is_empty());
                        let Some(target_symbol) = expression.target_symbol() else {
                            return Ok(None);
                        };
                        let view = view.module_symbol_view();
                        let target_symbol = self.canonical_symbol_id(
                            view,
                            target_symbol,
                            CanonicalSymbolMode::FollowAliases,
                        );

                        let expression_global_id = expression_id.into_global_any(view.module.id);
                        let mut heritage_type_id = if let Some(type_id) = ctx
                            .types
                            .get_inferred_type_id(expression_global_id)
                            .or_else(|| ctx.types.get_declared_type_id(expression_global_id))
                        {
                            type_id
                        } else {
                            self.resolve_declared_type_expression(
                                &mut ctx.reborrow(),
                                expression_id,
                                true,
                                true,
                            )?
                        };

                        if !current_substitutions.is_empty() {
                            let mut substitution_cache = HashMap::new();
                            heritage_type_id = self.substitute_static_parameters(
                                heritage_type_id,
                                &current_substitutions,
                                ctx.types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        heritage_type_id = self.materialize_static_arguments_in_type(
                            &mut ctx.reborrow(),
                            heritage_type_id,
                            &mut materialize_cache,
                        );

                        if let Some((resolved_symbol, resolved_arguments, _)) =
                            self.unwrap_type_symbol(ctx.types, heritage_type_id)
                        {
                            let mut resolved_arguments = resolved_arguments.unwrap_or_default();
                            if resolved_arguments.is_empty() && expression_has_generic_arguments {
                                let evaluated_arguments = self.evaluate_generic_arguments(
                                    &mut ctx.reborrow(),
                                    expression.generic_arguments(),
                                )?;
                                let evaluated_arguments = evaluated_arguments.unwrap_or_default();
                                let resolved_static_arguments = self
                                    .resolve_type_reference_static_arguments(
                                        &mut ctx.reborrow(),
                                        source_id,
                                        target_symbol,
                                        Some(evaluated_arguments.as_slice()),
                                        true,
                                    )?;
                                resolved_arguments =
                                    resolved_static_arguments.unwrap_or(evaluated_arguments);
                            }

                            return Ok(Some((resolved_symbol, resolved_arguments)));
                        }

                        Ok(Some((target_symbol, Vec::new())))
                    },
                )
                .map_err(AnalyzeError::from)??;
            let Some((next_symbol, next_arguments)) = resolved_heritage else {
                continue;
            };

            // map the resolved arguments onto the next symbol parameters
            let next_substitutions = self.build_type_parameter_substitutions_for_symbol(
                &mut ctx.reborrow(),
                next_symbol,
                source_id,
                &next_arguments,
            );
            if let Some(substitutions) = self.associated_projection_receiver_substitutions_inner(
                &mut ctx.reborrow(),
                source_id,
                next_symbol,
                owner_symbol,
                next_substitutions,
                visited_symbols,
            )? {
                return Ok(Some(substitutions));
            }
        }

        Ok(None)
    }

    /// Collect direct heritage expressions for associated projection traversal.
    fn heritage_expressions_for_associated_projection(
        &self,
        ctx: &TypeContext<'_>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<GlobalNodeId<TypeExpression>>> {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                let symbol_entry = view.symbols.get_symbol(symbol.local_id);
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return Vec::new();
                };
                if primary_declaration.local_id.ty != NodeType::Declaration {
                    return Vec::new();
                }

                let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                let mut expressions = Vec::new();
                match view.tree.get(declaration_id) {
                    Declaration::Class(declaration) => {
                        for expression_id in &declaration.implements_types {
                            expressions.push((*expression_id).into_global(view.module.id));
                        }
                    }
                    Declaration::Struct(declaration) => {
                        for expression_id in &declaration.implements_types {
                            expressions.push((*expression_id).into_global(view.module.id));
                        }
                    }
                    Declaration::Enum(declaration) => {
                        for expression_id in &declaration.implements_types {
                            expressions.push((*expression_id).into_global(view.module.id));
                        }
                    }
                    Declaration::Interface(declaration) => {
                        for expression_id in &declaration.extends_types {
                            expressions.push((*expression_id).into_global(view.module.id));
                        }
                    }
                    _ => {}
                }

                expressions
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Build interface substitutions for one interface owner on a receiver type.
    pub(crate) fn interface_substitutions_for_owner_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        interface_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        if interface_symbol.ty() != SymbolType::Interface {
            return Ok(None);
        }

        // normalize the receiver symbol for extension matching
        let canonical_receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // resolve substitutions from direct declaration implements clauses first
        if let Some(substitutions) = self.receiver_interface_substitutions(
            &mut ctx.reborrow(),
            source_id,
            canonical_receiver_symbol,
            receiver_arguments,
            interface_symbol,
        )? {
            return Ok(Some(substitutions));
        }

        // collect visible extensions for the receiver target
        let mut extension_symbols = self.visible_extension_symbols_for_target(
            ctx.symbol_type_view(),
            canonical_receiver_symbol,
        )?;

        // include directly declared extensions from the receiver module
        let declared_extension_symbols = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                canonical_receiver_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| {
                    let mut declared = Vec::new();
                    for declaration_id in view.tree.iter_node_ids_of_type::<Declaration>() {
                        let Declaration::Extension(declaration) = view.tree.get(declaration_id)
                        else {
                            continue;
                        };
                        let Some(extension_target) = declaration.target_symbol else {
                            continue;
                        };
                        let view = view.module_symbol_view();
                        let canonical_target = self.canonical_symbol_id(
                            view,
                            extension_target,
                            CanonicalSymbolMode::FollowAliases,
                        );
                        if canonical_target != canonical_receiver_symbol {
                            continue;
                        }

                        declared.push(declaration.symbol.into_global(view.module.id));
                    }

                    declared
                },
            )
            .map_err(AnalyzeError::from)?;

        // merge extension candidates without duplicates
        let mut seen_extensions = HashSet::new();
        extension_symbols.retain(|symbol| seen_extensions.insert(*symbol));
        for symbol in declared_extension_symbols {
            if seen_extensions.insert(symbol) {
                extension_symbols.push(symbol);
            }
        }
        if extension_symbols.is_empty() {
            return Ok(None);
        }

        // find an extension that implements the owning interface
        for extension_symbol in extension_symbols {
            let substitutions = self
                .with_module_tree_symbol_view_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    extension_symbol.module_id,
                    ctx.tree,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |view| -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
                        let symbol_entry = view.symbols.get_symbol(extension_symbol.local_id);
                        let Some(primary_declaration) = symbol_entry.primary_declaration else {
                            return Ok(None);
                        };
                        if primary_declaration.local_id.ty != NodeType::Declaration {
                            return Ok(None);
                        }
                        let declaration_id =
                            primary_declaration.local_id.into_typed::<Declaration>();
                        let Declaration::Extension(declaration) = view.tree.get(declaration_id)
                        else {
                            return Ok(None);
                        };
                        if declaration.implements_types.is_empty() {
                            return Ok(None);
                        }

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
                        self.interface_substitutions_for_owner_implements_types(
                            &mut ctx,
                            source_id,
                            extension_symbol,
                            receiver_arguments,
                            interface_symbol,
                            &declaration.implements_types,
                        )
                    },
                )
                .map_err(AnalyzeError::from)??;
            if substitutions.is_some() {
                return Ok(substitutions);
            }
        }

        Ok(None)
    }

    /// Resolve interface substitutions from receiver declaration heritage.
    fn receiver_interface_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        interface_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            receiver_symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                // resolve receiver declaration and heritage
                let symbol_entry = view.symbols.get_symbol(receiver_symbol.local_id);
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return Ok(None);
                };
                if primary_declaration.local_id.ty != NodeType::Declaration {
                    return Ok(None);
                }
                let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                let implements_types = match view.tree.get(declaration_id) {
                    Declaration::Class(declaration) => {
                        Some(declaration.implements_types.as_slice())
                    }
                    Declaration::Struct(declaration) => {
                        Some(declaration.implements_types.as_slice())
                    }
                    Declaration::Enum(declaration) => Some(declaration.implements_types.as_slice()),
                    _ => None,
                };
                let Some(implements_types) = implements_types else {
                    return Ok(None);
                };

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
                self.interface_substitutions_for_owner_implements_types(
                    &mut ctx,
                    source_id,
                    receiver_symbol,
                    receiver_arguments,
                    interface_symbol,
                    implements_types,
                )
            },
        )
        .map_err(AnalyzeError::from)?
    }

    /// Resolve interface substitutions for one owner and one implements list.
    fn interface_substitutions_for_owner_implements_types(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        interface_symbol: GlobalSymbolId,
        implements_types: &[LocalNodeId<TypeExpression>],
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        let receiver_substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            receiver_symbol,
            source_id,
            receiver_arguments,
        );

        self.interface_substitutions_for_implements_types(
            &mut ctx.reborrow(),
            source_id,
            interface_symbol,
            implements_types,
            &receiver_substitutions,
        )
    }

    /// Resolve interface substitutions from a list of implements expressions.
    fn interface_substitutions_for_implements_types(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        interface_symbol: GlobalSymbolId,
        implements_types: &[LocalNodeId<TypeExpression>],
        receiver_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        let interface_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            interface_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let interface_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), interface_symbol)
            .unwrap_or(interface_symbol);

        for interface_expression_id in implements_types {
            let Some(target_symbol) = ctx.tree.get(*interface_expression_id).target_symbol() else {
                continue;
            };
            let mut canonical_target = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            canonical_target = self
                .declaration_symbol_id(ctx.module_symbol_view(), canonical_target)
                .unwrap_or(canonical_target);

            // resolve interface arguments from heritage expressions
            let generic_argument_nodes = ctx.tree.get(*interface_expression_id).generic_arguments();
            let evaluated_static_arguments =
                self.evaluate_generic_arguments(&mut ctx.reborrow(), generic_argument_nodes)?;
            let evaluated_static_arguments = evaluated_static_arguments.unwrap_or_default();
            let resolved_static_arguments = self.resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                canonical_target,
                Some(evaluated_static_arguments.as_slice()),
                true,
            )?;
            let interface_arguments =
                resolved_static_arguments.unwrap_or(evaluated_static_arguments);
            let mut substitutions = self.build_type_parameter_substitutions_for_symbol(
                &mut ctx.reborrow(),
                canonical_target,
                source_id,
                &interface_arguments,
            );
            // apply receiver substitutions to inherited interface arguments
            if !receiver_substitutions.is_empty() {
                let mut cache = HashMap::new();
                for ty_id in substitutions.values_mut() {
                    let mapped = self.substitute_static_parameters(
                        *ty_id,
                        receiver_substitutions,
                        ctx.types,
                        &mut cache,
                    );
                    *ty_id = mapped;
                }
            }

            // direct match on the requested interface
            if canonical_target == interface_symbol {
                return Ok(Some(substitutions));
            }

            // inherited interface match through extends chains
            if canonical_target.ty() == SymbolType::Interface {
                let mut visited_symbols = HashSet::new();
                if let Some(inherited_substitutions) = self
                    .associated_projection_receiver_substitutions_inner(
                        &mut ctx.reborrow(),
                        source_id,
                        canonical_target,
                        interface_symbol,
                        substitutions,
                        &mut visited_symbols,
                    )?
                {
                    return Ok(Some(inherited_substitutions));
                }
            }
        }

        Ok(None)
    }

    /// Build one canonical projection environment for a projected member.
    pub(crate) fn projection_environment_for_member(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        explicit_member_arguments: Option<&[StaticArgument]>,
        member_ty: Option<&Type>,
        static_eval_visited_symbols: Option<&HashSet<GlobalSymbolId>>,
    ) -> AnalyzeResult<ProjectionEnvironment> {
        let mut substitutions = HashMap::new();
        let owner_symbol =
            self.query_owner_symbol_for_member_symbol(ctx.module_symbol_view(), target_symbol)?;
        let normalized_receiver = receiver_symbol
            .map(|receiver_symbol| {
                self.normalize_projection_receiver_reference(
                    &mut ctx.reborrow(),
                    source_id,
                    receiver_symbol,
                    receiver_arguments,
                )
            })
            .transpose()?;
        let owner_symbol = owner_symbol.map(|owner_symbol| {
            self.declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
                .unwrap_or(owner_symbol)
        });

        // bind inherited receiver arguments onto owner parameters first
        if owner_symbol.is_some()
            && let Some((_, receiver_arguments)) = normalized_receiver.as_ref()
        {
            let receiver_arguments = self.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                target_symbol,
                source_id,
                receiver_arguments,
            );
            let mut owner_substitutions = HashMap::new();
            self.extend_owner_substitutions_from_inherited(
                &mut ctx.reborrow(),
                source_id,
                target_symbol,
                &receiver_arguments,
                &mut owner_substitutions,
            );
            substitutions.extend(owner_substitutions);
        }

        // map receiver substitutions onto owner parameters
        let mut receiver_owner_substitutions = None;
        if let Some(owner_symbol) = owner_symbol
            && let Some((receiver_symbol, receiver_arguments)) = normalized_receiver.as_ref()
        {
            let receiver_arguments = self.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                *receiver_symbol,
                source_id,
                receiver_arguments,
            );
            if owner_symbol.ty() == SymbolType::Interface {
                receiver_owner_substitutions = self.interface_substitutions_for_owner_symbol(
                    &mut ctx.reborrow(),
                    source_id,
                    *receiver_symbol,
                    &receiver_arguments,
                    owner_symbol,
                )?;

                // fall back to general receiver traversal for constrained/interface projections
                if receiver_owner_substitutions.is_none() {
                    receiver_owner_substitutions = self
                        .receiver_projection_substitutions_for_owner(
                            &mut ctx.reborrow(),
                            source_id,
                            *receiver_symbol,
                            &receiver_arguments,
                            owner_symbol,
                        )?;
                }
            } else {
                receiver_owner_substitutions = self.receiver_projection_substitutions_for_owner(
                    &mut ctx.reborrow(),
                    source_id,
                    *receiver_symbol,
                    &receiver_arguments,
                    owner_symbol,
                )?;
            }
        }
        if let Some(receiver_owner_substitutions) = receiver_owner_substitutions.as_ref() {
            substitutions.extend(
                receiver_owner_substitutions
                    .iter()
                    .map(|(symbol, ty_id)| (*symbol, *ty_id)),
            );
        }

        // map owner associated comptime members onto receiver concrete values
        if let Some((receiver_symbol, _)) = normalized_receiver
            && let Some(owner_symbol) = owner_symbol
            && let Some(owner_comptime_substitutions) = self
                .owner_comptime_substitutions_for_receiver(
                    &mut ctx.reborrow(),
                    source_id,
                    target_symbol,
                    receiver_symbol,
                    owner_symbol,
                    static_eval_visited_symbols,
                    receiver_owner_substitutions.as_ref(),
                )?
        {
            substitutions.extend(owner_comptime_substitutions);
        }

        // map extension substitutions when the member is extension-owned
        if let Ok(Some(extension_context)) = self.resolve_extension_member_context(
            &mut ctx.reborrow(),
            source_id,
            target_symbol,
            receiver_arguments,
        ) {
            substitutions.extend(extension_context.substitutions);
        }

        // resolve member static arguments under the projection context
        let resolved_member_arguments = self.resolve_type_reference_static_arguments_with_bounds(
            &mut ctx.reborrow(),
            source_id,
            target_symbol,
            explicit_member_arguments,
            true,
            Some(&substitutions),
        )?;
        if let Some((member_argument_symbol, member_arguments)) = self
            .projection_member_argument_source_for_environment(
                target_symbol,
                explicit_member_arguments,
                resolved_member_arguments.as_deref(),
                member_ty,
            )
        {
            let member_substitutions = self.build_type_parameter_substitutions_for_symbol(
                &mut ctx.reborrow(),
                member_argument_symbol,
                source_id,
                member_arguments,
            );
            substitutions.extend(member_substitutions);
        }

        let (receiver_symbol, receiver_arguments) = normalized_receiver
            .map(|(receiver_symbol, receiver_arguments)| {
                (Some(receiver_symbol), receiver_arguments)
            })
            .unwrap_or((None, Vec::new()));

        Ok(ProjectionEnvironment {
            owner_symbol,
            receiver_symbol,
            receiver_arguments,
            substitutions,
        })
    }

    /// Build owner associated comptime substitutions for one projected receiver.
    pub(super) fn owner_comptime_substitutions_for_receiver(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        receiver_symbol: GlobalSymbolId,
        owner_symbol: GlobalSymbolId,
        static_eval_visited_symbols: Option<&HashSet<GlobalSymbolId>>,
        receiver_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        if !matches!(
            owner_symbol.ty(),
            SymbolType::Interface | SymbolType::Class | SymbolType::Struct
        ) {
            return Ok(None);
        }
        let owner_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
            .unwrap_or(owner_symbol);

        // normalize the receiver symbol for lookup
        let canonical_receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let canonical_receiver_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), canonical_receiver_symbol)
            .unwrap_or(canonical_receiver_symbol);

        // avoid recursive default evaluation when projecting on interface owners
        if owner_symbol.ty() == SymbolType::Interface && canonical_receiver_symbol == owner_symbol {
            return Ok(None);
        }
        let Some(receiver_substitutions) = receiver_substitutions else {
            return Ok(None);
        };

        // collect owner associated comptime member symbols by name
        let owner_members = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                owner_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| {
                    let mut members = Vec::new();
                    let symbol_entry = view.symbols.get_symbol(owner_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return members;
                    };
                    if primary_declaration.local_id.ty != NodeType::Declaration {
                        return members;
                    }

                    let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                    let declaration = view.tree.get(declaration_id);
                    let Some(member_ids) = declaration.member_ids() else {
                        return members;
                    };

                    for member_id in member_ids {
                        let Member::AssociatedConst {
                            name,
                            symbol,
                            value,
                            ..
                        } = view.tree.get(*member_id)
                        else {
                            continue;
                        };
                        members.push((
                            StaticKey::Name(*name),
                            symbol.into_global(view.module.id),
                            value.is_none(),
                        ));
                    }

                    members
                },
            )
            .map_err(AnalyzeError::from)?;
        if owner_members.is_empty() {
            return Ok(None);
        }

        // resolve concrete receiver values for owner associated comptime members
        let mut substitutions = HashMap::new();
        for (member_key, owner_member_symbol, requires_implementation) in owner_members {
            // skip self substitution for the projected member:
            // this avoids recursive evaluation when materializing that same associated comptime member
            if owner_member_symbol == target_symbol {
                continue;
            }
            if static_eval_visited_symbols
                .is_some_and(|visited_symbols| visited_symbols.contains(&owner_member_symbol))
            {
                continue;
            }

            let resolved_member_symbol = self
                .with_module_tree_symbol_view_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    canonical_receiver_symbol.module_id,
                    ctx.tree,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_interface,
                    |view| {
                        self.query_static_member_symbol(
                            ctx.compiler_context.revision(),
                            view.module,
                            ctx.profile,
                            canonical_receiver_symbol,
                            member_key,
                            view.tree,
                            view.symbols,
                        )
                    },
                )
                .map_err(AnalyzeError::from)?
                .unwrap_or(owner_member_symbol);
            if static_eval_visited_symbols
                .is_some_and(|visited_symbols| visited_symbols.contains(&resolved_member_symbol))
            {
                continue;
            }

            let should_skip_interface_owner_default = owner_symbol.ty() == SymbolType::Interface
                && resolved_member_symbol == owner_member_symbol
                && requires_implementation;
            if should_skip_interface_owner_default {
                let should_report_missing =
                    requires_implementation && canonical_receiver_symbol != owner_symbol;
                if should_report_missing {
                    ctx.types
                        .mark_symbol_with_unimplemented_associated_requirements(
                            canonical_receiver_symbol,
                        );
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: source_id
                            .into_global(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        message: "missing associated comptime implementation".to_string(),
                    });
                }
                continue;
            }

            let mut visited_symbols = static_eval_visited_symbols.cloned().unwrap_or_default();
            let Some(value) = self.resolve_static_constant_reference_for_mode(
                &mut ctx.reborrow(),
                resolved_member_symbol,
                Some(receiver_substitutions),
                &mut visited_symbols,
                StaticConstantResolutionMode::InstantiatedDeclare,
            )?
            else {
                let should_report_missing =
                    requires_implementation && canonical_receiver_symbol != owner_symbol;
                if should_report_missing {
                    ctx.types
                        .mark_symbol_with_unimplemented_associated_requirements(
                            canonical_receiver_symbol,
                        );
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: source_id
                            .into_global(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        message: "missing associated comptime implementation".to_string(),
                    });
                }
                continue;
            };

            let Some(type_id) =
                self.static_expression_type_id_for_substitution(source_id, &value, ctx.types)
            else {
                continue;
            };

            // normalize inherited owner comptime values before publishing them as substitutions
            let mut type_id = type_id;
            if !receiver_substitutions.is_empty() {
                let mut substitution_cache = HashMap::new();
                type_id = self.substitute_static_parameters(
                    type_id,
                    receiver_substitutions,
                    ctx.types,
                    &mut substitution_cache,
                );
            }

            let mut materialize_cache = TypeRewriteCache::new();
            type_id = self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                type_id,
                &mut materialize_cache,
            );
            type_id = self.normalize_type_with_relation(
                &mut ctx.reborrow(),
                type_id,
                NormalizationMode::Assign,
                RelationMode::STATIC_EVAL,
            );
            type_id = self.normalized_projection_substitution_type(type_id, ctx.types);

            substitutions.insert(owner_member_symbol, type_id);
        }

        if substitutions.is_empty() {
            return Ok(None);
        }

        Ok(Some(substitutions))
    }

    /// Convert a static expression to a local type id for substitution.
    pub(crate) fn static_expression_type_id_for_substitution(
        &self,
        source_id: LocalNodeIdAny,
        value: &StaticExpression,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        match value {
            StaticExpression::ScalarLiteral { value } => Some(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value.clone()),
                },
                source_id,
            )),
            StaticExpression::TypeLiteral { value } => Some(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                source_id,
            )),
            StaticExpression::Type { ty } => Some(*ty),
            _ => None,
        }
    }

    /// Find one projection substitution for a symbol, tolerating placeholder symbol types.
    fn projection_substitution_type_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<LocalTypeId> {
        if let Some(substitution) = substitutions.get(&symbol) {
            return Some(*substitution);
        }

        substitutions.iter().find_map(|(candidate, type_id)| {
            let matches_symbol = candidate.module_id == symbol.module_id
                && candidate.local_id.id == symbol.local_id.id;
            if matches_symbol { Some(*type_id) } else { None }
        })
    }

    /// Convert one projection substitution type into a static expression.
    fn static_expression_from_projection_substitution_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> StaticExpression {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Type::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            _ => StaticExpression::Type { ty: type_id },
        }
    }

    /// Normalize one projection substitution type for static argument replacement.
    fn normalized_projection_substitution_type(
        &self,
        mut type_id: LocalTypeId,
        types: &TypeTable,
    ) -> LocalTypeId {
        // unwrap value wrappers so static arguments receive the underlying type
        while let Type::Value { value } = types.get_type(type_id) {
            type_id = *value;
        }

        type_id
    }

    /// Return the projected substitution symbol referenced by one type expression.
    fn projection_substitution_symbol_from_type_expression(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<GlobalSymbolId> {
        let mut expression_id = expression_id;

        // peel wrappers used around projection arguments
        loop {
            match ctx.tree.get(expression_id) {
                TypeExpression::Parenthesized { expression } => {
                    expression_id = *expression;
                }
                TypeExpression::AsComptime { target_type: right } => {
                    expression_id = *right;
                }
                _ => break,
            }
        }

        if let Some(target_symbol) = ctx.tree.get(expression_id).target_symbol() {
            return Some(target_symbol);
        }

        if let TypeExpression::Member { left, name, .. } = ctx.tree.get(expression_id)
            && matches!(ctx.tree.get(*left), TypeExpression::This)
            && let Some((owner_symbol, _)) = self.owner_symbol_for_this_type_expression(ctx, *left)
            && let Some(member_symbol) = self.query_static_member_symbol(
                ctx.compiler_context.revision(),
                ctx.module,
                ctx.profile,
                owner_symbol,
                StaticKey::Name(*name),
                ctx.tree,
                ctx.symbols,
            )
        {
            return Some(member_symbol);
        }

        None
    }

    /// Apply associated projection substitutions by alias type-expression shape.
    pub(crate) fn apply_projection_substitutions_from_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve local declared types before rewriting structured type shapes
        let needs_declared_shape = matches!(
            ctx.tree.get(expression_id),
            TypeExpression::Mapped { .. }
                | TypeExpression::Tuple { .. }
                | TypeExpression::Index { .. }
                | TypeExpression::Parenthesized { .. }
                | TypeExpression::AsComptime { .. }
        );
        if needs_declared_shape && matches!(ctx.types.get_type(local_type_id), Type::Unevaluated(_))
        {
            self.resolve_declared_type(&mut ctx.reborrow(), local_type_id)?;
        }

        // projection alias roots
        if let Some(mapped_alias_target) = self.projection_substituted_alias_target_for_expression(
            &mut ctx.reborrow(),
            expression_id,
            substitutions,
        )? {
            return Ok(mapped_alias_target);
        }

        // unevaluated projection references
        if let Some(mapped_type) = self.substitute_projection_unevaluated_type(
            &mut ctx.reborrow(),
            expression_id,
            local_type_id,
            substitutions,
        )? {
            return Ok(mapped_type);
        }

        // recurse through nested index expressions and set count inferred types from substitutions
        match ctx.tree.get(expression_id).clone() {
            TypeExpression::Mapped {
                parameter, value, ..
            } => {
                return self.apply_projection_substitutions_to_mapped_type(
                    &mut ctx.reborrow(),
                    parameter.source_type,
                    parameter.key_remap,
                    value,
                    local_type_id,
                    substitutions,
                );
            }
            TypeExpression::Tuple { elements } => {
                return self.apply_projection_substitutions_to_tuple_type(
                    &mut ctx.reborrow(),
                    &elements,
                    local_type_id,
                    substitutions,
                );
            }
            TypeExpression::Index { left, index } => {
                return self.apply_projection_substitutions_to_type_index(
                    &mut ctx.reborrow(),
                    expression_id,
                    left,
                    index,
                    local_type_id,
                    substitutions,
                );
            }
            TypeExpression::Parenthesized { expression } => {
                return self.apply_projection_substitutions_from_type_expression(
                    &mut ctx.reborrow(),
                    expression,
                    local_type_id,
                    substitutions,
                );
            }
            TypeExpression::AsComptime { target_type: right } => {
                return self.apply_projection_substitutions_from_type_expression(
                    &mut ctx.reborrow(),
                    right,
                    local_type_id,
                    substitutions,
                );
            }
            _ => {}
        }

        // map direct reference substitutions for projected symbols
        let (symbol, static_arguments) = match ctx.types.get_type(local_type_id).clone() {
            Type::Reference {
                symbol,
                static_arguments,
            } => (symbol, static_arguments),
            _ => return Ok(local_type_id),
        };
        if static_arguments.is_none() {
            return self.substitute_projection_reference_without_arguments(
                &mut ctx.reborrow(),
                expression_id,
                symbol,
                local_type_id,
                substitutions,
            );
        }

        // map reference static arguments that originate from owner projections
        self.substitute_projection_reference_with_arguments(
            &mut ctx.reborrow(),
            expression_id,
            symbol,
            static_arguments,
            local_type_id,
            substitutions,
        )
    }

    /// Return one projection-substituted type for a tuple type expression.
    fn apply_projection_substitutions_to_tuple_type(
        &self,
        ctx: &mut TypeContext<'_>,
        element_arguments: &[LocalNodeId<destack_dir::TupleElement>],
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        let Type::Tuple {
            elements,
            is_readonly,
        } = ctx.types.get_type(local_type_id).clone()
        else {
            return Ok(local_type_id);
        };
        if elements.len() != element_arguments.len() {
            return Ok(local_type_id);
        }

        let mut mapped_elements = Vec::with_capacity(elements.len());
        let mut changed = false;
        for (argument_id, mut element) in element_arguments.iter().zip(elements.into_iter()) {
            let Some(element_expression_id) = ctx.tree.get::<TupleElement>(*argument_id).value()
            else {
                continue;
            };
            let mapped_element_ty = self.apply_projection_substitutions_from_type_expression(
                &mut ctx.reborrow(),
                element_expression_id,
                element.ty,
                substitutions,
            )?;
            if mapped_element_ty != element.ty {
                element.ty = mapped_element_ty;
                changed = true;
            }
            mapped_elements.push(element);
        }
        if !changed {
            return Ok(local_type_id);
        }

        Ok(ctx.types.insert_type_from_type(
            Type::Tuple {
                elements: mapped_elements,
                is_readonly,
            },
            local_type_id,
        ))
    }

    /// Return one projection-substituted type for a mapped type expression.
    fn apply_projection_substitutions_to_mapped_type(
        &self,
        ctx: &mut TypeContext<'_>,
        parameter_constraint_expression: LocalNodeId<TypeExpression>,
        parameter_key_remap_expression: Option<LocalNodeId<TypeExpression>>,
        value_expression: LocalNodeId<TypeExpression>,
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        match ctx.types.get_type(local_type_id).clone() {
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.apply_projection_substitutions_from_type_expression(
                    &mut ctx.reborrow(),
                    parameter_constraint_expression,
                    parameter.constraint,
                    substitutions,
                )?;
                let mapped_key_remap = match (parameter.key_remap, parameter_key_remap_expression) {
                    (Some(key_remap), Some(key_remap_expression)) => {
                        Some(self.apply_projection_substitutions_from_type_expression(
                            &mut ctx.reborrow(),
                            key_remap_expression,
                            key_remap,
                            substitutions,
                        )?)
                    }
                    (key_remap, _) => key_remap,
                };
                let mapped_value = self.apply_projection_substitutions_from_type_expression(
                    &mut ctx.reborrow(),
                    value_expression,
                    value,
                    substitutions,
                )?;
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    return Ok(local_type_id);
                }

                let parameter = MappedTypeParameter {
                    name: parameter.name,
                    symbol: parameter.symbol.into(),
                    constraint: mapped_constraint,
                    key_remap: mapped_key_remap,
                };
                Ok(ctx.types.insert_type_from_type(
                    Type::Mapped {
                        parameter,
                        modifiers,
                        value: mapped_value,
                    },
                    local_type_id,
                ))
            }

            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut mapped_fields = Vec::with_capacity(fields.len());
                let mut changed = false;
                for mut field in fields {
                    let mapped_field_ty = self
                        .apply_projection_substitutions_from_type_expression(
                            &mut ctx.reborrow(),
                            value_expression,
                            field.ty,
                            substitutions,
                        )?;
                    if mapped_field_ty != field.ty {
                        field.ty = mapped_field_ty;
                        changed = true;
                    }
                    mapped_fields.push(field);
                }
                if !changed {
                    return Ok(local_type_id);
                }

                Ok(ctx.types.insert_type_from_type(
                    Type::Object {
                        fields: mapped_fields,
                        call_signatures,
                        construct_signatures,
                        index_signatures,
                    },
                    local_type_id,
                ))
            }

            _ => Ok(local_type_id),
        }
    }

    /// Return one projection-substituted alias target for a projection-root type expression.
    fn projection_substituted_alias_target_for_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(target_symbol) = self.projection_substitution_symbol_from_type_expression(
            ctx.tree_symbol_view(),
            expression_id,
        ) else {
            return Ok(None);
        };
        let Some(alias_target_id) = self.require_alias_target_type_id_for_symbol(
            &mut ctx.reborrow(),
            target_symbol,
            expression_id.into_any(),
        )?
        else {
            return Ok(None);
        };

        let mapped_alias_target = self.apply_associated_projection_substitutions(
            &mut ctx.reborrow(),
            target_symbol,
            alias_target_id,
            substitutions,
        )?;
        if mapped_alias_target == alias_target_id {
            return Ok(None);
        }

        Ok(Some(mapped_alias_target))
    }

    /// Resolve one alias type-expression node for projection substitution by declaration kind.
    fn projection_alias_expression_for_symbol(
        &self,
        view: TreeSymbolView<'_>,
        target_symbol: GlobalSymbolId,
    ) -> Option<LocalNodeId<TypeExpression>> {
        let symbol_entry = view.symbols.get_symbol(target_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::Member {
            return None;
        }

        let member_id = primary_declaration.local_id.into_typed::<Member>();
        match view.tree.get(member_id) {
            Member::AssociatedType {
                value: Some(alias_expression),
                ..
            } => Some(*alias_expression),
            _ => None,
        }
    }

    /// Return one projection-substituted type for an unevaluated reference expression.
    fn substitute_projection_unevaluated_type(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        if !matches!(ctx.types.get_type(local_type_id), Type::Unevaluated(_)) {
            return Ok(None);
        }

        let Some(target_symbol) = self.projection_substitution_symbol_from_type_expression(
            ctx.tree_symbol_view(),
            expression_id,
        ) else {
            return Ok(None);
        };

        if let Some(substitution) =
            self.projection_substitution_type_for_symbol(target_symbol, substitutions)
        {
            let mapped_type = self.normalized_projection_substitution_type(substitution, ctx.types);
            return Ok(Some(mapped_type));
        }

        let mut visited_symbols = HashSet::new();
        let Ok(Some(value)) = self.resolve_static_constant_reference_for_mode(
            &mut ctx.reborrow(),
            target_symbol,
            Some(substitutions),
            &mut visited_symbols,
            StaticConstantResolutionMode::InstantiatedInfer,
        ) else {
            return Ok(None);
        };
        let Some(value_type_id) = self.static_expression_type_id_for_substitution(
            expression_id.into_any(),
            &value,
            ctx.types,
        ) else {
            return Ok(None);
        };

        let mut mapped_value_type_id = value_type_id;
        if !substitutions.is_empty() {
            let mut substitution_cache = HashMap::new();
            mapped_value_type_id = self.substitute_static_parameters(
                mapped_value_type_id,
                substitutions,
                ctx.types,
                &mut substitution_cache,
            );
        }

        let mut materialize_cache = TypeRewriteCache::new();
        mapped_value_type_id = self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            mapped_value_type_id,
            &mut materialize_cache,
        );
        mapped_value_type_id = self.normalize_type_with_relation(
            &mut ctx.reborrow(),
            mapped_value_type_id,
            NormalizationMode::Assign,
            RelationMode::STATIC_EVAL,
        );

        Ok(Some(mapped_value_type_id))
    }

    /// Return one projection-substituted type for a type-index expression.
    fn apply_projection_substitutions_to_type_index(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        left: LocalNodeId<TypeExpression>,
        index: LocalNodeId<TypeExpression>,
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        let type_index_parts = match ctx.types.get_type(local_type_id) {
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => Some((*element, *count, Some(*is_readonly))),
            Type::Index {
                left: element,
                index: count,
            } => Some((*element, *count, None)),
            _ => None,
        };
        let Some((element, count, is_readonly)) = type_index_parts else {
            return Ok(local_type_id);
        };

        let mut substitution_cache = HashMap::new();
        let mut mapped_count = self.substitute_static_parameters(
            count,
            substitutions,
            ctx.types,
            &mut substitution_cache,
        );

        if let Some(target_symbol) =
            self.projection_substitution_symbol_from_type_expression(ctx.tree_symbol_view(), index)
            && let Some(substitution) =
                self.projection_substitution_type_for_symbol(target_symbol, substitutions)
        {
            mapped_count = self.normalized_projection_substitution_type(substitution, ctx.types);
        }

        // map direct count references through projection substitutions
        if let Type::Reference { symbol, .. } = ctx.types.get_type(mapped_count).clone()
            && let Some(substitution) =
                self.projection_substitution_type_for_symbol(symbol, substitutions)
        {
            mapped_count = self.normalized_projection_substitution_type(substitution, ctx.types);
        }

        // materialize remaining comptime references using projection substitutions
        if let Type::Reference { symbol, .. } = ctx.types.get_type(mapped_count).clone() {
            let mut visited_symbols = HashSet::new();
            if let Ok(Some(value)) = self.resolve_static_constant_reference_for_mode(
                &mut ctx.reborrow(),
                symbol,
                Some(substitutions),
                &mut visited_symbols,
                StaticConstantResolutionMode::InstantiatedInfer,
            ) && let Some(value_type_id) = self.static_expression_type_id_for_substitution(
                expression_id.into_any(),
                &value,
                ctx.types,
            ) {
                mapped_count = value_type_id;
                if !substitutions.is_empty() {
                    let mut substitution_cache = HashMap::new();
                    mapped_count = self.substitute_static_parameters(
                        mapped_count,
                        substitutions,
                        ctx.types,
                        &mut substitution_cache,
                    );
                }

                let mut materialize_cache = TypeRewriteCache::new();
                mapped_count = self.materialize_static_arguments_in_type(
                    &mut ctx.reborrow(),
                    mapped_count,
                    &mut materialize_cache,
                );
                mapped_count =
                    self.normalized_projection_substitution_type(mapped_count, ctx.types);
            }
        }

        let mapped_element = self.apply_projection_substitutions_from_type_expression(
            &mut ctx.reborrow(),
            left,
            element,
            substitutions,
        )?;

        if mapped_element == element && mapped_count == count {
            return Ok(local_type_id);
        }

        let uses_index_access =
            self.type_index_uses_index_access(&mut ctx.reborrow(), mapped_element, index)?;
        if uses_index_access {
            return Ok(ctx.types.insert_type_from_type(
                Type::Index {
                    left: mapped_element,
                    index: mapped_count,
                },
                local_type_id,
            ));
        }

        Ok(ctx.types.insert_type_from_type(
            Type::ArraySized {
                element: mapped_element,
                count: mapped_count,
                is_readonly: is_readonly.unwrap_or(false),
            },
            local_type_id,
        ))
    }

    /// Return one projection-substituted reference type with no static arguments.
    fn substitute_projection_reference_without_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        symbol: GlobalSymbolId,
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(mapped_symbol) = self.projection_substitution_symbol_from_type_expression(
            ctx.tree_symbol_view(),
            expression_id,
        ) && (mapped_symbol == symbol
            || (mapped_symbol.module_id == symbol.module_id
                && mapped_symbol.local_id.id == symbol.local_id.id))
            && let Some(substitution) =
                self.projection_substitution_type_for_symbol(mapped_symbol, substitutions)
        {
            return Ok(self.normalized_projection_substitution_type(substitution, ctx.types));
        }

        if let Some(substitution) =
            self.projection_substitution_type_for_symbol(symbol, substitutions)
        {
            return Ok(self.normalized_projection_substitution_type(substitution, ctx.types));
        }

        let mut visited_symbols = HashSet::new();
        if let Ok(Some(value)) = self.resolve_static_constant_reference_for_mode(
            &mut ctx.reborrow(),
            symbol,
            Some(substitutions),
            &mut visited_symbols,
            StaticConstantResolutionMode::InstantiatedInfer,
        ) && let Some(value_type_id) = self.static_expression_type_id_for_substitution(
            expression_id.into_any(),
            &value,
            ctx.types,
        ) {
            return Ok(value_type_id);
        }

        Ok(local_type_id)
    }

    /// Return one projection-substituted reference type with static arguments.
    fn substitute_projection_reference_with_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        symbol: GlobalSymbolId,
        static_arguments: Option<Vec<StaticArgument>>,
        local_type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        let Some(static_arguments) = static_arguments else {
            return Ok(local_type_id);
        };
        let Some(argument_nodes) = ctx.tree.get(expression_id).generic_arguments() else {
            return Ok(local_type_id);
        };

        // walk static arguments in lock-step with the alias expression arguments
        let mut changed = false;
        let mut mapped_arguments = static_arguments;
        for (argument_index, argument_node) in argument_nodes.iter().enumerate() {
            let Some(current_argument) = mapped_arguments.get(argument_index).cloned() else {
                continue;
            };
            let argument_type_expression_id = match ctx.tree.get(*argument_node) {
                destack_dir::GenericArgument::Type { value } => *value,
                destack_dir::GenericArgument::Value { .. } => continue,
                destack_dir::GenericArgument::Error => continue,
            };

            // replace direct symbol references with projection substitutions
            let target_symbol = self.projection_substitution_symbol_from_type_expression(
                ctx.tree_symbol_view(),
                argument_type_expression_id,
            );
            if let Some(target_symbol) = target_symbol
                && let Some(substitution) =
                    self.projection_substitution_type_for_symbol(target_symbol, substitutions)
            {
                let substitution =
                    self.normalized_projection_substitution_type(substitution, ctx.types);
                let substitution_value = self
                    .static_expression_from_projection_substitution_type(substitution, ctx.types);
                let argument_name = match current_argument {
                    StaticArgument::Evaluated { name, .. } => name,
                    StaticArgument::Unevaluated { .. } => None,
                };
                let replacement = StaticArgument::Evaluated {
                    name: argument_name,
                    value: substitution_value,
                };
                if replacement != current_argument {
                    mapped_arguments[argument_index] = replacement;
                    changed = true;
                }
                continue;
            }

            // recurse into nested type arguments to keep fixed-array substitutions aligned
            let StaticArgument::Evaluated { name, value } = current_argument else {
                continue;
            };
            let StaticExpression::Type { ty } = value else {
                continue;
            };
            let mapped_type = self.apply_projection_substitutions_from_type_expression(
                &mut ctx.reborrow(),
                argument_type_expression_id,
                ty,
                substitutions,
            )?;
            if mapped_type == ty {
                continue;
            }

            mapped_arguments[argument_index] = StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty: mapped_type },
            };
            changed = true;
        }

        let mapped_reference_type_id = if changed {
            ctx.types.insert_type_from_type(
                Type::Reference {
                    symbol,
                    static_arguments: Some(mapped_arguments.clone()),
                },
                local_type_id,
            )
        } else {
            local_type_id
        };

        Ok(mapped_reference_type_id)
    }

    /// Apply associated projection substitutions to imported alias targets when needed.
    pub(crate) fn apply_associated_projection_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        target_symbol: GlobalSymbolId,
        alias_target_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<LocalTypeId> {
        if substitutions.is_empty() {
            return Ok(alias_target_id);
        }

        let mapped_alias_target = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                target_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| -> AnalyzeResult<LocalTypeId> {
                    let owner_view = TreeSymbolView::new(
                        view.compiler_context,
                        view.module,
                        ctx.profile,
                        view.tree,
                        view.symbols,
                    );
                    let expression_id =
                        self.projection_alias_expression_for_symbol(owner_view, target_symbol);
                    let Some(expression_id) = expression_id else {
                        return Ok(alias_target_id);
                    };
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
                    self.apply_projection_substitutions_from_type_expression(
                        &mut ctx,
                        expression_id,
                        alias_target_id,
                        substitutions,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;
        let mapped_alias_target = mapped_alias_target?;

        Ok(mapped_alias_target)
    }

    /// Select one deterministic argument source for projection member substitutions.
    fn projection_member_argument_source_for_environment<'a>(
        &self,
        target_symbol: GlobalSymbolId,
        explicit_static_arguments: Option<&'a [StaticArgument]>,
        resolved_static_arguments: Option<&'a [StaticArgument]>,
        member_ty: Option<&'a Type>,
    ) -> Option<(GlobalSymbolId, &'a [StaticArgument])> {
        // use resolved arguments from explicit projection expressions when available
        if let Some(arguments) = resolved_static_arguments
            && !arguments.is_empty()
        {
            return Some((target_symbol, arguments));
        }

        // otherwise use explicit arguments directly
        if let Some(arguments) = explicit_static_arguments
            && !arguments.is_empty()
        {
            return Some((target_symbol, arguments));
        }

        // otherwise use reference carried arguments from the projected member type
        if let Some(Type::Reference {
            symbol,
            static_arguments: Some(arguments),
        }) = member_ty
            && !arguments.is_empty()
        {
            return Some((*symbol, arguments));
        }

        None
    }
}
