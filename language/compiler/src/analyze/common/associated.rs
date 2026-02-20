use std::collections::{HashMap, HashSet};

use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, REWRITER_TAG_ASSOCIATED_ALIAS, RelationMode,
    TypeRewriteCache, TypeWalkContext, TypeWalkKey, rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler};
use destack_base::StringId;
use destack_dir::{
    Declaration, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, Heritage, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, Member, NodeTree, NodeType, NormalizationMode, StaticArgument,
    StaticExpression, StaticKey, SymbolTable, SymbolType, Type, TypeLiteral, TypeRewriter,
    TypeRewriterOptions, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Rewrite associated alias references inside projected member types.
struct AssociatedAliasProjectionRewriter<'a> {
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
    fn new(
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

        // keep references outside the owner declaration unchanged
        let owner_symbol = self.compiler.owner_symbol_for_member_symbol(
            self.module,
            self.profile,
            *symbol,
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
            *symbol,
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
                *symbol,
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

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
        let Some(contract_symbol) =
            self.declaration_symbol_id(module, symbols, profile, contract_symbol)
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
            .with_module_tree_symbols_or_local_for_stage(
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
        let Some(contract_symbol) =
            self.declaration_symbol_id(module, symbols, profile, contract_symbol)
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
            .with_module_tree_symbols_or_local_for_stage(
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
        projection_receiver_symbol = self
            .declaration_symbol_id(module, symbols, profile, projection_receiver_symbol)
            .unwrap_or(projection_receiver_symbol);
        let projection_receiver_arguments = receiver_arguments.clone();

        let mut lookup_symbol = projection_receiver_symbol;
        let mut lookup_arguments = receiver_arguments;

        // follow static parameter constraints for projected members
        if self.symbol_is_static_parameter(module, profile, lookup_symbol, symbols, types)
            && let Some(constraint_ty_id) = self.static_parameter_constraint_type(
                module,
                profile,
                lookup_symbol,
                expression_id.into_any(),
                symbols,
                types,
            )
            && let Type::Reference {
                symbol,
                static_arguments,
            } = types.get_type(constraint_ty_id)
        {
            lookup_symbol = *symbol;
            lookup_arguments = static_arguments.clone().unwrap_or_default();
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
            .with_module_tree_symbols_or_local_for_stage(
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

        self.with_module_tree_symbols_or_local_for_stage(
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
    pub(crate) fn rewrite_associated_aliases_for_owner(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // skip when no substitutions are available
        if substitutions.is_empty() {
            return type_id;
        }

        // rewrite associated aliases for the owner in one pass
        let mut rewriter = AssociatedAliasProjectionRewriter::new(
            self,
            module,
            profile,
            source_id,
            owner_symbol,
            substitutions,
            tree,
            symbols,
        );

        rewriter.rewrite_type_id(types, type_id)
    }

    /// Resolve receiver substitutions for an associated projection owner.
    fn associated_projection_receiver_substitutions_for_owner_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        owner_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        let canonical_receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let canonical_receiver_symbol = self
            .declaration_symbol_id(module, symbols, profile, canonical_receiver_symbol)
            .unwrap_or(canonical_receiver_symbol);
        let owner_symbol = self
            .declaration_symbol_id(module, symbols, profile, owner_symbol)
            .unwrap_or(owner_symbol);
        let receiver_substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            canonical_receiver_symbol,
            source_id,
            receiver_arguments,
            tree,
            symbols,
            types,
        );

        let mut visited_symbols = HashSet::new();
        self.associated_projection_receiver_substitutions_inner(
            module,
            profile,
            source_id,
            canonical_receiver_symbol,
            owner_symbol,
            receiver_substitutions,
            tree,
            symbols,
            types,
            &mut visited_symbols,
        )
    }

    /// Resolve receiver substitutions for associated projections along heritage edges.
    fn associated_projection_receiver_substitutions_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        current_symbol: GlobalSymbolId,
        owner_symbol: GlobalSymbolId,
        current_substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
        if self.symbol_is_static_parameter(module, profile, current_symbol, symbols, types)
            && let Some(mut constraint_type_id) = self.static_parameter_constraint_type(
                module,
                profile,
                current_symbol,
                source_id,
                symbols,
                types,
            )
        {
            if !current_substitutions.is_empty() {
                let mut substitution_cache = HashMap::new();
                constraint_type_id = self.substitute_static_parameters(
                    constraint_type_id,
                    &current_substitutions,
                    types,
                    &mut substitution_cache,
                );
            }

            let mut materialize_cache = TypeRewriteCache::new();
            constraint_type_id = self.materialize_static_arguments_in_type(
                module,
                profile,
                constraint_type_id,
                tree,
                symbols,
                types,
                &mut materialize_cache,
            );

            if let Some((mut next_symbol, next_arguments, _)) =
                self.unwrap_type_symbol(types, constraint_type_id)
            {
                next_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    next_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                next_symbol = self
                    .declaration_symbol_id(module, symbols, profile, next_symbol)
                    .unwrap_or(next_symbol);

                let next_substitutions = self.build_type_parameter_substitutions_for_symbol(
                    module,
                    profile,
                    next_symbol,
                    source_id,
                    next_arguments.as_deref().unwrap_or_default(),
                    tree,
                    symbols,
                    types,
                );
                if let Some(substitutions) = self
                    .associated_projection_receiver_substitutions_inner(
                        module,
                        profile,
                        source_id,
                        next_symbol,
                        owner_symbol,
                        next_substitutions,
                        tree,
                        symbols,
                        types,
                        visited_symbols,
                    )?
                {
                    return Ok(Some(substitutions));
                }
            }
        }

        // collect direct heritage expressions for this symbol
        let heritage_expressions = self.heritage_expressions_for_associated_projection(
            module,
            profile,
            current_symbol,
            tree,
            symbols,
        )?;
        for heritage_expression_id in heritage_expressions {
            // resolve the heritage target and applied arguments
            let resolved_heritage = self
                .with_module_tree_symbols_or_local_for_stage(
                    module,
                    profile,
                    heritage_expression_id.module_id,
                    tree,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |owner_module,
                     owner_tree,
                     owner_symbols|
                     -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
                        let expression_id = heritage_expression_id.local_id;
                        let expression = owner_tree.get(expression_id);
                        let expression_has_static_arguments = expression
                            .static_arguments()
                            .is_some_and(|arguments| !arguments.is_empty());
                        let Some(target_symbol) = expression.target_symbol() else {
                            return Ok(None);
                        };
                        let target_symbol = self.canonical_symbol_id(
                            owner_module,
                            owner_symbols,
                            profile,
                            target_symbol,
                            CanonicalSymbolMode::FollowAliases,
                        );

                        let expression_global_id = expression_id.into_global_any(owner_module.id);
                        let mut heritage_type_id = if let Some(type_id) = types
                            .get_inferred_type_id(expression_global_id)
                            .or_else(|| types.get_declared_type_id(expression_global_id))
                        {
                            type_id
                        } else {
                            self.resolve_declared_type_expression(
                                owner_module,
                                profile,
                                expression_id,
                                owner_tree,
                                owner_symbols,
                                types,
                                true,
                                true,
                            )?
                        };

                        if !current_substitutions.is_empty() {
                            let mut substitution_cache = HashMap::new();
                            heritage_type_id = self.substitute_static_parameters(
                                heritage_type_id,
                                &current_substitutions,
                                types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        heritage_type_id = self.materialize_static_arguments_in_type(
                            owner_module,
                            profile,
                            heritage_type_id,
                            owner_tree,
                            owner_symbols,
                            types,
                            &mut materialize_cache,
                        );

                        // refresh cached heritage references when explicit static arguments were dropped
                        if expression_has_static_arguments
                            && let Some((_, resolved_arguments, _)) =
                                self.unwrap_type_symbol(types, heritage_type_id)
                            && resolved_arguments
                                .as_ref()
                                .is_none_or(|arguments| arguments.is_empty())
                        {
                            heritage_type_id = self.resolve_declared_type_expression(
                                owner_module,
                                profile,
                                expression_id,
                                owner_tree,
                                owner_symbols,
                                types,
                                true,
                                true,
                            )?;

                            if !current_substitutions.is_empty() {
                                let mut substitution_cache = HashMap::new();
                                heritage_type_id = self.substitute_static_parameters(
                                    heritage_type_id,
                                    &current_substitutions,
                                    types,
                                    &mut substitution_cache,
                                );
                            }

                            heritage_type_id = self.materialize_static_arguments_in_type(
                                owner_module,
                                profile,
                                heritage_type_id,
                                owner_tree,
                                owner_symbols,
                                types,
                                &mut materialize_cache,
                            );
                        }

                        if let Some((resolved_symbol, resolved_arguments, _)) =
                            self.unwrap_type_symbol(types, heritage_type_id)
                        {
                            let mut resolved_arguments = resolved_arguments.unwrap_or_default();
                            if resolved_arguments.is_empty() && expression_has_static_arguments {
                                let evaluated_arguments = self.evaluate_static_arguments(
                                    owner_module,
                                    profile,
                                    expression.static_arguments(),
                                    owner_tree,
                                    owner_symbols,
                                    types,
                                )?;
                                let evaluated_arguments = evaluated_arguments.unwrap_or_default();
                                let options =
                                    self.analyze_context_options_for_module(owner_module.id);
                                let resolved_static_arguments = self
                                    .resolve_type_reference_static_arguments_for_symbol(
                                        owner_module,
                                        profile,
                                        source_id,
                                        target_symbol,
                                        Some(evaluated_arguments.as_slice()),
                                        true,
                                        &options,
                                        owner_tree,
                                        owner_symbols,
                                        types,
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
                module,
                profile,
                next_symbol,
                source_id,
                &next_arguments,
                tree,
                symbols,
                types,
            );
            if let Some(substitutions) = self.associated_projection_receiver_substitutions_inner(
                module,
                profile,
                source_id,
                next_symbol,
                owner_symbol,
                next_substitutions,
                tree,
                symbols,
                types,
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
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Vec<GlobalNodeId<Expression>>> {
        self.with_module_tree_symbols_or_local_for_stage(
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return Vec::new();
                };
                if primary_declaration.local_id.ty != NodeType::Declaration {
                    return Vec::new();
                }

                let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                let heritage = match owner_tree.get(declaration_id) {
                    Declaration::Class { heritage, .. }
                    | Declaration::Struct { heritage, .. }
                    | Declaration::Enum { heritage, .. }
                    | Declaration::Interface { heritage, .. } => Some(heritage),
                    _ => None,
                };
                let Some(Heritage {
                    extends_types,
                    implements_types,
                    ..
                }) = heritage
                else {
                    return Vec::new();
                };

                let mut expressions = Vec::new();
                if let Some(extends_types) = extends_types.as_ref() {
                    for expression_id in extends_types {
                        expressions.push((*expression_id).into_global(owner_module.id));
                    }
                }

                if let Some(implements_types) = implements_types.as_ref() {
                    for expression_id in implements_types {
                        expressions.push((*expression_id).into_global(owner_module.id));
                    }
                }

                expressions
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Build interface substitutions for one interface owner on a receiver type.
    pub(crate) fn interface_substitutions_for_owner_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        interface_symbol: GlobalSymbolId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        if interface_symbol.ty() != SymbolType::Interface {
            return Ok(None);
        }

        // normalize the receiver symbol for extension matching
        let canonical_receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // resolve substitutions from direct declaration implements clauses first
        if let Some(substitutions) = self.interface_substitutions_for_receiver_declaration(
            module,
            profile,
            source_id,
            canonical_receiver_symbol,
            receiver_arguments,
            interface_symbol,
            options,
            tree,
            symbols,
            types,
        )? {
            return Ok(Some(substitutions));
        }

        // collect visible extensions for the receiver target
        let mut extension_symbols = self.visible_extension_symbols_for_target(
            module,
            profile,
            symbols,
            types,
            canonical_receiver_symbol,
        )?;

        // include directly declared extensions from the receiver module
        let declared_extension_symbols = self
            .with_module_tree_symbols_or_local_for_stage(
                module,
                profile,
                canonical_receiver_symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let mut declared = Vec::new();
                    for declaration_id in owner_tree.iter_node_ids_of_type::<Declaration>() {
                        let Declaration::Extension {
                            descriptor,
                            target_symbol: Some(extension_target),
                            ..
                        } = owner_tree.get(declaration_id)
                        else {
                            continue;
                        };
                        let canonical_target = self.canonical_symbol_id(
                            owner_module,
                            owner_symbols,
                            profile,
                            *extension_target,
                            CanonicalSymbolMode::FollowAliases,
                        );
                        if canonical_target != canonical_receiver_symbol {
                            continue;
                        }

                        declared.push(descriptor.symbol.into_global(owner_module.id));
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
                .with_module_tree_symbols_or_local_for_stage(
                    module,
                    profile,
                    extension_symbol.module_id,
                    tree,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |owner_module,
                     owner_tree,
                     owner_symbols|
                     -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
                        let symbol_entry = owner_symbols.get_symbol(extension_symbol.local_id);
                        let Some(primary_declaration) = symbol_entry.primary_declaration else {
                            return Ok(None);
                        };
                        if primary_declaration.local_id.ty != NodeType::Declaration {
                            return Ok(None);
                        }
                        let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                        let Declaration::Extension { heritage, .. } =
                            owner_tree.get(declaration_id)
                        else {
                            return Ok(None);
                        };
                        let Some(implements_types) = heritage.implements_types.as_ref() else {
                            return Ok(None);
                        };

                        let receiver_substitutions =
                            self.build_type_parameter_substitutions_for_symbol(
                                owner_module,
                                profile,
                                extension_symbol,
                                source_id,
                                receiver_arguments,
                                owner_tree,
                                owner_symbols,
                                types,
                            );

                        self.interface_substitutions_for_implements_types(
                            owner_module,
                            profile,
                            source_id,
                            interface_symbol,
                            implements_types,
                            &receiver_substitutions,
                            options,
                            owner_tree,
                            owner_symbols,
                            types,
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
    fn interface_substitutions_for_receiver_declaration(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        interface_symbol: GlobalSymbolId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        self.with_module_tree_symbols_or_local_for_stage(
            module,
            profile,
            receiver_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                // resolve receiver declaration and heritage
                let symbol_entry = owner_symbols.get_symbol(receiver_symbol.local_id);
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return Ok(None);
                };
                if primary_declaration.local_id.ty != NodeType::Declaration {
                    return Ok(None);
                }
                let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                let implements_types = match owner_tree.get(declaration_id) {
                    Declaration::Class { heritage, .. }
                    | Declaration::Struct { heritage, .. }
                    | Declaration::Enum { heritage, .. } => heritage.implements_types.as_deref(),
                    _ => None,
                };
                let Some(implements_types) = implements_types else {
                    return Ok(None);
                };

                // resolve receiver substitutions from projection arguments
                let receiver_substitutions = self.build_type_parameter_substitutions_for_symbol(
                    owner_module,
                    profile,
                    receiver_symbol,
                    source_id,
                    receiver_arguments,
                    owner_tree,
                    owner_symbols,
                    types,
                );

                self.interface_substitutions_for_implements_types(
                    owner_module,
                    profile,
                    source_id,
                    interface_symbol,
                    implements_types,
                    &receiver_substitutions,
                    options,
                    owner_tree,
                    owner_symbols,
                    types,
                )
            },
        )
        .map_err(AnalyzeError::from)?
    }

    /// Resolve interface substitutions from a list of implements expressions.
    fn interface_substitutions_for_implements_types(
        &self,
        owner_module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        interface_symbol: GlobalSymbolId,
        implements_types: &[LocalNodeId<Expression>],
        receiver_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        options: &AnalyzeOptions,
        owner_tree: &NodeTree,
        owner_symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        let interface_symbol = self.canonical_symbol_id(
            owner_module,
            owner_symbols,
            profile,
            interface_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let interface_symbol = self
            .declaration_symbol_id(owner_module, owner_symbols, profile, interface_symbol)
            .unwrap_or(interface_symbol);

        for interface_expression_id in implements_types {
            let Some(target_symbol) = owner_tree.get(*interface_expression_id).target_symbol()
            else {
                continue;
            };
            let mut canonical_target = self.canonical_symbol_id(
                owner_module,
                owner_symbols,
                profile,
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            canonical_target = self
                .declaration_symbol_id(owner_module, owner_symbols, profile, canonical_target)
                .unwrap_or(canonical_target);

            // resolve interface arguments from heritage expressions
            let static_argument_nodes = owner_tree.get(*interface_expression_id).static_arguments();
            let evaluated_static_arguments = self.evaluate_static_arguments(
                owner_module,
                profile,
                static_argument_nodes,
                owner_tree,
                owner_symbols,
                types,
            )?;
            let evaluated_static_arguments = evaluated_static_arguments.unwrap_or_default();
            let resolved_static_arguments = self
                .resolve_type_reference_static_arguments_for_symbol(
                    owner_module,
                    profile,
                    source_id,
                    canonical_target,
                    Some(evaluated_static_arguments.as_slice()),
                    true,
                    options,
                    owner_tree,
                    owner_symbols,
                    types,
                )?;
            let interface_arguments =
                resolved_static_arguments.unwrap_or(evaluated_static_arguments);
            let mut substitutions = self.build_type_parameter_substitutions_for_symbol(
                owner_module,
                profile,
                canonical_target,
                source_id,
                &interface_arguments,
                owner_tree,
                owner_symbols,
                types,
            );

            // apply receiver substitutions to inherited interface arguments
            if !receiver_substitutions.is_empty() {
                let mut cache = HashMap::new();
                for ty_id in substitutions.values_mut() {
                    let mapped = self.substitute_static_parameters(
                        *ty_id,
                        receiver_substitutions,
                        types,
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
                        owner_module,
                        profile,
                        source_id,
                        canonical_target,
                        interface_symbol,
                        substitutions,
                        owner_tree,
                        owner_symbols,
                        types,
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
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        explicit_member_arguments: Option<&[StaticArgument]>,
        member_ty: Option<&Type>,
        options: &AnalyzeOptions,
        static_eval_visited_symbols: Option<&HashSet<GlobalSymbolId>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ProjectionEnvironment> {
        let mut substitutions = HashMap::new();
        let owner_symbol =
            self.query_owner_symbol_for_member_symbol(module, profile, target_symbol, symbols)?;
        let canonical_receiver_symbol = receiver_symbol.map(|receiver_symbol| {
            let canonical_receiver_symbol = self.canonical_symbol_id(
                module,
                symbols,
                profile,
                receiver_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            self.declaration_symbol_id(module, symbols, profile, canonical_receiver_symbol)
                .unwrap_or(canonical_receiver_symbol)
        });
        let owner_symbol = owner_symbol.map(|owner_symbol| {
            self.declaration_symbol_id(module, symbols, profile, owner_symbol)
                .unwrap_or(owner_symbol)
        });

        // map receiver substitutions onto owner parameters
        let mut receiver_owner_substitutions = None;
        if let Some(owner_symbol) = owner_symbol
            && let Some(receiver_symbol) = canonical_receiver_symbol
        {
            if owner_symbol.ty() == SymbolType::Interface {
                receiver_owner_substitutions = self.interface_substitutions_for_owner_symbol(
                    module,
                    profile,
                    source_id,
                    receiver_symbol,
                    receiver_arguments,
                    owner_symbol,
                    options,
                    tree,
                    symbols,
                    types,
                )?;

                // fall back to general receiver traversal for constrained/interface projections
                if receiver_owner_substitutions.is_none() {
                    receiver_owner_substitutions = self
                        .associated_projection_receiver_substitutions_for_owner_symbol(
                            module,
                            profile,
                            source_id,
                            receiver_symbol,
                            receiver_arguments,
                            owner_symbol,
                            tree,
                            symbols,
                            types,
                        )?;
                }
            } else {
                receiver_owner_substitutions = self
                    .associated_projection_receiver_substitutions_for_owner_symbol(
                        module,
                        profile,
                        source_id,
                        receiver_symbol,
                        receiver_arguments,
                        owner_symbol,
                        tree,
                        symbols,
                        types,
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
        if let Some(receiver_symbol) = canonical_receiver_symbol
            && let Some(owner_symbol) = owner_symbol
            && let Some(owner_comptime_substitutions) = self
                .owner_comptime_substitutions_for_receiver(
                    module,
                    profile,
                    source_id,
                    receiver_symbol,
                    owner_symbol,
                    tree,
                    symbols,
                    static_eval_visited_symbols,
                    receiver_owner_substitutions.as_ref(),
                    types,
                )?
        {
            substitutions.extend(owner_comptime_substitutions);
        }

        // map extension substitutions when the member is extension-owned
        if let Ok(Some(extension_context)) = self.resolve_extension_member_context(
            module,
            profile,
            source_id,
            target_symbol,
            receiver_arguments,
            options,
            tree,
            symbols,
            types,
        ) {
            substitutions.extend(extension_context.substitutions);
        }

        // resolve member static arguments under the projection context
        let resolved_member_arguments = self
            .resolve_type_reference_static_arguments_for_symbol_with_bound_substitutions(
                module,
                profile,
                source_id,
                target_symbol,
                explicit_member_arguments,
                true,
                options,
                Some(&substitutions),
                tree,
                symbols,
                types,
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
                module,
                profile,
                member_argument_symbol,
                source_id,
                member_arguments,
                tree,
                symbols,
                types,
            );
            substitutions.extend(member_substitutions);
        }

        Ok(ProjectionEnvironment {
            owner_symbol,
            substitutions,
        })
    }

    /// Build owner associated comptime substitutions for one projected receiver.
    fn owner_comptime_substitutions_for_receiver(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        owner_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        static_eval_visited_symbols: Option<&HashSet<GlobalSymbolId>>,
        receiver_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        if !matches!(
            owner_symbol.ty(),
            SymbolType::Interface | SymbolType::Class | SymbolType::Struct
        ) {
            return Ok(None);
        }
        let owner_symbol = self
            .declaration_symbol_id(module, symbols, profile, owner_symbol)
            .unwrap_or(owner_symbol);

        // normalize the receiver symbol for lookup
        let canonical_receiver_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            receiver_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let canonical_receiver_symbol = self
            .declaration_symbol_id(module, symbols, profile, canonical_receiver_symbol)
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
            .with_module_tree_symbols_or_local_for_stage(
                module,
                profile,
                owner_symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let mut members = Vec::new();
                    let symbol_entry = owner_symbols.get_symbol(owner_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return members;
                    };
                    if primary_declaration.local_id.ty != NodeType::Declaration {
                        return members;
                    }

                    let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
                    let declaration = owner_tree.get(declaration_id);
                    let Some(member_ids) = declaration.member_ids() else {
                        return members;
                    };

                    for member_id in member_ids {
                        let Member::ComptimeConst {
                            name,
                            symbol,
                            value,
                            ..
                        } = owner_tree.get(*member_id)
                        else {
                            continue;
                        };
                        members.push((
                            StaticKey::Name(*name),
                            symbol.into_global(owner_module.id),
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
            let resolved_member_symbol = self
                .with_module_tree_symbols_or_local_for_stage(
                    module,
                    profile,
                    canonical_receiver_symbol.module_id,
                    tree,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |receiver_module, receiver_tree, receiver_symbols| {
                        self.resolve_static_member_symbol_in_tables(
                            receiver_module,
                            profile,
                            canonical_receiver_symbol,
                            member_key,
                            receiver_tree,
                            receiver_symbols,
                        )
                    },
                )
                .map_err(AnalyzeError::from)?
                .unwrap_or(owner_member_symbol);

            // avoid recursive projection substitution for members already under static evaluation
            let is_member_in_current_static_evaluation =
                static_eval_visited_symbols.is_some_and(|visited_symbols| {
                    visited_symbols.contains(&resolved_member_symbol)
                        || visited_symbols.contains(&owner_member_symbol)
                });
            if is_member_in_current_static_evaluation {
                continue;
            }

            let should_skip_interface_owner_default = owner_symbol.ty() == SymbolType::Interface
                && resolved_member_symbol == owner_member_symbol
                && requires_implementation;
            if should_skip_interface_owner_default {
                let should_report_missing =
                    requires_implementation && canonical_receiver_symbol != owner_symbol;
                if should_report_missing {
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: source_id
                            .into_global(module.id)
                            .into_anchored(Some(profile)),
                        message: "missing associated comptime implementation".to_string(),
                    });
                }
                continue;
            }

            let mut visited_symbols = static_eval_visited_symbols.cloned().unwrap_or_default();
            let Some(value) = self.static_expression_from_constant_reference_specialized(
                module,
                profile,
                resolved_member_symbol,
                tree,
                symbols,
                types,
                receiver_substitutions,
                &mut visited_symbols,
            )?
            else {
                let should_report_missing =
                    requires_implementation && canonical_receiver_symbol != owner_symbol;
                if should_report_missing {
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: source_id
                            .into_global(module.id)
                            .into_anchored(Some(profile)),
                        message: "missing associated comptime implementation".to_string(),
                    });
                }
                continue;
            };

            let Some(type_id) =
                self.static_expression_type_id_for_substitution(source_id, &value, types)
            else {
                continue;
            };
            substitutions.insert(owner_member_symbol, type_id);
        }

        if substitutions.is_empty() {
            return Ok(None);
        }

        Ok(Some(substitutions))
    }

    /// Convert a static expression to a local type id for substitution.
    fn static_expression_type_id_for_substitution(
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

    /// Import one alias target without requiring pre-materialized static value arguments.
    fn relaxed_alias_target_type_id_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let typed_symbol = self
            .with_module_tree_symbols_or_local_for_stage(
                module,
                profile,
                symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |_, _, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                    if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                        return None;
                    }

                    Some(GlobalSymbolId::new(
                        symbol.module_id,
                        symbol.local_id.with_type(symbol_entry.ty),
                    ))
                },
            )
            .map_err(AnalyzeError::from)
            .ok()??;

        if typed_symbol.module_id == module.id {
            types.record_normalization_symbol_dependency(typed_symbol);
            return types
                .get_alias_target_type_id(typed_symbol)
                .or_else(|| types.get_alias_target_type_id(symbol));
        }

        self.with_module_tree_symbols_or_local_for_stage(
            module,
            profile,
            typed_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, _owner_tree, _owner_symbols| {
                let owner_types = owner_module.dir(profile).types.read();
                let remote_target_id = owner_types.get_alias_target_type_id(typed_symbol)?;
                let remote_target_ty = owner_types.get_type(remote_target_id);
                let local_target_id = self.import_type_from_remote_for_node(
                    source_id,
                    remote_target_ty,
                    &owner_types,
                    typed_symbol,
                    types,
                );
                types.record_normalization_symbol_dependency(typed_symbol);
                types.set_alias_target_type_id(typed_symbol, local_target_id);
                Some(local_target_id)
            },
        )
        .map_err(AnalyzeError::from)
        .ok()?
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

    /// Return the projected substitution symbol referenced by one expression.
    fn projection_substitution_symbol_from_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        owner_tree: &NodeTree,
        owner_symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let mut expression_id = expression_id;

        // peel wrappers used around projection arguments
        loop {
            match owner_tree.get(expression_id) {
                Expression::Parenthesized { expression } => {
                    expression_id = *expression;
                }
                Expression::TypeUnary {
                    operator: destack_dir::TypeUnaryOperator::AsComptime,
                    right,
                } => {
                    expression_id = *right;
                }
                _ => break,
            }
        }

        if let Some(target_symbol) = owner_tree.get(expression_id).target_symbol() {
            return Some(target_symbol);
        }

        if let Expression::Member { left, name, .. } = owner_tree.get(expression_id)
            && matches!(owner_tree.get(*left), Expression::This)
            && let Some((owner_symbol, _)) = self.owner_symbol_for_this_expression(
                module,
                profile,
                *left,
                owner_tree,
                owner_symbols,
            )
            && let Some(member_symbol) = self.resolve_static_member_symbol_in_tables(
                module,
                profile,
                owner_symbol,
                StaticKey::Name(*name),
                owner_tree,
                owner_symbols,
            )
        {
            return Some(member_symbol);
        }

        None
    }

    /// Apply associated projection substitutions by alias expression shape.
    fn apply_projection_substitutions_from_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        local_type_id: LocalTypeId,
        owner_tree: &NodeTree,
        owner_symbols: &SymbolTable,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // map unevaluated direct references when projection substitutions become available
        if matches!(types.get_type(local_type_id), Type::Unevaluated(_))
            && let Some(target_symbol) = self.projection_substitution_symbol_from_expression(
                module,
                profile,
                expression_id,
                owner_tree,
                owner_symbols,
            )
        {
            if let Some(substitution) =
                self.projection_substitution_type_for_symbol(target_symbol, substitutions)
            {
                return self.normalized_projection_substitution_type(substitution, types);
            }

            let mut visited_symbols = HashSet::new();
            if let Ok(Some(value)) = self.static_expression_from_constant_reference_specialized(
                module,
                profile,
                target_symbol,
                owner_tree,
                owner_symbols,
                types,
                substitutions,
                &mut visited_symbols,
            ) && let Some(value_type_id) = self.static_expression_type_id_for_substitution(
                expression_id.into_any(),
                &value,
                types,
            ) {
                let mut mapped_value_type_id = value_type_id;
                if !substitutions.is_empty() {
                    let mut substitution_cache = HashMap::new();
                    mapped_value_type_id = self.substitute_static_parameters(
                        mapped_value_type_id,
                        substitutions,
                        types,
                        &mut substitution_cache,
                    );
                }

                let mut materialize_cache = TypeRewriteCache::new();
                mapped_value_type_id = self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    mapped_value_type_id,
                    owner_tree,
                    owner_symbols,
                    types,
                    &mut materialize_cache,
                );
                mapped_value_type_id = self.normalize_type_with_relation(
                    module,
                    profile,
                    mapped_value_type_id,
                    owner_symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::STATIC_EVAL,
                );

                return mapped_value_type_id;
            }
        }

        // recurse through nested index expressions and set count inferred types from substitutions
        match owner_tree.get(expression_id).clone() {
            Expression::TypeIndex { left, index } => {
                let array_parts = match types.get_type(local_type_id) {
                    Type::ArraySized {
                        element,
                        count,
                        is_readonly,
                    } => Some((*element, *count, *is_readonly)),
                    _ => None,
                };
                let Some((element, count, is_readonly)) = array_parts else {
                    return local_type_id;
                };
                let mut substitution_cache = HashMap::new();
                let mut mapped_count = self.substitute_static_parameters(
                    count,
                    substitutions,
                    types,
                    &mut substitution_cache,
                );

                if let Some(target_symbol) = self.projection_substitution_symbol_from_expression(
                    module,
                    profile,
                    index,
                    owner_tree,
                    owner_symbols,
                ) && let Some(substitution) =
                    self.projection_substitution_type_for_symbol(target_symbol, substitutions)
                {
                    mapped_count =
                        self.normalized_projection_substitution_type(substitution, types);
                }

                // map direct count references through projection substitutions
                if let Type::Reference { symbol, .. } = types.get_type(mapped_count).clone()
                    && let Some(substitution) =
                        self.projection_substitution_type_for_symbol(symbol, substitutions)
                {
                    mapped_count =
                        self.normalized_projection_substitution_type(substitution, types);
                }

                // materialize remaining comptime references using projection substitutions
                if let Type::Reference { symbol, .. } = types.get_type(mapped_count).clone() {
                    let mut visited_symbols = HashSet::new();
                    if let Ok(Some(value)) = self
                        .static_expression_from_constant_reference_specialized(
                            module,
                            profile,
                            symbol,
                            owner_tree,
                            owner_symbols,
                            types,
                            substitutions,
                            &mut visited_symbols,
                        )
                        && let Some(value_type_id) = self
                            .static_expression_type_id_for_substitution(
                                expression_id.into_any(),
                                &value,
                                types,
                            )
                    {
                        mapped_count = value_type_id;
                        if !substitutions.is_empty() {
                            let mut substitution_cache = HashMap::new();
                            mapped_count = self.substitute_static_parameters(
                                mapped_count,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        mapped_count = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            mapped_count,
                            owner_tree,
                            owner_symbols,
                            types,
                            &mut materialize_cache,
                        );
                        mapped_count = self.normalize_type_with_relation(
                            module,
                            profile,
                            mapped_count,
                            owner_symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::STATIC_EVAL,
                        );
                    }
                }

                let mapped_element = self.apply_projection_substitutions_from_expression(
                    module,
                    profile,
                    left,
                    element,
                    owner_tree,
                    owner_symbols,
                    substitutions,
                    types,
                );
                if mapped_element == element && mapped_count == count {
                    return local_type_id;
                }

                return types.insert_type_from_type(
                    Type::ArraySized {
                        element: mapped_element,
                        count: mapped_count,
                        is_readonly,
                    },
                    local_type_id,
                );
            }
            Expression::Parenthesized { expression } => {
                return self.apply_projection_substitutions_from_expression(
                    module,
                    profile,
                    expression,
                    local_type_id,
                    owner_tree,
                    owner_symbols,
                    substitutions,
                    types,
                );
            }
            Expression::TypeUnary {
                operator: destack_dir::TypeUnaryOperator::AsComptime,
                right,
            } => {
                return self.apply_projection_substitutions_from_expression(
                    module,
                    profile,
                    right,
                    local_type_id,
                    owner_tree,
                    owner_symbols,
                    substitutions,
                    types,
                );
            }
            _ => {}
        }

        // map direct reference substitutions for projected symbols
        let (symbol, static_arguments) = match types.get_type(local_type_id).clone() {
            Type::Reference {
                symbol,
                static_arguments,
            } => (symbol, static_arguments),
            _ => return local_type_id,
        };
        if static_arguments.is_none() {
            if let Some(mapped_symbol) = self.projection_substitution_symbol_from_expression(
                module,
                profile,
                expression_id,
                owner_tree,
                owner_symbols,
            ) && (mapped_symbol == symbol
                || (mapped_symbol.module_id == symbol.module_id
                    && mapped_symbol.local_id.id == symbol.local_id.id))
                && let Some(substitution) =
                    self.projection_substitution_type_for_symbol(mapped_symbol, substitutions)
            {
                return self.normalized_projection_substitution_type(substitution, types);
            }

            if let Some(substitution) =
                self.projection_substitution_type_for_symbol(symbol, substitutions)
            {
                return self.normalized_projection_substitution_type(substitution, types);
            }

            let mut visited_symbols = HashSet::new();
            if let Ok(Some(value)) = self.static_expression_from_constant_reference_specialized(
                module,
                profile,
                symbol,
                owner_tree,
                owner_symbols,
                types,
                substitutions,
                &mut visited_symbols,
            ) && let Some(value_type_id) = self.static_expression_type_id_for_substitution(
                expression_id.into_any(),
                &value,
                types,
            ) {
                return value_type_id;
            }

            return local_type_id;
        }

        // map reference static arguments that originate from owner projections
        let Some(static_arguments) = static_arguments else {
            return local_type_id;
        };
        let Some(argument_nodes) = owner_tree.get(expression_id).static_arguments() else {
            return local_type_id;
        };

        // walk static arguments in lock-step with the alias expression arguments
        let mut changed = false;
        let mut mapped_arguments = static_arguments;
        for (argument_index, argument_node) in argument_nodes.iter().enumerate() {
            let Some(current_argument) = mapped_arguments.get(argument_index).cloned() else {
                continue;
            };
            let argument_expression_id = owner_tree.get(*argument_node).value();

            // replace direct symbol references with projection substitutions
            if let Some(target_symbol) = self.projection_substitution_symbol_from_expression(
                module,
                profile,
                argument_expression_id,
                owner_tree,
                owner_symbols,
            ) && let Some(substitution) =
                self.projection_substitution_type_for_symbol(target_symbol, substitutions)
            {
                let substitution =
                    self.normalized_projection_substitution_type(substitution, types);
                let substitution_value =
                    self.static_expression_from_projection_substitution_type(substitution, types);
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
            let mapped_type = self.apply_projection_substitutions_from_expression(
                module,
                profile,
                argument_expression_id,
                ty,
                owner_tree,
                owner_symbols,
                substitutions,
                types,
            );
            if mapped_type == ty {
                continue;
            }

            mapped_arguments[argument_index] = StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty: mapped_type },
            };
            changed = true;
        }

        if !changed {
            return local_type_id;
        }

        types.insert_type_from_type(
            Type::Reference {
                symbol,
                static_arguments: Some(mapped_arguments),
            },
            local_type_id,
        )
    }

    /// Apply associated projection substitutions to imported alias targets when needed.
    fn apply_associated_projection_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        alias_target_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if substitutions.is_empty() {
            return Ok(alias_target_id);
        }

        let mapped_alias_target = self
            .with_module_tree_symbols_or_local_for_stage(
                module,
                profile,
                target_symbol.module_id,
                tree,
                symbols,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(target_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return alias_target_id;
                    };
                    if primary_declaration.local_id.ty != NodeType::Member {
                        return alias_target_id;
                    }

                    let member_id = primary_declaration.local_id.into_typed::<Member>();
                    let Member::Type {
                        value: Some(alias_expression),
                        ..
                    } = owner_tree.get(member_id)
                    else {
                        return alias_target_id;
                    };

                    self.apply_projection_substitutions_from_expression(
                        owner_module,
                        profile,
                        *alias_expression,
                        alias_target_id,
                        owner_tree,
                        owner_symbols,
                        substitutions,
                        types,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;

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

    /// Materialize associated member projections with receiver substitutions.
    pub(crate) fn materialize_associated_member_projection(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        static_arguments: Option<&[StaticArgument]>,
        member_ty: Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        let options = self.analyze_context_options_for_module(module.id);
        let projection_environment = self.projection_environment_for_member(
            module,
            profile,
            source_id,
            target_symbol,
            receiver_symbol,
            receiver_arguments,
            static_arguments,
            Some(&member_ty),
            &options,
            None,
            tree,
            symbols,
            types,
        )?;
        let substitutions = projection_environment.substitutions;
        let owner_symbol = projection_environment.owner_symbol;

        // reject explicit static arguments on non-parameterized members
        if let Some(member_arguments) = static_arguments
            && !member_arguments.is_empty()
        {
            let parameter_count = self
                .with_module_tree_symbols_or_local_for_stage(
                    module,
                    profile,
                    target_symbol.module_id,
                    tree,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |owner_module, owner_tree, owner_symbols| {
                        self.collect_static_parameter_symbols(
                            owner_module,
                            target_symbol,
                            profile,
                            owner_tree,
                            owner_symbols,
                            types,
                        )
                        .map(|parameters| parameters.len())
                    },
                )
                .map_err(AnalyzeError::from)?;

            if parameter_count == Some(0) {
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: source_id
                        .into_global(module.id)
                        .into_anchored(Some(profile)),
                    message: "too many static arguments".to_string(),
                });
                return Ok(Type::Error);
            }
        }

        // re-evaluate associated comptime constants with receiver substitutions
        let member_kind = self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
            )
            .map_err(AnalyzeError::from)?;
        if matches!(
            member_kind,
            Some(StaticMemberSymbolKind::AssociatedComptimeConst)
        ) {
            let mut visited_symbols = HashSet::new();
            if let Some(static_value) = self.static_expression_from_constant_reference_specialized(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
                &substitutions,
                &mut visited_symbols,
            )? && let Some(value_type_id) =
                self.static_expression_type_id_for_substitution(source_id, &static_value, types)
            {
                return Ok(types.get_type(value_type_id).clone());
            }
        }

        // materialize associated type alias targets with merged substitutions
        let mut alias_target_id = if let Some(alias_target_id) = self
            .alias_target_type_id_for_symbol(
                module,
                profile,
                target_symbol,
                source_id,
                symbols,
                types,
            ) {
            alias_target_id
        } else if let Some(alias_target_id) = self.relaxed_alias_target_type_id_for_symbol(
            module,
            profile,
            target_symbol,
            source_id,
            tree,
            symbols,
            types,
        ) {
            alias_target_id
        } else {
            return Ok(member_ty);
        };
        let is_self_alias_target = matches!(
            types.get_type(alias_target_id),
            Type::Reference {
                symbol,
                static_arguments: None,
            } if *symbol == target_symbol
        );

        // refresh local alias targets from source expressions once projection context is available
        if target_symbol.module_id == module.id {
            let alias_source = types.get_type_source(alias_target_id);
            if let Ok(alias_expression_id) = alias_source.try_into_typed::<Expression>()
                && tree.has_node_id(alias_expression_id.id)
            {
                alias_target_id = self.resolve_declared_type_expression_fresh(
                    module,
                    profile,
                    alias_expression_id,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
            }
        }
        // refresh remote self-referential aliases in their owner module with remote tree ids
        else if is_self_alias_target {
            let reevaluated_remote_target = self
                .with_module_tree_symbols_for_stage(
                    module,
                    profile,
                    target_symbol.module_id,
                    AnalyzeDependencyStage::Declare,
                    |owner_module,
                     owner_tree,
                     owner_symbols|
                     -> AnalyzeResult<Option<LocalTypeId>> {
                        let symbol_entry = owner_symbols.get_symbol(target_symbol.local_id);
                        let Some(primary_declaration) = symbol_entry.primary_declaration else {
                            return Ok(None);
                        };
                        if primary_declaration.local_id.ty != NodeType::Member {
                            return Ok(None);
                        }

                        let member_id = primary_declaration.local_id.into_typed::<Member>();
                        let Member::Type {
                            value: Some(alias_value_expression),
                            ..
                        } = owner_tree.get(member_id)
                        else {
                            return Ok(None);
                        };

                        let mut owner_types = owner_module.dir(profile).types.write();
                        let reevaluated_remote_id = self.resolve_declared_type_expression_fresh(
                            owner_module,
                            profile,
                            *alias_value_expression,
                            owner_tree,
                            owner_symbols,
                            &mut owner_types,
                            true,
                            true,
                        )?;
                        let reevaluated_remote_ty = owner_types.get_type(reevaluated_remote_id);
                        let local_target_id = self.import_type_from_remote_for_node(
                            source_id,
                            reevaluated_remote_ty,
                            &owner_types,
                            target_symbol,
                            types,
                        );
                        Ok(Some(local_target_id))
                    },
                )
                .map_err(AnalyzeError::from)??;
            if let Some(reevaluated_remote_target) = reevaluated_remote_target {
                alias_target_id = reevaluated_remote_target;
            }
        }

        alias_target_id = self.apply_associated_projection_substitutions(
            module,
            profile,
            target_symbol,
            alias_target_id,
            &substitutions,
            tree,
            symbols,
            types,
        )?;

        let mut materialize_cache = TypeRewriteCache::new();
        let materialized_alias = self.materialize_static_arguments_in_type(
            module,
            profile,
            alias_target_id,
            tree,
            symbols,
            types,
            &mut materialize_cache,
        );

        // substitute after materialization so owner scoped references are concrete first
        let mapped_alias = if substitutions.is_empty() {
            materialized_alias
        } else {
            let mut substitution_cache = HashMap::new();
            self.substitute_static_parameters(
                materialized_alias,
                &substitutions,
                types,
                &mut substitution_cache,
            )
        };

        // rematerialize after substitution to normalize mapped references
        let mapped_alias = self.materialize_static_arguments_in_type(
            module,
            profile,
            mapped_alias,
            tree,
            symbols,
            types,
            &mut materialize_cache,
        );

        let mapped_alias = if let Some(owner_symbol) = owner_symbol {
            let mut rewriter = AssociatedAliasProjectionRewriter::new(
                self,
                module,
                profile,
                source_id,
                owner_symbol,
                &substitutions,
                tree,
                symbols,
            );
            rewriter.rewrite_type_id(types, mapped_alias)
        } else {
            mapped_alias
        };

        Ok(types.get_type(mapped_alias).clone())
    }

    /// Check whether an associated type alias requires explicit static arguments.
    pub(crate) fn associated_type_requires_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<bool> {
        // only associated type aliases can require projection arguments
        self.with_module_tree_symbols_or_local_for_stage(
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |_owner_module, owner_tree, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                if symbol_entry.ty != SymbolType::TypeAlias {
                    return false;
                }

                // skip aliases that are not declared as members
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return false;
                };
                if primary_declaration.local_id.ty != NodeType::Member {
                    return false;
                }

                // require arguments when any parameter has no default
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                let Member::Type {
                    static_parameters, ..
                } = owner_tree.get(member_id)
                else {
                    return false;
                };
                let Some(parameters) = static_parameters.as_ref() else {
                    return false;
                };
                if parameters.is_empty() {
                    return false;
                }

                parameters
                    .iter()
                    .any(|parameter_id| !owner_tree.get(*parameter_id).has_default())
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the owning declaration symbol for a member symbol.
    pub(crate) fn query_owner_symbol_for_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        self.with_module_symbols_or_local_for_stage(
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
        self.with_module_tree_symbols_or_local_for_stage(
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
