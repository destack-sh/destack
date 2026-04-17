use std::collections::{HashMap, HashSet};

use crate::analyze::common::{
    AnalyzeIndex, CanonicalSymbolMode, ModuleSymbolView, REWRITER_TAG_ASSOCIATED_ALIAS,
    TreeSymbolView, TypeContext, TypeRewriteCache, TypeView, TypeWalkContext, TypeWalkKey,
    rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeResult, Compiler, CompilerContext, ResolveError, ResolveResult};
use destack_core::StringId;
use destack_dir::{
    Declaration, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Member, NodeTree, NodeType, StaticArgument, StaticExpression, StaticKey,
    SymbolTable, SymbolType, Type, TypeExpression, TypeRewriter, TypeRewriterOptions, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

/// Rewrite associated alias references inside projected member types.
pub(super) struct AssociatedAliasProjectionRewriter<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The pinned compiler context.
    compiler_context: &'a CompilerContext<'a>,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The source node for diagnostics.
    source_id: LocalNodeIdAny,
    /// The projection receiver symbol, when this rewrite came from a member projection.
    receiver_symbol: Option<GlobalSymbolId>,
    /// The projection receiver static arguments.
    receiver_arguments: &'a [StaticArgument],
    /// The substitutions currently applied to the projection.
    substitutions: &'a HashMap<GlobalSymbolId, LocalTypeId>,
    /// The tree used for static argument substitution.
    tree: &'a NodeTree,
    /// The symbols used for static argument substitution.
    symbols: &'a SymbolTable,
    /// The cache key for rewrite memoization.
    cache_key: u64,
    /// The rewrite options.
    options: TypeRewriterOptions,
    /// The local rewrite cache.
    cache: TypeRewriteCache,
}

#[allow(clippy::too_many_arguments)]
impl<'a> AssociatedAliasProjectionRewriter<'a> {
    /// Create a rewriter for a projected associated alias graph.
    pub(super) fn new(
        compiler: &'a Compiler,
        compiler_context: &'a CompilerContext<'a>,
        module: &'a Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &'a [StaticArgument],
        substitutions: &'a HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
    ) -> Self {
        // derive a stable rewrite key from owner and source
        let owner_key = owner_symbol.module_id.package_id.raw()
            ^ ((owner_symbol.module_id.local_id as u64) << 32)
            ^ ((owner_symbol.local_id.id as u64) << 1)
            ^ ((owner_symbol.local_id.ty as u64) << 53);
        let walk_context = TypeWalkContext::new(TypeWalkKey::BASE)
            .with_rewriter_tag(REWRITER_TAG_ASSOCIATED_ALIAS)
            .with_context_key(owner_key ^ source_id.cache_key());
        let options = walk_context.rewriter_options();
        let cache_key = options.cache_key();

        Self {
            compiler,
            compiler_context,
            module,
            profile,
            source_id,
            receiver_symbol,
            receiver_arguments,
            substitutions,
            tree,
            symbols,
            cache_key,
            options,
            cache: TypeRewriteCache::new(),
        }
    }

    /// Borrow the rewriter module and symbols as an immutable symbol view.
    fn module_symbol_view(&self) -> ModuleSymbolView<'_> {
        ModuleSymbolView::new(
            self.compiler_context,
            self.module,
            self.profile,
            self.symbols,
        )
    }
}

impl TypeRewriter for AssociatedAliasProjectionRewriter<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.options
    }

    fn rewrite_any(
        &mut self,
        types: &mut TypeTable,
        _type_id: LocalTypeId,
        ty: &Type,
    ) -> Option<LocalTypeId> {
        // only rewrite nominal references
        let Type::Reference {
            symbol,
            generic_arguments,
        } = ty
        else {
            return None;
        };

        // normalize the reference symbol before member-kind checks
        let mut symbol = self.compiler.canonical_symbol_id(
            self.module_symbol_view(),
            *symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        symbol = self
            .compiler
            .declaration_symbol_id(self.module_symbol_view(), symbol)
            .unwrap_or(symbol);

        let options = self
            .compiler_context
            .analyze_context_options_for_module(self.module.id);
        let mut ctx = TypeContext::new(
            self.compiler_context,
            self.module,
            self.profile,
            &options,
            self.tree,
            self.symbols,
            types,
            AnalyzeIndex::default(),
        );

        // derive owner substitutions for associated aliases from the projection receiver
        let owner_symbol = self
            .compiler
            .query_owner_symbol_for_member_symbol(self.module_symbol_view(), symbol)
            .ok()
            .flatten()?;
        let mut substitutions = self.substitutions.clone();
        if let Some(receiver_symbol) = self.receiver_symbol {
            let receiver_arguments = self.compiler.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                receiver_symbol,
                self.source_id,
                self.receiver_arguments,
            );
            let receiver_owner_substitutions = if owner_symbol.ty() == SymbolType::Interface {
                self.compiler
                    .interface_substitutions_for_owner_symbol(
                        &mut ctx.reborrow(),
                        self.source_id,
                        receiver_symbol,
                        &receiver_arguments,
                        owner_symbol,
                    )
                    .ok()
                    .flatten()
                    .or_else(|| {
                        self.compiler
                            .receiver_projection_substitutions_for_owner(
                                &mut ctx.reborrow(),
                                self.source_id,
                                receiver_symbol,
                                &receiver_arguments,
                                owner_symbol,
                            )
                            .ok()
                            .flatten()
                    })
            } else {
                self.compiler
                    .receiver_projection_substitutions_for_owner(
                        &mut ctx.reborrow(),
                        self.source_id,
                        receiver_symbol,
                        &receiver_arguments,
                        owner_symbol,
                    )
                    .ok()
                    .flatten()
            };

            if let Some(receiver_owner_substitutions) = receiver_owner_substitutions {
                let owner_comptime_substitutions = self
                    .compiler
                    .owner_comptime_substitutions_for_receiver(
                        &mut ctx.reborrow(),
                        self.source_id,
                        symbol,
                        receiver_symbol,
                        owner_symbol,
                        None,
                        Some(&receiver_owner_substitutions),
                    )
                    .ok()
                    .flatten();

                for (parameter_symbol, type_id) in receiver_owner_substitutions {
                    substitutions.entry(parameter_symbol).or_insert(type_id);
                }
                if let Some(owner_comptime_substitutions) = owner_comptime_substitutions {
                    for (parameter_symbol, type_id) in owner_comptime_substitutions {
                        substitutions.entry(parameter_symbol).or_insert(type_id);
                    }
                }
            }
        }

        // substitute inherited owner comptime references before alias expansion
        if let Some(substitution) = substitutions.get(&symbol).copied().or_else(|| {
            substitutions.iter().find_map(|(candidate, type_id)| {
                let matches_symbol = candidate.module_id == symbol.module_id
                    && candidate.local_id.id == symbol.local_id.id;
                if matches_symbol { Some(*type_id) } else { None }
            })
        }) {
            let mut substitution = substitution;
            while let Type::Value { value } = ctx.types.get_type(substitution) {
                substitution = *value;
            }

            return Some(substitution);
        }

        // keep non associated aliases unchanged
        if symbol.ty() != SymbolType::TypeAlias {
            return None;
        }

        // resolve the alias target for this member alias
        let alias_target_id = self.compiler.alias_target_type_id_for_symbol(
            &mut ctx.reborrow(),
            symbol,
            self.source_id,
        )?;

        // map explicit member arguments onto alias static parameters
        if let Some(member_arguments) = generic_arguments.as_deref() {
            let member_substitutions = self.compiler.build_type_parameter_substitutions_for_symbol(
                &mut ctx.reborrow(),
                symbol,
                self.source_id,
                member_arguments,
            );
            substitutions.extend(member_substitutions);
        }

        // apply substitutions into the alias target
        let mapped_alias_id = if substitutions.is_empty() {
            alias_target_id
        } else {
            let mut substitution_cache = HashMap::new();
            let mapped_alias_id = self.compiler.substitute_static_parameters(
                alias_target_id,
                &substitutions,
                ctx.types,
                &mut substitution_cache,
            );
            self.compiler
                .apply_associated_projection_substitutions(
                    &mut ctx.reborrow(),
                    symbol,
                    mapped_alias_id,
                    &substitutions,
                )
                .ok()?
        };

        // materialize static arguments after substitution
        let mut materialize_cache = TypeRewriteCache::new();
        Some(self.compiler.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            mapped_alias_id,
            &mut materialize_cache,
        ))
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, type_id: LocalTypeId) -> LocalTypeId {
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, type_id);
        self.cache = cache;
        mapped
    }
}

/// Requirements for one associated type member from an inherited contract.
#[derive(Clone, Debug)]
pub(crate) struct AssociatedTypeRequirement {
    /// The associated type name.
    pub(crate) name: StringId,
    /// The contract member symbol.
    pub(crate) symbol: GlobalSymbolId,
    /// The contract member parameter symbols.
    pub(crate) parameter_symbols: Vec<GlobalSymbolId>,
    /// The optional bound expression node on the contract member.
    pub(crate) bound_node: Option<GlobalNodeIdAny>,
    /// Whether the contract member requires an explicit implementation.
    pub(crate) requires_implementation: bool,
}

/// Requirements for one associated comptime member from an inherited contract.
#[derive(Clone, Debug)]
pub(crate) struct AssociatedComptimeRequirement {
    /// The associated comptime member name.
    pub(crate) name: StringId,
    /// The contract member symbol.
    pub(crate) symbol: GlobalSymbolId,
    /// The optional type expression node on the contract member.
    pub(crate) type_node: Option<GlobalNodeIdAny>,
    /// Whether the contract member requires an explicit implementation.
    pub(crate) requires_implementation: bool,
}

/// Classification for static symbols used in associated projection paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StaticMemberSymbolKind {
    /// An associated type member.
    AssociatedType,
    /// An associated comptime member.
    AssociatedComptimeConst,
    /// An enum field member.
    EnumField,
    /// Any other static member kind.
    Other,
}

/// Selection result for an associated projection member lookup.
#[derive(Clone, Debug)]
pub(crate) struct AssociatedProjectionSelection {
    /// The projected associated member symbol.
    pub(crate) target_symbol: GlobalSymbolId,
    /// The receiver symbol used for projection.
    pub(crate) receiver_symbol: GlobalSymbolId,
    /// The receiver static arguments used for projection.
    pub(crate) receiver_arguments: Vec<StaticArgument>,
}

/// Canonical substitution environment for one associated projection.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProjectionEnvironment {
    /// The owner symbol that declares the projected member.
    pub(crate) owner_symbol: Option<GlobalSymbolId>,
    /// The normalized receiver symbol used for projection, when present.
    pub(crate) receiver_symbol: Option<GlobalSymbolId>,
    /// The normalized receiver static arguments used for projection.
    pub(crate) receiver_arguments: Vec<StaticArgument>,
    /// The merged substitutions for receiver, extension, and member parameters.
    pub(crate) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Primary semantic faults that block cascading missing-member diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingMemberDiagnosticBlocker {
    /// Receiver type already failed earlier analysis.
    ReceiverTypeError,
    /// Receiver type is still indeterminate for stable member diagnostics.
    ReceiverTypeIndeterminate,
    /// Receiver declaration is missing required associated implementations.
    UnsatisfiedAssociatedContractRequirements,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect parent type expressions for one nominal declaration.
    fn associated_parent_types(
        &self,
        declaration: &Declaration,
    ) -> Vec<LocalNodeId<TypeExpression>> {
        let mut parent_types = Vec::new();

        // class: implements only
        if let Declaration::Class(declaration) = declaration {
            parent_types.extend(declaration.implements_types.iter().copied());

            return parent_types;
        }

        // struct and enum: implements only
        if let Declaration::Struct(declaration) = declaration {
            parent_types.extend(declaration.implements_types.iter().copied());

            return parent_types;
        }
        if let Declaration::Enum(declaration) = declaration {
            parent_types.extend(declaration.implements_types.iter().copied());

            return parent_types;
        }

        // interface: extends only
        if let Declaration::Interface(declaration) = declaration {
            parent_types.extend(declaration.extends_types.iter().copied());
        }

        parent_types
    }

    /// Normalize one projection receiver reference to the owner-facing nominal receiver.
    pub(crate) fn normalize_projection_receiver_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
    ) -> AnalyzeResult<(GlobalSymbolId, Vec<StaticArgument>)> {
        let mut receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        receiver_symbol = self.resolve_type_reference_symbol(ctx, receiver_symbol);
        receiver_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), receiver_symbol)
            .unwrap_or(receiver_symbol);
        let mut receiver_arguments = receiver_arguments.to_vec();

        // follow static parameter constraints before alias projection
        if self.symbol_is_static_parameter(ctx.symbol_type_view(), receiver_symbol) {
            let constraint_ty_id = self.projection_static_parameter_constraint_type(
                &mut ctx.reborrow(),
                source_id,
                receiver_symbol,
            )?;
            if let Some(constraint_ty_id) = constraint_ty_id
                && let Type::Reference {
                    symbol,
                    generic_arguments,
                } = ctx.types.get_type(constraint_ty_id)
            {
                receiver_symbol = *symbol;
                receiver_arguments = generic_arguments.clone().unwrap_or_default();
            }
        }

        // project through alias references to reach the nominal receiver owner
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(&mut ctx.reborrow(), receiver_symbol, source_id)
        {
            let mapped_alias_target = if receiver_arguments.is_empty() {
                alias_target_id
            } else {
                let substitutions = self.build_type_parameter_substitutions_for_symbol(
                    &mut ctx.reborrow(),
                    receiver_symbol,
                    source_id,
                    &receiver_arguments,
                );
                if substitutions.is_empty() {
                    alias_target_id
                } else {
                    let mut substitution_cache = HashMap::new();
                    self.substitute_static_parameters(
                        alias_target_id,
                        &substitutions,
                        ctx.types,
                        &mut substitution_cache,
                    )
                }
            };

            let mut materialize_cache = TypeRewriteCache::new();
            let mapped_alias_target = self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                mapped_alias_target,
                &mut materialize_cache,
            );
            if let Some((alias_symbol, generic_arguments, _)) =
                self.unwrap_type_symbol(ctx.types, mapped_alias_target)
            {
                receiver_symbol = alias_symbol;
                receiver_arguments = generic_arguments.unwrap_or_default();
            }
        }

        receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        receiver_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), receiver_symbol)
            .unwrap_or(receiver_symbol);

        Ok((receiver_symbol, receiver_arguments))
    }

    /// Return true when one declaration symbol has unimplemented associated requirements.
    pub(crate) fn symbol_has_missing_associated_requirements(
        &self,
        ctx: TypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<bool> {
        let Some(symbol) = self
            .declaration_symbol_id_for_artifact(
                ctx.module_symbol_view(),
                symbol,
                destack_artifact::ArtifactKey::dir_declared,
            )
            .map_err(AnalyzeError::from)?
        else {
            return Ok(false);
        };
        if !matches!(
            symbol.ty(),
            SymbolType::Class | SymbolType::Interface | SymbolType::Struct
        ) {
            return Ok(false);
        }

        let has_missing_requirements = self
            .with_module_types_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.types,
                destack_artifact::ArtifactKey::dir_declared,
                |_, owner_types| {
                    owner_types.symbol_has_unimplemented_associated_requirements(symbol)
                },
            )
            .map_err(AnalyzeError::from)?;

        Ok(has_missing_requirements)
    }

    /// Return true when one receiver type resolves to a symbol with unsatisfied associated requirements.
    pub(crate) fn receiver_has_missing_associated_requirements(
        &self,
        ctx: TypeView<'_>,
        receiver_ty_id: LocalTypeId,
    ) -> AnalyzeResult<bool> {
        let Some((receiver_symbol, _, _)) = self.unwrap_type_symbol(ctx.types, receiver_ty_id)
        else {
            return Ok(false);
        };

        self.symbol_has_missing_associated_requirements(ctx, receiver_symbol)
    }

    /// Return the primary semantic blocker for one missing-member diagnostic, when present.
    pub(crate) fn should_block_missing_member_diagnostic(
        &self,
        ctx: TypeView<'_>,
        receiver_ty_id: LocalTypeId,
        allow_associated_contract_blocker: bool,
    ) -> AnalyzeResult<Option<MissingMemberDiagnosticBlocker>> {
        if self.type_blocks_cascading_diagnostic(receiver_ty_id, ctx.types) {
            return Ok(Some(MissingMemberDiagnosticBlocker::ReceiverTypeError));
        }

        let receiver_unwrapped_ty_id = ctx.types.unwrap_value_type_id(receiver_ty_id);
        if self.type_is_solver_placeholder(receiver_unwrapped_ty_id, ctx.types)
            || self.unwrapped_value_type_is_unevaluated(receiver_unwrapped_ty_id, ctx.types)
        {
            return Ok(Some(
                MissingMemberDiagnosticBlocker::ReceiverTypeIndeterminate,
            ));
        }

        if let Type::Reference { symbol, .. } = ctx.types.get_type(receiver_unwrapped_ty_id) {
            let symbol_value_type_id = ctx.types.get_value_type_id(*symbol);
            if symbol_value_type_id.is_some_and(|symbol_value_type_id| {
                let symbol_unwrapped_ty_id = ctx.types.unwrap_value_type_id(symbol_value_type_id);
                self.type_is_solver_placeholder(symbol_unwrapped_ty_id, ctx.types)
                    || self.unwrapped_value_type_is_unevaluated(symbol_unwrapped_ty_id, ctx.types)
            }) {
                return Ok(Some(
                    MissingMemberDiagnosticBlocker::ReceiverTypeIndeterminate,
                ));
            }
        }

        if allow_associated_contract_blocker
            && self.receiver_has_missing_associated_requirements(ctx, receiver_ty_id)?
        {
            return Ok(Some(
                MissingMemberDiagnosticBlocker::UnsatisfiedAssociatedContractRequirements,
            ));
        }

        Ok(None)
    }

    /// Report one missing-member diagnostic unless a primary semantic blocker applies.
    pub(crate) fn report_missing_member_diagnostic(
        &self,
        ctx: TypeView<'_>,
        expression_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
        member_key: StaticKey,
        allow_associated_contract_blocker: bool,
    ) -> AnalyzeResult<bool> {
        let blocker = self.should_block_missing_member_diagnostic(
            ctx,
            receiver_ty_id,
            allow_associated_contract_blocker,
        )?;
        if blocker.is_some() {
            return Ok(false);
        }

        let error = AnalyzeError::MissingMember {
            node: expression_id
                .into_global(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
            receiver_ty: receiver_ty_id.into_global(ctx.module.id),
            member_key,
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);

        Ok(true)
    }

    /// Collect contract associated type requirements for one contract symbol.
    pub(crate) fn collect_contract_associated_type_requirements(
        &self,
        ctx: TreeSymbolView<'_>,
        contract_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<AssociatedTypeRequirement>> {
        // normalize contract references to declaration owners
        let Some(contract_symbol) = self
            .declaration_symbol_id_for_artifact(
                ctx.module_symbol_view(),
                contract_symbol,
                destack_artifact::ArtifactKey::dir_declared,
            )
            .map_err(AnalyzeError::from)?
        else {
            return Ok(Vec::new());
        };

        // collect requirements with cycle protection
        let mut visited_contracts = HashSet::new();
        self.collect_contract_associated_type_requirements_inner(
            ctx,
            contract_symbol,
            &mut visited_contracts,
        )
    }

    /// Collect contract associated type requirements through declaration heritage.
    fn collect_contract_associated_type_requirements_inner(
        &self,
        ctx: TreeSymbolView<'_>,
        contract_symbol: GlobalSymbolId,
        visited_contracts: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Vec<AssociatedTypeRequirement>> {
        // normalize contract declarations and break recursive cycles
        let Some(contract_symbol) =
            self.declaration_symbol_id(ctx.module_symbol_view(), contract_symbol)
        else {
            return Ok(Vec::new());
        };
        if !visited_contracts.insert(contract_symbol) {
            return Ok(Vec::new());
        }

        // collect local requirements and direct parent contracts
        let (local_requirements, parent_contracts) = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                contract_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| {
                    let mut requirements = Vec::new();
                    let mut parents = Vec::new();

                    // resolve the contract declaration node
                    let symbol_entry = view.symbols.get_symbol(contract_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return (requirements, parents);
                    };
                    if primary_declaration.local_id.ty != NodeType::Declaration {
                        return (requirements, parents);
                    }

                    let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                    let declaration = view.tree.get(declaration_id);
                    let Some(members) = declaration.member_ids() else {
                        return (requirements, parents);
                    };
                    // collect local associated requirements
                    for member_id in members {
                        let Member::AssociatedType {
                            name,
                            generic_parameters,
                            constraint,
                            value,
                            symbol,
                            ..
                        } = view.tree.get(*member_id)
                        else {
                            continue;
                        };

                        let parameter_symbols = generic_parameters
                            .iter()
                            .map(|parameter_id| {
                                view.tree
                                    .get(*parameter_id)
                                    .symbol()
                                    .into_global(view.module.id)
                            })
                            .collect::<Vec<_>>();

                        requirements.push(AssociatedTypeRequirement {
                            name: *name,
                            symbol: symbol.into_global(view.module.id),
                            parameter_symbols,
                            bound_node: constraint
                                .map(|constraint| constraint.into_global_any(view.module.id)),
                            requires_implementation: value.is_none(),
                        });
                    }

                    // collect parent contracts from extends and implements
                    let parent_types = self.associated_parent_types(declaration);
                    for parent_type_id in parent_types {
                        let Some(parent_symbol) = view.tree.get(parent_type_id).target_symbol()
                        else {
                            continue;
                        };

                        let view = view.module_symbol_view();
                        let canonical_parent = self.canonical_symbol_id(
                            view,
                            parent_symbol,
                            CanonicalSymbolMode::FollowAliases,
                        );
                        let Some(parent_symbol) =
                            self.declaration_symbol_id(view, canonical_parent)
                        else {
                            continue;
                        };
                        parents.push(parent_symbol);
                    }

                    (requirements, parents)
                },
            )
            .map_err(AnalyzeError::from)?;

        // collect inherited requirements before local overrides
        let mut requirements = Vec::new();
        for parent_contract in parent_contracts {
            let inherited = self.collect_contract_associated_type_requirements_inner(
                ctx,
                parent_contract,
                visited_contracts,
            )?;
            requirements.extend(inherited);
        }

        // apply local overrides by associated type name
        for local_requirement in local_requirements {
            requirements.retain(|requirement| requirement.name != local_requirement.name);
            requirements.push(local_requirement);
        }

        Ok(requirements)
    }

    /// Collect contract associated comptime requirements for one contract symbol.
    pub(crate) fn collect_contract_associated_comptime_requirements(
        &self,
        ctx: TreeSymbolView<'_>,
        contract_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<AssociatedComptimeRequirement>> {
        // normalize contract references to declaration owners
        let Some(contract_symbol) = self
            .declaration_symbol_id_for_artifact(
                ctx.module_symbol_view(),
                contract_symbol,
                destack_artifact::ArtifactKey::dir_declared,
            )
            .map_err(AnalyzeError::from)?
        else {
            return Ok(Vec::new());
        };

        // collect requirements with cycle protection
        let mut visited_contracts = HashSet::new();
        self.collect_contract_associated_comptime_requirements_inner(
            ctx,
            contract_symbol,
            &mut visited_contracts,
        )
    }

    /// Collect contract associated comptime requirements through declaration heritage.
    fn collect_contract_associated_comptime_requirements_inner(
        &self,
        ctx: TreeSymbolView<'_>,
        contract_symbol: GlobalSymbolId,
        visited_contracts: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Vec<AssociatedComptimeRequirement>> {
        // normalize contract declarations and break recursive cycles
        let Some(contract_symbol) =
            self.declaration_symbol_id(ctx.module_symbol_view(), contract_symbol)
        else {
            return Ok(Vec::new());
        };
        if !visited_contracts.insert(contract_symbol) {
            return Ok(Vec::new());
        }

        // collect local requirements and direct parent contracts
        let (local_requirements, parent_contracts) = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                contract_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |view| {
                    let mut requirements = Vec::new();
                    let mut parents = Vec::new();

                    // resolve the contract declaration node
                    let symbol_entry = view.symbols.get_symbol(contract_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return (requirements, parents);
                    };
                    if primary_declaration.local_id.ty != NodeType::Declaration {
                        return (requirements, parents);
                    }

                    let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                    let declaration = view.tree.get(declaration_id);
                    let Some(members) = declaration.member_ids() else {
                        return (requirements, parents);
                    };
                    // collect local associated requirements
                    for member_id in members {
                        let Member::AssociatedConst {
                            name,
                            declared_type,
                            value,
                            symbol,
                            ..
                        } = view.tree.get(*member_id)
                        else {
                            continue;
                        };

                        requirements.push(AssociatedComptimeRequirement {
                            name: *name,
                            symbol: symbol.into_global(view.module.id),
                            type_node: declared_type
                                .map(|declared_type| declared_type.into_global_any(view.module.id)),
                            requires_implementation: value.is_none(),
                        });
                    }

                    // collect parent contracts from extends and implements
                    let parent_types = self.associated_parent_types(declaration);
                    for parent_type_id in parent_types {
                        let Some(parent_symbol) = view.tree.get(parent_type_id).target_symbol()
                        else {
                            continue;
                        };

                        let view = view.module_symbol_view();
                        let canonical_parent = self.canonical_symbol_id(
                            view,
                            parent_symbol,
                            CanonicalSymbolMode::FollowAliases,
                        );
                        let Some(parent_symbol) =
                            self.declaration_symbol_id(view, canonical_parent)
                        else {
                            continue;
                        };
                        parents.push(parent_symbol);
                    }

                    (requirements, parents)
                },
            )
            .map_err(AnalyzeError::from)?;

        // collect inherited requirements before local overrides
        let mut requirements = Vec::new();
        for parent_contract in parent_contracts {
            let inherited = self.collect_contract_associated_comptime_requirements_inner(
                ctx,
                parent_contract,
                visited_contracts,
            )?;
            requirements.extend(inherited);
        }

        // apply local overrides by associated comptime name
        for local_requirement in local_requirements {
            requirements.retain(|requirement| requirement.name != local_requirement.name);
            requirements.push(local_requirement);
        }

        Ok(requirements)
    }

    /// Select a projected static member symbol from a nominal receiver.
    pub(crate) fn select_associated_projection_member_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        preferred_kind: Option<StaticMemberSymbolKind>,
        _validate_static_argument_bounds: bool,
        _enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<AssociatedProjectionSelection>> {
        // evaluate the receiver to a reference-like type
        let left_ty_id = ctx
            .types
            .get_declared_or_inferred_type_id(left.into_global_any(ctx.module.id))
            .ok_or_else(|| AnalyzeError::MissingType {
                node: left
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            })?;
        let left_ty = ctx.types.get_type(left_ty_id).clone();
        let receiver_reference = self
            .unwrap_type_symbol(ctx.types, left_ty_id)
            .map(|(symbol, generic_arguments, _)| (symbol, generic_arguments.unwrap_or_default()))
            .or_else(|| match left_ty {
                Type::Intersection { elements } | Type::Union { elements } => {
                    elements.iter().find_map(|element_id| {
                        self.unwrap_type_symbol(ctx.types, *element_id).map(
                            |(symbol, generic_arguments, _)| {
                                (symbol, generic_arguments.unwrap_or_default())
                            },
                        )
                    })
                }
                Type::This => self.owner_symbol_for_this_expression(ctx.tree_symbol_view(), left),
                _ => None,
            });
        // prefer the explicit syntax receiver when it exists:
        // namespace imports and other projected receivers can carry more precise
        // symbol information than the inferred left type
        let syntax_receiver =
            self.associated_projection_receiver_from_expression(&mut ctx.reborrow(), left)?;
        let receiver_reference = match (syntax_receiver, receiver_reference) {
            (
                Some((syntax_symbol, syntax_arguments)),
                Some((fallback_symbol, fallback_arguments)),
            ) if syntax_symbol == fallback_symbol => {
                let syntax_requires_deferral = self.receiver_projection_arguments_require_deferral(
                    ctx.type_view(),
                    &syntax_arguments,
                );
                let fallback_requires_deferral = self
                    .receiver_projection_arguments_require_deferral(
                        ctx.type_view(),
                        &fallback_arguments,
                    );
                let should_use_fallback_arguments = (syntax_requires_deferral
                    && !fallback_requires_deferral)
                    || (syntax_arguments.is_empty() && !fallback_arguments.is_empty());
                let chosen_arguments = if should_use_fallback_arguments {
                    fallback_arguments
                } else {
                    syntax_arguments
                };

                Some((syntax_symbol, chosen_arguments))
            }
            (Some(syntax_receiver), _) => Some(syntax_receiver),
            (None, fallback_receiver) => fallback_receiver,
        };

        let Some((receiver_symbol, receiver_arguments)) = receiver_reference else {
            return Ok(None);
        };
        let (projection_receiver_symbol, projection_receiver_arguments) = self
            .normalize_projection_receiver_reference(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                receiver_symbol,
                &receiver_arguments,
            )?;

        let lookup_symbol = projection_receiver_symbol;

        // resolve the projected member symbol on the normalized receiver symbol
        let projected_symbol = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                lookup_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_interface,
                |view| {
                    self.query_static_member_symbol(
                        ctx.compiler_context.revision(),
                        view.module,
                        ctx.profile,
                        lookup_symbol,
                        member_key,
                        view.tree,
                        view.symbols,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;
        let Some(mut projected_symbol) = projected_symbol else {
            return Ok(None);
        };

        // prefer one member-kind class when the owner has ambiguous same-name members
        if let Some(preferred_kind) = preferred_kind
            && let Some(preferred_symbol) = self.query_direct_member_symbol_for_key_and_kind(
                &*ctx,
                lookup_symbol,
                member_key,
                preferred_kind,
            )?
        {
            projected_symbol = preferred_symbol;
        }

        Ok(Some(AssociatedProjectionSelection {
            target_symbol: projected_symbol,
            receiver_symbol: projection_receiver_symbol,
            receiver_arguments: projection_receiver_arguments,
        }))
    }

    /// Select a projected static member symbol from a nominal type receiver.
    pub(crate) fn select_associated_projection_type_member_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        left: LocalNodeId<TypeExpression>,
        member_key: StaticKey,
        preferred_kind: Option<StaticMemberSymbolKind>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<AssociatedProjectionSelection>> {
        // evaluate the receiver to a reference-like type
        let left_ty_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            left,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        let left_ty = ctx.types.get_type(left_ty_id).clone();
        let receiver_reference = self
            .unwrap_type_symbol(ctx.types, left_ty_id)
            .map(|(symbol, generic_arguments, _)| (symbol, generic_arguments.unwrap_or_default()))
            .or_else(|| match left_ty {
                Type::Intersection { elements } | Type::Union { elements } => {
                    elements.iter().find_map(|element_id| {
                        self.unwrap_type_symbol(ctx.types, *element_id).map(
                            |(symbol, generic_arguments, _)| {
                                (symbol, generic_arguments.unwrap_or_default())
                            },
                        )
                    })
                }
                Type::This => {
                    self.owner_symbol_for_this_type_expression(ctx.tree_symbol_view(), left)
                }
                _ => None,
            });

        // prefer the explicit syntax receiver when it exists:
        // namespace imports and other projected receivers can carry more precise
        // symbol information than the inferred left type
        let syntax_receiver =
            self.associated_projection_receiver_from_type_expression(&mut ctx.reborrow(), left)?;
        let receiver_reference = match (syntax_receiver, receiver_reference) {
            (
                Some((syntax_symbol, syntax_arguments)),
                Some((fallback_symbol, fallback_arguments)),
            ) if syntax_symbol == fallback_symbol => {
                let syntax_requires_deferral = self.receiver_projection_arguments_require_deferral(
                    ctx.type_view(),
                    &syntax_arguments,
                );
                let fallback_requires_deferral = self
                    .receiver_projection_arguments_require_deferral(
                        ctx.type_view(),
                        &fallback_arguments,
                    );
                let should_use_fallback_arguments = (syntax_requires_deferral
                    && !fallback_requires_deferral)
                    || (syntax_arguments.is_empty() && !fallback_arguments.is_empty());
                let chosen_arguments = if should_use_fallback_arguments {
                    fallback_arguments
                } else {
                    syntax_arguments
                };

                Some((syntax_symbol, chosen_arguments))
            }
            (Some(syntax_receiver), _) => Some(syntax_receiver),
            (None, fallback_receiver) => fallback_receiver,
        };

        let Some((receiver_symbol, receiver_arguments)) = receiver_reference else {
            return Ok(None);
        };
        let (projection_receiver_symbol, projection_receiver_arguments) = self
            .normalize_projection_receiver_reference(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                receiver_symbol,
                &receiver_arguments,
            )?;

        let lookup_symbol = projection_receiver_symbol;

        // resolve the projected member symbol on the normalized receiver symbol
        let projected_symbol = self
            .with_module_tree_symbol_view_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                lookup_symbol.module_id,
                ctx.tree,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_interface,
                |view| {
                    self.query_static_member_symbol(
                        ctx.compiler_context.revision(),
                        view.module,
                        ctx.profile,
                        lookup_symbol,
                        member_key,
                        view.tree,
                        view.symbols,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;
        let Some(mut projected_symbol) = projected_symbol else {
            return Ok(None);
        };

        // prefer one member-kind class when the owner has ambiguous same-name members
        if let Some(preferred_kind) = preferred_kind
            && let Some(preferred_symbol) = self.query_direct_member_symbol_for_key_and_kind(
                &*ctx,
                lookup_symbol,
                member_key,
                preferred_kind,
            )?
        {
            projected_symbol = preferred_symbol;
        }

        Ok(Some(AssociatedProjectionSelection {
            target_symbol: projected_symbol,
            receiver_symbol: projection_receiver_symbol,
            receiver_arguments: projection_receiver_arguments,
        }))
    }

    /// Return true when associated projection receiver arguments require deferral.
    pub(crate) fn receiver_projection_arguments_require_deferral(
        &self,
        ctx: TypeView<'_>,
        arguments: &[StaticArgument],
    ) -> bool {
        for argument in arguments {
            if self.projection_argument_requires_deferral(ctx, argument) {
                return true;
            }
        }

        false
    }

    /// Return true when associated projection receiver arguments still depend on static parameters.
    pub(crate) fn receiver_projection_arguments_have_static_parameters(
        &self,
        ctx: TypeView<'_>,
        arguments: &[StaticArgument],
    ) -> bool {
        for argument in arguments {
            // unresolved arguments still depend on static-parameter substitution
            let StaticArgument::Evaluated { value, .. } = argument else {
                return true;
            };

            let StaticExpression::Type { ty } = value else {
                continue;
            };

            let mut visited = HashSet::new();
            if self.type_contains_static_parameters(ctx, *ty, &mut visited) {
                return true;
            }
        }

        false
    }

    /// Return true when one associated projection static argument requires deferral.
    fn projection_argument_requires_deferral(
        &self,
        ctx: TypeView<'_>,
        argument: &StaticArgument,
    ) -> bool {
        // unevaluated static arguments are unresolved
        let StaticArgument::Evaluated { value, .. } = argument else {
            return true;
        };

        // type arguments must converge before projection materialization
        let StaticExpression::Type { ty } = value else {
            return false;
        };

        !self.type_is_converged_for_static_evaluation(ctx, *ty)
    }

    /// Build a projection receiver from one expression when type evaluation is unavailable.
    pub(crate) fn associated_projection_receiver_from_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
        // use the same receiver resolution path as ordinary member lookup
        let expression_id = self.unwrap_parenthesized_expression(expression_id, ctx.tree);
        let expression = ctx.tree.get(expression_id);
        let target_symbol = self.resolve_direct_receiver_symbol_for_expression(ctx, expression_id);
        let Some(target_symbol) = target_symbol else {
            return Ok(None);
        };

        // preserve and resolve explicit static arguments from syntax
        let generic_argument_nodes = expression.generic_arguments();
        let generic_arguments =
            self.evaluate_generic_arguments(&mut ctx.reborrow(), generic_argument_nodes)?;
        let generic_arguments = self.resolve_type_reference_static_arguments(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            target_symbol,
            generic_arguments.as_deref(),
            false,
        )?;
        let generic_arguments = generic_arguments.unwrap_or_default();

        let mut target_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        target_symbol = self.resolve_type_reference_symbol(ctx, target_symbol);
        target_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), target_symbol)
            .unwrap_or(target_symbol);

        Ok(Some((target_symbol, generic_arguments)))
    }

    /// Build a projection receiver from one type expression when type evaluation is unavailable.
    pub(crate) fn associated_projection_receiver_from_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
        // use the same receiver resolution path as ordinary member lookup
        let expression_id = self.unwrap_parenthesized_type_expression(expression_id, ctx.tree);
        let expression = ctx.tree.get(expression_id);
        let target_symbol =
            self.resolve_direct_receiver_symbol_for_type_expression(ctx, expression_id);
        let Some(target_symbol) = target_symbol else {
            return Ok(None);
        };

        // preserve and resolve explicit static arguments from syntax
        let generic_argument_nodes = expression.generic_arguments();
        let generic_arguments =
            self.evaluate_generic_arguments(&mut ctx.reborrow(), generic_argument_nodes)?;
        let generic_arguments = self.resolve_type_reference_static_arguments(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            target_symbol,
            generic_arguments.as_deref(),
            false,
        )?;
        let generic_arguments = generic_arguments.unwrap_or_default();

        let mut target_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        target_symbol = self.resolve_type_reference_symbol(ctx, target_symbol);
        target_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), target_symbol)
            .unwrap_or(target_symbol);

        Ok(Some((target_symbol, generic_arguments)))
    }

    /// Query one owner member symbol for one key and one static-member kind.
    pub(crate) fn query_direct_member_symbol_for_key_and_kind(
        &self,
        ctx: &TypeContext<'_>,
        owner_symbol: GlobalSymbolId,
        member_key: StaticKey,
        preferred_kind: StaticMemberSymbolKind,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let Some(member_name) = member_key.name() else {
            return Ok(None);
        };

        let owner_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
            .unwrap_or(owner_symbol);

        let symbol = self
            .query_declared_direct_member_symbol_for_name_and_kind(
                ctx.compiler_context,
                ctx.module.id,
                ctx.profile,
                owner_symbol.module_id,
                owner_symbol,
                member_name,
                preferred_kind,
                ctx.tree,
                ctx.symbols,
            )
            .map_err(AnalyzeError::from)?;
        if symbol.is_some() {
            return Ok(symbol);
        }

        let symbol = self.query_extension_direct_member_symbol_for_name_and_kind(
            ctx,
            owner_symbol,
            member_name,
            preferred_kind,
        )?;

        Ok(symbol)
    }

    /// Query one direct declared member symbol from one immutable declared view.
    fn query_declared_direct_member_symbol_in_view(
        &self,
        view: TreeSymbolView<'_>,
        owner_symbol: GlobalSymbolId,
        member_name: StringId,
        preferred_kind: StaticMemberSymbolKind,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = view.symbols.get_symbol(owner_symbol.local_id);
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        for declaration_id in declaration_ids {
            if declaration_id.local_id.ty != NodeType::Declaration {
                continue;
            }
            let declaration_id = declaration_id.local_id.into_typed::<Declaration>();
            let declaration = view.tree.get(declaration_id);
            let Some(members) = declaration.member_ids() else {
                continue;
            };

            for member_id in members {
                let member = view.tree.get(*member_id);
                let (name, symbol, kind) = match member {
                    Member::AssociatedType { name, symbol, .. } => (
                        *name,
                        self.typed_global_symbol_id(view.module.id, view.symbols, *symbol),
                        StaticMemberSymbolKind::AssociatedType,
                    ),
                    Member::AssociatedConst { name, symbol, .. } => (
                        *name,
                        self.typed_global_symbol_id(view.module.id, view.symbols, *symbol),
                        StaticMemberSymbolKind::AssociatedComptimeConst,
                    ),
                    _ => continue,
                };

                if name == member_name && kind == preferred_kind {
                    return Some(symbol);
                }
            }
        }

        None
    }

    /// Query one direct declared member symbol by name and static-member kind.
    fn query_declared_direct_member_symbol_for_name_and_kind(
        &self,
        context: &CompilerContext<'_>,
        current_module_id: ModuleId,
        profile_id: ProfileId,
        owner_module_id: ModuleId,
        owner_symbol: GlobalSymbolId,
        member_name: StringId,
        preferred_kind: StaticMemberSymbolKind,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        let revision = context.revision();
        let module = self
            .repository
            .module(revision, current_module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing module snapshot for {current_module_id:?}"),
            })?;
        let module = module.as_ref();

        Ok(self.with_module_tree_symbol_view_or_local_for_artifact(
            context,
            module,
            profile_id,
            owner_module_id,
            tree,
            symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                self.query_declared_direct_member_symbol_in_view(
                    view,
                    owner_symbol,
                    member_name,
                    preferred_kind,
                )
            },
        )?)
    }

    /// Query one direct extension member symbol by name and static-member kind.
    fn query_extension_direct_member_symbol_for_name_and_kind(
        &self,
        ctx: &TypeContext<'_>,
        owner_symbol: GlobalSymbolId,
        member_name: StringId,
        preferred_kind: StaticMemberSymbolKind,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let extension_symbols =
            self.visible_extension_symbols_for_target(ctx.symbol_type_view(), owner_symbol)?;
        for extension_symbol in extension_symbols {
            let Some(extension) =
                self.extension_for_symbol_in_module(ctx.module_type_view(), extension_symbol)?
            else {
                continue;
            };
            if !self.is_extension_visible(
                ctx.compiler_context.revision(),
                ctx.module,
                ctx.profile,
                &extension,
            )? {
                continue;
            }

            let symbol = self
                .with_module_tree_symbol_view_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    extension_symbol.module_id,
                    ctx.tree,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |view| {
                        self.query_declared_direct_member_symbol_in_view(
                            view,
                            extension_symbol,
                            member_name,
                            preferred_kind,
                        )
                    },
                )
                .map_err(AnalyzeError::from)?;
            if symbol.is_some() {
                return Ok(symbol);
            }
        }

        Ok(None)
    }

    /// Resolve one associated member symbol for a concrete receiver and static member key.
    pub(crate) fn resolve_associated_member_symbol_for_receiver(
        &self,
        ctx: &mut TypeContext<'_>,
        receiver_symbol: GlobalSymbolId,
        member_key: StaticKey,
        member_kind: StaticMemberSymbolKind,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let mut receiver_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        receiver_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), receiver_symbol)
            .unwrap_or(receiver_symbol);

        let Some(mut member_symbol) = self.query_static_member_symbol(
            ctx.compiler_context.revision(),
            ctx.module,
            ctx.profile,
            receiver_symbol,
            member_key,
            ctx.tree,
            ctx.symbols,
        ) else {
            return Ok(None);
        };

        if let Some(preferred_symbol) = self.query_direct_member_symbol_for_key_and_kind(
            &*ctx,
            receiver_symbol,
            member_key,
            member_kind,
        )? {
            member_symbol = preferred_symbol;
        }

        if self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), member_symbol)?
            != Some(member_kind)
        {
            return Ok(None);
        }

        let member_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            member_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let member_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), member_symbol)
            .unwrap_or(member_symbol);

        Ok(Some(member_symbol))
    }

    /// Resolve a declaration owner symbol for one `this` receiver expression.
    pub(crate) fn owner_symbol_for_this_expression(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        // walk parent nodes until we find a declaration owner
        let mut current_id = expression_id.id;
        while let Some(parent) = ctx.tree.get_parent(current_id) {
            if parent.ty == NodeType::Declaration {
                let declaration_id = parent.into_typed::<Declaration>();
                let owner_symbol = match ctx.tree.get(declaration_id) {
                    Declaration::Class(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Struct(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Interface(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Enum(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Extension(declaration) => declaration.target_symbol,
                    _ => None,
                }?;

                let owner_symbol = self
                    .declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
                    .unwrap_or(owner_symbol);
                return Some((owner_symbol, Vec::new()));
            }

            current_id = parent.id;
        }

        None
    }

    /// Resolve a declaration owner symbol for one `this` receiver type expression.
    pub(crate) fn owner_symbol_for_this_type_expression(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        // walk parent nodes until we find a declaration owner
        let mut current_id = expression_id.id;
        while let Some(parent) = ctx.tree.get_parent(current_id) {
            if parent.ty == NodeType::Declaration {
                let declaration_id = parent.into_typed::<Declaration>();
                let owner_symbol = match ctx.tree.get(declaration_id) {
                    Declaration::Class(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Struct(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Interface(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Enum(declaration) => {
                        Some(declaration.symbol.into_global(ctx.module.id))
                    }
                    Declaration::Extension(declaration) => declaration.target_symbol,
                    _ => None,
                }?;

                let owner_symbol = self
                    .declaration_symbol_id(ctx.module_symbol_view(), owner_symbol)
                    .unwrap_or(owner_symbol);
                return Some((owner_symbol, Vec::new()));
            }

            current_id = parent.id;
        }

        None
    }

    /// Build projection receiver arguments that reference one owner's own static parameters.
    pub(crate) fn owner_projection_receiver_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        owner_symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Vec<StaticArgument> {
        let Some(parameter_symbols) =
            self.collect_static_parameter_symbols(ctx.type_view(), owner_symbol)
        else {
            return Vec::new();
        };

        let mut receiver_arguments = Vec::with_capacity(parameter_symbols.len());
        for parameter_symbol in parameter_symbols {
            let parameter =
                self.resolve_static_parameter(&mut ctx.reborrow(), parameter_symbol, source_id);
            let reference_ty = if let Some(substituted_ty_id) = substitutions.get(&parameter_symbol)
            {
                *substituted_ty_id
            } else {
                ctx.types.insert_type_from_any(
                    Type::Reference {
                        symbol: parameter_symbol,
                        generic_arguments: None,
                    },
                    source_id,
                )
            };
            receiver_arguments.push(StaticArgument::Evaluated {
                name: parameter.name,
                value: StaticExpression::Type { ty: reference_ty },
            });
        }

        receiver_arguments
    }

    /// Rewrite owner-scoped associated aliases in one type id with known substitutions.
    pub(crate) fn query_owner_symbol_for_member_symbol(
        &self,
        view: ModuleSymbolView<'_>,
        member_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let member_symbol = self
            .declaration_symbol_id(view, member_symbol)
            .unwrap_or_else(|| {
                self.canonical_symbol_id(view, member_symbol, CanonicalSymbolMode::FollowAliases)
            });

        self.with_module_symbols_or_local_for_artifact(
            view.compiler_context,
            view.module,
            view.profile,
            member_symbol.module_id,
            view.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |owner_module, owner_symbols| {
                // resolve the member entry and its scope owner
                let member_entry = owner_symbols.get_symbol(member_symbol.local_id);
                let scope = owner_symbols.get_scope_by_id(member_entry.scope.0);
                let owner_id = scope.owner_id?;
                let owner_entry = owner_symbols.get_symbol(owner_id);

                // keep only declaration owners that can hold member types
                if !matches!(
                    owner_entry.ty,
                    SymbolType::Class
                        | SymbolType::Struct
                        | SymbolType::Interface
                        | SymbolType::Enum
                        | SymbolType::Extension
                ) {
                    return None;
                }

                Some(
                    owner_id
                        .with_type(owner_entry.ty)
                        .into_global(owner_module.id),
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Classify one static symbol for associated projection paths.
    pub(crate) fn query_static_member_symbol_kind_for_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<StaticMemberSymbolKind>> {
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
                let primary_declaration = symbol_entry.primary_declaration?;

                match primary_declaration.local_id.ty {
                    NodeType::EnumField => Some(StaticMemberSymbolKind::EnumField),
                    NodeType::Member => {
                        let member_id = primary_declaration.local_id.into_typed::<Member>();
                        let member_kind = match view.tree.get(member_id) {
                            Member::AssociatedType { .. } => StaticMemberSymbolKind::AssociatedType,
                            Member::AssociatedConst { .. } => {
                                StaticMemberSymbolKind::AssociatedComptimeConst
                            }
                            _ => StaticMemberSymbolKind::Other,
                        };
                        Some(member_kind)
                    }
                    _ => {
                        let scope = view.symbols.get_scope_by_id(symbol_entry.scope.0);
                        let owner_scope_id = scope.owner_id?;
                        let owner_entry = view.symbols.get_symbol(owner_scope_id);
                        if !matches!(
                            owner_entry.ty,
                            SymbolType::Class
                                | SymbolType::Struct
                                | SymbolType::Interface
                                | SymbolType::Enum
                                | SymbolType::Extension
                        ) {
                            return None;
                        }

                        let owner_declaration = owner_entry.primary_declaration?;
                        if owner_declaration.local_id.ty != NodeType::Declaration {
                            return None;
                        }

                        let declaration_id = owner_declaration.local_id.into_typed::<Declaration>();
                        let declaration = view.tree.get(declaration_id);
                        let member_ids = declaration.member_ids()?;
                        for member_id in member_ids {
                            let member = view.tree.get(*member_id);
                            if member.symbol() != symbol.local_id {
                                continue;
                            }

                            let kind = match member {
                                Member::AssociatedType { .. } => {
                                    StaticMemberSymbolKind::AssociatedType
                                }
                                Member::AssociatedConst { .. } => {
                                    StaticMemberSymbolKind::AssociatedComptimeConst
                                }
                                _ => StaticMemberSymbolKind::Other,
                            };
                            return Some(kind);
                        }

                        None
                    }
                }
            },
        )
        .map_err(AnalyzeError::from)
    }
}
