use std::collections::{HashMap, HashSet};

use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, REWRITER_TAG_ASSOCIATED_ALIAS, TypeRewriteCache,
    TypeWalkContext, TypeWalkKey, rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_base::StringId;
use destack_dir::{
    Declaration, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Member, NodeTree, NodeType, StaticArgument, StaticKey, SymbolTable, SymbolType,
    Type, TypeRewriter, TypeRewriterOptions, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Rewrite associated alias references inside projected member types.
pub(super) struct AssociatedAliasProjectionRewriter<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The source node for diagnostics.
    source_id: LocalNodeIdAny,
    /// The owner symbol that defines the projected associated aliases.
    owner_symbol: GlobalSymbolId,
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
        module: &'a Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
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
            module,
            profile,
            source_id,
            owner_symbol,
            substitutions,
            tree,
            symbols,
            cache_key,
            options,
            cache: TypeRewriteCache::new(),
        }
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
            static_arguments,
        } = ty
        else {
            return None;
        };

        // normalize the reference symbol before member-kind checks
        let mut symbol = self.compiler.canonical_symbol_id(
            self.module,
            self.symbols,
            self.profile,
            *symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        symbol = self
            .compiler
            .declaration_symbol_id(self.module, self.symbols, self.profile, symbol)
            .unwrap_or(symbol);

        // keep references outside the owner declaration unchanged
        let owner_symbol = self.compiler.owner_symbol_for_member_symbol(
            self.module,
            self.profile,
            symbol,
            self.symbols,
        )?;
        if owner_symbol != self.owner_symbol {
            return None;
        }

        // keep non associated aliases unchanged
        if symbol.ty() != SymbolType::TypeAlias {
            return None;
        }

        // resolve the alias target for this member alias
        let alias_target_id = self.compiler.alias_target_type_id_for_symbol(
            self.module,
            self.profile,
            symbol,
            self.source_id,
            self.symbols,
            types,
        )?;

        // start from the caller substitutions
        let mut substitutions = self.substitutions.clone();

        // map explicit member arguments onto alias static parameters
        if let Some(member_arguments) = static_arguments.as_deref() {
            let member_substitutions = self.compiler.build_type_parameter_substitutions_for_symbol(
                self.module,
                self.profile,
                symbol,
                self.source_id,
                member_arguments,
                self.tree,
                self.symbols,
                types,
            );
            substitutions.extend(member_substitutions);
        }

        // apply substitutions into the alias target
        let mapped_alias_id = if substitutions.is_empty() {
            alias_target_id
        } else {
            let mut substitution_cache = HashMap::new();
            self.compiler.substitute_static_parameters(
                alias_target_id,
                &substitutions,
                types,
                &mut substitution_cache,
            )
        };

        // materialize static arguments after substitution
        let mut materialize_cache = TypeRewriteCache::new();
        let mapped_alias_id = self.compiler.materialize_static_arguments_in_type(
            self.module,
            self.profile,
            mapped_alias_id,
            self.tree,
            self.symbols,
            types,
            &mut materialize_cache,
        );

        // return one rewrite step and let the outer walker handle recursion and caching
        Some(mapped_alias_id)
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
    /// The merged substitutions for receiver, extension, and member parameters.
    pub(crate) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Primary semantic faults that block cascading missing-member diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingMemberDiagnosticBlocker {
    /// Receiver type already failed earlier analysis.
    ReceiverTypeError,
    /// Receiver declaration is missing required associated implementations.
    UnsatisfiedAssociatedContractRequirements,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when one declaration symbol has unimplemented associated requirements.
    pub(crate) fn symbol_has_unimplemented_associated_requirements(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<bool> {
        let Some(symbol) = self
            .declaration_symbol_id_at_stage(
                module,
                symbols,
                profile,
                symbol,
                AnalyzeDependencyStage::Declare,
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
            .with_module_types_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                types,
                AnalyzeDependencyStage::Declare,
                |_, owner_types| {
                    owner_types.symbol_has_unimplemented_associated_requirements(symbol)
                },
            )
            .map_err(AnalyzeError::from)?;

        Ok(has_missing_requirements)
    }

    /// Return true when one receiver type resolves to a symbol with unsatisfied associated requirements.
    pub(crate) fn receiver_type_has_unimplemented_associated_requirements(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<bool> {
        let Some((receiver_symbol, _, _)) = self.unwrap_type_symbol(types, receiver_ty_id) else {
            return Ok(false);
        };

        self.symbol_has_unimplemented_associated_requirements(
            module,
            profile,
            receiver_symbol,
            symbols,
            types,
        )
    }

    /// Return the primary semantic blocker for one missing-member diagnostic, when present.
    pub(crate) fn missing_member_diagnostic_blocker_for_receiver_type(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
        allow_associated_contract_blocker: bool,
    ) -> AnalyzeResult<Option<MissingMemberDiagnosticBlocker>> {
        if self.type_blocks_cascading_diagnostic(receiver_ty_id, types) {
            return Ok(Some(MissingMemberDiagnosticBlocker::ReceiverTypeError));
        }

        if allow_associated_contract_blocker
            && self.receiver_type_has_unimplemented_associated_requirements(
                module,
                profile,
                receiver_ty_id,
                symbols,
                types,
            )?
        {
            return Ok(Some(
                MissingMemberDiagnosticBlocker::UnsatisfiedAssociatedContractRequirements,
            ));
        }

        Ok(None)
    }

    /// Report one missing-member diagnostic unless a primary semantic blocker applies.
    pub(crate) fn report_missing_member_diagnostic_for_receiver_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        member_key: StaticKey,
        symbols: &SymbolTable,
        types: &TypeTable,
        allow_associated_contract_blocker: bool,
    ) -> AnalyzeResult<bool> {
        let blocker = self.missing_member_diagnostic_blocker_for_receiver_type(
            module,
            profile,
            receiver_ty_id,
            symbols,
            types,
            allow_associated_contract_blocker,
        )?;
        if blocker.is_some() {
            return Ok(false);
        }

        let error = AnalyzeError::MissingMember {
            node: expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            receiver_ty: receiver_ty_id.into_global(module.id),
            member_key,
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);

        Ok(true)
    }

    /// Emit one missing-member diagnostic unless a primary semantic blocker applies.
    pub(crate) fn emit_missing_member_diagnostic_for_receiver_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        member_key: StaticKey,
        symbols: &SymbolTable,
        types: &TypeTable,
        allow_associated_contract_blocker: bool,
    ) -> AnalyzeResult<()> {
        self.report_missing_member_diagnostic_for_receiver_type(
            module,
            profile,
            expression_id,
            receiver_ty_id,
            member_key,
            symbols,
            types,
            allow_associated_contract_blocker,
        )?;

        Ok(())
    }

    /// Collect contract associated type requirements for one contract symbol.
    pub(crate) fn collect_contract_associated_type_requirements(
        &self,
        module: &Module,
        profile: ProfileId,
        contract_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Vec<AssociatedTypeRequirement>> {
        // normalize contract references to declaration owners
        let Some(contract_symbol) = self
            .declaration_symbol_id_at_stage(
                module,
                symbols,
                profile,
                contract_symbol,
                AnalyzeDependencyStage::Declare,
            )
            .map_err(AnalyzeError::from)?
        else {
            return Ok(Vec::new());
        };

        // collect requirements with cycle protection
        let mut visited_contracts = HashSet::new();
        self.collect_contract_associated_type_requirements_inner(
            module,
            profile,
            contract_symbol,
            tree,
            symbols,
            &mut visited_contracts,
        )
    }

    /// Collect contract associated type requirements through declaration heritage.
    fn collect_contract_associated_type_requirements_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        contract_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        visited_contracts: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Vec<AssociatedTypeRequirement>> {
        // normalize contract declarations and break recursive cycles
        let Some(contract_symbol) =
            self.declaration_symbol_id(module, symbols, profile, contract_symbol)
        else {
            return Ok(Vec::new());
        };
        if !visited_contracts.insert(contract_symbol) {
            return Ok(Vec::new());
        }

        // collect local requirements and direct parent contracts
        let (local_requirements, parent_contracts) = self
            .with_module_tree_symbols_or_local_at_stage(
                module,
                profile,
                contract_symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let mut requirements = Vec::new();
                    let mut parents = Vec::new();

                    // resolve the contract declaration node
                    let symbol_entry = owner_symbols.get_symbol(contract_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return (requirements, parents);
                    };
                    if primary_declaration.local_id.ty != NodeType::Declaration {
                        return (requirements, parents);
                    }

                    let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                    let (members, heritage) = match owner_tree.get(declaration_id) {
                        Declaration::Interface {
                            members, heritage, ..
                        }
                        | Declaration::Class {
                            members, heritage, ..
                        }
                        | Declaration::Struct {
                            members, heritage, ..
                        } => (members, heritage),
                        _ => return (requirements, parents),
                    };

                    // collect local associated requirements
                    for member_id in members {
                        let Member::Type {
                            name,
                            static_parameters,
                            ty,
                            value,
                            symbol,
                            ..
                        } = owner_tree.get(*member_id)
                        else {
                            continue;
                        };

                        let parameter_symbols = static_parameters
                            .as_ref()
                            .map(|parameters| {
                                parameters
                                    .iter()
                                    .map(|parameter_id| {
                                        owner_tree
                                            .get(*parameter_id)
                                            .symbol()
                                            .into_global(owner_module.id)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        requirements.push(AssociatedTypeRequirement {
                            name: *name,
                            symbol: symbol.into_global(owner_module.id),
                            parameter_symbols,
                            bound_node: ty.map(|ty| ty.into_global_any(owner_module.id)),
                            requires_implementation: value.is_none(),
                        });
                    }

                    // collect parent contracts from extends and implements
                    let mut parent_types = Vec::new();
                    if let Some(extends_types) = heritage.extends_types.as_ref() {
                        parent_types.extend(extends_types.iter().copied());
                    }
                    if let Some(implements_types) = heritage.implements_types.as_ref() {
                        parent_types.extend(implements_types.iter().copied());
                    }
                    for parent_type_id in parent_types {
                        let Some(parent_symbol) = owner_tree.get(parent_type_id).target_symbol()
                        else {
                            continue;
                        };

                        let canonical_parent = self.canonical_symbol_id(
                            owner_module,
                            owner_symbols,
                            profile,
                            parent_symbol,
                            CanonicalSymbolMode::FollowAliases,
                        );
                        let Some(parent_symbol) = self.declaration_symbol_id(
                            owner_module,
                            owner_symbols,
                            profile,
                            canonical_parent,
                        ) else {
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
                module,
                profile,
                parent_contract,
                tree,
                symbols,
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
        module: &Module,
        profile: ProfileId,
        contract_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Vec<AssociatedComptimeRequirement>> {
        // normalize contract references to declaration owners
        let Some(contract_symbol) = self
            .declaration_symbol_id_at_stage(
                module,
                symbols,
                profile,
                contract_symbol,
                AnalyzeDependencyStage::Declare,
            )
            .map_err(AnalyzeError::from)?
        else {
            return Ok(Vec::new());
        };

        // collect requirements with cycle protection
        let mut visited_contracts = HashSet::new();
        self.collect_contract_associated_comptime_requirements_inner(
            module,
            profile,
            contract_symbol,
            tree,
            symbols,
            &mut visited_contracts,
        )
    }

    /// Collect contract associated comptime requirements through declaration heritage.
    fn collect_contract_associated_comptime_requirements_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        contract_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        visited_contracts: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Vec<AssociatedComptimeRequirement>> {
        // normalize contract declarations and break recursive cycles
        let Some(contract_symbol) =
            self.declaration_symbol_id(module, symbols, profile, contract_symbol)
        else {
            return Ok(Vec::new());
        };
        if !visited_contracts.insert(contract_symbol) {
            return Ok(Vec::new());
        }

        // collect local requirements and direct parent contracts
        let (local_requirements, parent_contracts) = self
            .with_module_tree_symbols_or_local_at_stage(
                module,
                profile,
                contract_symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let mut requirements = Vec::new();
                    let mut parents = Vec::new();

                    // resolve the contract declaration node
                    let symbol_entry = owner_symbols.get_symbol(contract_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return (requirements, parents);
                    };
                    if primary_declaration.local_id.ty != NodeType::Declaration {
                        return (requirements, parents);
                    }

                    let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                    let (members, heritage) = match owner_tree.get(declaration_id) {
                        Declaration::Interface {
                            members, heritage, ..
                        }
                        | Declaration::Class {
                            members, heritage, ..
                        }
                        | Declaration::Struct {
                            members, heritage, ..
                        } => (members, heritage),
                        _ => return (requirements, parents),
                    };

                    // collect local associated requirements
                    for member_id in members {
                        let Member::ComptimeConst {
                            name,
                            ty,
                            value,
                            symbol,
                            ..
                        } = owner_tree.get(*member_id)
                        else {
                            continue;
                        };

                        requirements.push(AssociatedComptimeRequirement {
                            name: *name,
                            symbol: symbol.into_global(owner_module.id),
                            type_node: ty.map(|ty| ty.into_global_any(owner_module.id)),
                            requires_implementation: value.is_none(),
                        });
                    }

                    // collect parent contracts from extends and implements
                    let mut parent_types = Vec::new();
                    if let Some(extends_types) = heritage.extends_types.as_ref() {
                        parent_types.extend(extends_types.iter().copied());
                    }
                    if let Some(implements_types) = heritage.implements_types.as_ref() {
                        parent_types.extend(implements_types.iter().copied());
                    }
                    for parent_type_id in parent_types {
                        let Some(parent_symbol) = owner_tree.get(parent_type_id).target_symbol()
                        else {
                            continue;
                        };

                        let canonical_parent = self.canonical_symbol_id(
                            owner_module,
                            owner_symbols,
                            profile,
                            parent_symbol,
                            CanonicalSymbolMode::FollowAliases,
                        );
                        let Some(parent_symbol) = self.declaration_symbol_id(
                            owner_module,
                            owner_symbols,
                            profile,
                            canonical_parent,
                        ) else {
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
                module,
                profile,
                parent_contract,
                tree,
                symbols,
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
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        preferred_kind: Option<StaticMemberSymbolKind>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<AssociatedProjectionSelection>> {
        // evaluate the receiver to a reference-like type
        let left_ty_id = self.resolve_declared_type_expression(
            module,
            profile,
            left,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        let left_ty = types.get_type(left_ty_id).clone();
        let receiver_reference = self
            .unwrap_type_symbol(types, left_ty_id)
            .map(|(symbol, static_arguments, _)| (symbol, static_arguments.unwrap_or_default()))
            .or_else(|| match left_ty {
                Type::Intersection { elements } | Type::Union { elements } => {
                    elements.iter().find_map(|element_id| {
                        self.unwrap_type_symbol(types, *element_id).map(
                            |(symbol, static_arguments, _)| {
                                (symbol, static_arguments.unwrap_or_default())
                            },
                        )
                    })
                }
                Type::This => {
                    self.owner_symbol_for_this_expression(module, profile, left, tree, symbols)
                }
                _ => None,
            });
        let mut receiver_reference = match receiver_reference {
            Some(reference) => Some(reference),
            None => self.associated_projection_receiver_from_expression(
                module, profile, left, tree, symbols, types,
            )?,
        };

        // preserve explicit receiver arguments from syntax when type evaluation dropped them
        if let Some((_, lookup_arguments)) = receiver_reference.as_ref()
            && lookup_arguments.is_empty()
            && let Some((syntax_symbol, syntax_arguments)) = self
                .associated_projection_receiver_from_expression(
                    module, profile, left, tree, symbols, types,
                )?
            && !syntax_arguments.is_empty()
        {
            receiver_reference = Some((syntax_symbol, syntax_arguments));
        }

        let Some((receiver_symbol, receiver_arguments)) = receiver_reference else {
            return Ok(None);
        };
        let mut projection_receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        projection_receiver_symbol = self.resolve_type_reference_symbol(
            module,
            profile,
            projection_receiver_symbol,
            tree,
            symbols,
        );
        projection_receiver_symbol = self
            .declaration_symbol_id(module, symbols, profile, projection_receiver_symbol)
            .unwrap_or(projection_receiver_symbol);
        let projection_receiver_arguments = receiver_arguments.clone();

        let mut lookup_symbol = projection_receiver_symbol;
        let mut lookup_arguments = receiver_arguments;

        // follow static parameter constraints for projected members
        if self.symbol_is_static_parameter(module, profile, lookup_symbol, symbols, types) {
            let constraint_ty_id = self.projection_static_parameter_constraint_type_for_traversal(
                module,
                profile,
                expression_id.into_any(),
                lookup_symbol,
                symbols,
                types,
            )?;
            if let Some(constraint_ty_id) = constraint_ty_id
                && let Type::Reference {
                    symbol,
                    static_arguments,
                } = types.get_type(constraint_ty_id)
            {
                lookup_symbol = *symbol;
                lookup_arguments = static_arguments.clone().unwrap_or_default();
            }
        }

        // project through alias references before static member lookup
        if let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
            module,
            profile,
            lookup_symbol,
            expression_id.into_any(),
            symbols,
            types,
        ) {
            let mapped_alias_target = if lookup_arguments.is_empty() {
                alias_target_id
            } else {
                let substitutions = self.build_type_parameter_substitutions_for_symbol(
                    module,
                    profile,
                    lookup_symbol,
                    expression_id.into_any(),
                    &lookup_arguments,
                    tree,
                    symbols,
                    types,
                );
                if substitutions.is_empty() {
                    alias_target_id
                } else {
                    let mut substitution_cache = HashMap::new();
                    self.substitute_static_parameters(
                        alias_target_id,
                        &substitutions,
                        types,
                        &mut substitution_cache,
                    )
                }
            };

            let mut materialize_cache = TypeRewriteCache::new();
            let mapped_alias_target = self.materialize_static_arguments_in_type(
                module,
                profile,
                mapped_alias_target,
                tree,
                symbols,
                types,
                &mut materialize_cache,
            );
            if let Some((alias_symbol, _, _)) = self.unwrap_type_symbol(types, mapped_alias_target)
            {
                lookup_symbol = alias_symbol;
            }
        }

        // resolve the projected member symbol on the normalized receiver symbol
        let projected_symbol = self
            .with_module_tree_symbols_or_local_at_stage(
                module,
                profile,
                lookup_symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    self.resolve_static_member_symbol_in_tables(
                        owner_module,
                        profile,
                        lookup_symbol,
                        member_key,
                        owner_tree,
                        owner_symbols,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;
        let Some(mut projected_symbol) = projected_symbol else {
            return Ok(None);
        };

        // prefer one member-kind class when the owner has ambiguous same-name members
        if let Some(preferred_kind) = preferred_kind {
            if let Some(preferred_symbol) = self.query_direct_member_symbol_for_key_and_kind(
                module,
                profile,
                lookup_symbol,
                member_key,
                preferred_kind,
                tree,
                symbols,
            )? {
                projected_symbol = preferred_symbol;
            }
        }

        let projected_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            projected_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let projected_symbol = self
            .declaration_symbol_id(module, symbols, profile, projected_symbol)
            .unwrap_or(projected_symbol);

        Ok(Some(AssociatedProjectionSelection {
            target_symbol: projected_symbol,
            receiver_symbol: projection_receiver_symbol,
            receiver_arguments: projection_receiver_arguments,
        }))
    }

    /// Build a projection receiver from one expression when type evaluation is unavailable.
    fn associated_projection_receiver_from_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
        let expression = tree.get(expression_id);
        let target_symbol = self
            .reference_symbol_for_expression(module, expression_id, profile, tree, symbols)
            .or_else(|| expression.target_symbol())
            .or_else(|| match expression {
                Expression::Instantiation { left, .. } => tree.get(*left).target_symbol(),
                _ => None,
            });
        let Some(target_symbol) = target_symbol else {
            return Ok(None);
        };
        let static_argument_nodes = expression.static_arguments();
        let static_arguments = self.evaluate_static_arguments(
            module,
            profile,
            static_argument_nodes,
            tree,
            symbols,
            types,
        )?;
        let static_arguments = static_arguments.unwrap_or_default();

        let mut target_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        target_symbol =
            self.resolve_type_reference_symbol(module, profile, target_symbol, tree, symbols);
        target_symbol = self
            .declaration_symbol_id(module, symbols, profile, target_symbol)
            .unwrap_or(target_symbol);

        Ok(Some((target_symbol, static_arguments)))
    }

    /// Query one owner member symbol for one key and one static-member kind.
    pub(crate) fn query_direct_member_symbol_for_key_and_kind(
        &self,
        module: &Module,
        profile: ProfileId,
        owner_symbol: GlobalSymbolId,
        member_key: StaticKey,
        preferred_kind: StaticMemberSymbolKind,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let Some(member_name) = member_key.name() else {
            return Ok(None);
        };

        let owner_symbol = self
            .declaration_symbol_id(module, symbols, profile, owner_symbol)
            .unwrap_or(owner_symbol);

        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            owner_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(owner_symbol.local_id);
                let mut declaration_ids = Vec::new();
                if let Some(primary_declaration) = symbol_entry.primary_declaration {
                    declaration_ids.push(primary_declaration);
                }
                if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref()
                {
                    declaration_ids.extend(secondary_declarations.iter().copied());
                }

                for declaration_id in declaration_ids {
                    if declaration_id.local_id.ty != NodeType::Declaration {
                        continue;
                    }
                    let declaration_id = declaration_id.local_id.into_typed::<Declaration>();
                    let declaration = owner_tree.get(declaration_id);
                    let members = match declaration {
                        Declaration::Class { members, .. }
                        | Declaration::Struct { members, .. }
                        | Declaration::Interface { members, .. }
                        | Declaration::Enum { members, .. }
                        | Declaration::Extension { members, .. } => members.as_slice(),
                        _ => continue,
                    };

                    for member_id in members {
                        let member = owner_tree.get(*member_id);
                        let (name, symbol, kind) = match member {
                            Member::Type { name, symbol, .. } => (
                                *name,
                                symbol.into_global(owner_module.id),
                                StaticMemberSymbolKind::AssociatedType,
                            ),
                            Member::ComptimeConst { name, symbol, .. } => (
                                *name,
                                symbol.into_global(owner_module.id),
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
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve one associated member symbol for a concrete receiver and static member key.
    pub(crate) fn resolve_associated_member_symbol_for_receiver(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_symbol: GlobalSymbolId,
        member_key: StaticKey,
        member_kind: StaticMemberSymbolKind,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let mut receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        receiver_symbol = self
            .declaration_symbol_id(module, symbols, profile, receiver_symbol)
            .unwrap_or(receiver_symbol);

        let Some(mut member_symbol) = self.resolve_static_member_symbol_in_tables(
            module,
            profile,
            receiver_symbol,
            member_key,
            tree,
            symbols,
        ) else {
            return Ok(None);
        };

        if let Some(preferred_symbol) = self.query_direct_member_symbol_for_key_and_kind(
            module,
            profile,
            receiver_symbol,
            member_key,
            member_kind,
            tree,
            symbols,
        )? {
            member_symbol = preferred_symbol;
        }

        if self.query_static_member_symbol_kind_for_symbol(
            module,
            profile,
            member_symbol,
            tree,
            symbols,
        )? != Some(member_kind)
        {
            return Ok(None);
        }

        let member_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            member_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let member_symbol = self
            .declaration_symbol_id(module, symbols, profile, member_symbol)
            .unwrap_or(member_symbol);

        Ok(Some(member_symbol))
    }

    /// Resolve a declaration owner symbol for one `this` receiver expression.
    pub(crate) fn owner_symbol_for_this_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<(GlobalSymbolId, Vec<StaticArgument>)> {
        // walk parent nodes until we find a declaration owner
        let mut current_id = expression_id.id;
        while let Some(parent) = tree.get_parent(current_id) {
            if parent.ty == NodeType::Declaration {
                let declaration_id = parent.into_typed::<Declaration>();
                let owner_symbol = match tree.get(declaration_id) {
                    Declaration::Class { descriptor, .. }
                    | Declaration::Struct { descriptor, .. }
                    | Declaration::Interface { descriptor, .. }
                    | Declaration::Enum { descriptor, .. } => {
                        Some(descriptor.symbol.into_global(module.id))
                    }
                    Declaration::Extension { target_symbol, .. } => *target_symbol,
                    _ => None,
                }?;

                let owner_symbol = self
                    .declaration_symbol_id(module, symbols, profile, owner_symbol)
                    .unwrap_or(owner_symbol);
                return Some((owner_symbol, Vec::new()));
            }

            current_id = parent.id;
        }

        None
    }

    /// Rewrite owner-scoped associated aliases in one type id with known substitutions.
    pub(crate) fn query_owner_symbol_for_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        self.with_module_symbols_or_local_at_stage(
            module,
            profile,
            member_symbol.module_id,
            symbols,
            AnalyzeDependencyStage::Declare,
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

    /// Resolve the owning declaration symbol for a member symbol.
    pub(crate) fn owner_symbol_for_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        self.query_owner_symbol_for_member_symbol(module, profile, member_symbol, symbols)
            .ok()
            .flatten()
    }

    /// Classify one static symbol for associated projection paths.
    pub(crate) fn query_static_member_symbol_kind_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<StaticMemberSymbolKind>> {
        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |_owner_module, owner_tree, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return None;
                };

                match primary_declaration.local_id.ty {
                    NodeType::EnumField => Some(StaticMemberSymbolKind::EnumField),
                    NodeType::Member => {
                        let member_id = primary_declaration.local_id.into_typed::<Member>();
                        let member_kind = match owner_tree.get(member_id) {
                            Member::Type { .. } => StaticMemberSymbolKind::AssociatedType,
                            Member::ComptimeConst { .. } => {
                                StaticMemberSymbolKind::AssociatedComptimeConst
                            }
                            _ => StaticMemberSymbolKind::Other,
                        };
                        Some(member_kind)
                    }
                    _ => {
                        let scope = owner_symbols.get_scope_by_id(symbol_entry.scope.0);
                        let owner_scope_id = scope.owner_id?;
                        let owner_entry = owner_symbols.get_symbol(owner_scope_id);
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
                        let declaration = owner_tree.get(declaration_id);
                        let member_ids = declaration.member_ids()?;
                        for member_id in member_ids {
                            let member = owner_tree.get(*member_id);
                            if member.symbol() != symbol.local_id {
                                continue;
                            }

                            let kind = match member {
                                Member::Type { .. } => StaticMemberSymbolKind::AssociatedType,
                                Member::ComptimeConst { .. } => {
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
