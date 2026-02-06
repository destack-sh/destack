use std::collections::{HashMap, HashSet};

use crate::analyze::common::{
    CanonicalSymbolMode, REWRITER_TAG_ASSOCIATED_ALIAS, TypeRewriteCache, TypeWalkContext,
    TypeWalkKey, rewrite_type_with_cache,
};
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, DependencyItem, DependencyMode, Expression, GlobalNodeId, GlobalSymbolId,
    Heritage, LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, NodeTree, NodeType, StaticArgument,
    StaticKey, SymbolSpaceOrder, SymbolTable, SymbolType, Type, TypeRewriter, TypeRewriterOptions,
    TypeTable,
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

/// A resolved type-member target and projection context.
#[derive(Clone, Debug)]
pub(crate) struct TypeMemberResolution {
    /// The resolved member symbol.
    pub(crate) target_symbol: GlobalSymbolId,
    /// The resolved receiver symbol for associated projections.
    pub(crate) receiver_symbol: Option<GlobalSymbolId>,
    /// The receiver static arguments used for associated projections.
    pub(crate) receiver_arguments: Vec<StaticArgument>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Select a member symbol used in a type expression.
    pub(crate) fn select_type_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<TypeMemberResolution>> {
        // select namespace import members first
        if let Some(namespace_symbol) = self.select_namespace_member_symbol(
            module,
            profile,
            expression_id,
            left,
            member_key,
            tree,
            symbols,
        ) {
            return Ok(Some(TypeMemberResolution {
                target_symbol: namespace_symbol,
                receiver_symbol: None,
                receiver_arguments: Vec::new(),
            }));
        }

        // then select projected members through nominal receiver types
        if let Some((projected_symbol, projected_receiver_symbol, projected_args)) = self
            .select_associated_projection_member_symbol(
                module,
                profile,
                expression_id,
                left,
                member_key,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?
        {
            return Ok(Some(TypeMemberResolution {
                target_symbol: projected_symbol,
                receiver_symbol: Some(projected_receiver_symbol),
                receiver_arguments: projected_args,
            }));
        }

        Ok(None)
    }

    /// Select a namespace-import member symbol used in a type expression.
    fn select_namespace_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // require a reference expression on the left side
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(left)
        else {
            return None;
        };

        // namespace imports are local dependency items
        if target_symbol.module_id != module.id {
            return None;
        }

        // require a dependency declaration
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::DependencyItem {
            return None;
        }

        // select dependency mode and target module
        let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
        let dependency = tree.get(dependency_id);
        let (mode, target_module) = match dependency {
            DependencyItem::Remote {
                mode,
                target_module,
                ..
            } => (*mode, Some(*target_module)),
            DependencyItem::UnresolvedRemote {
                mode,
                target_module,
                ..
            } => (*mode, *target_module),
            _ => (DependencyMode::Item, None),
        };

        // only namespace imports can project members
        if mode != DependencyMode::Namespace {
            return None;
        }

        // select type space first, then value space
        let target_module = target_module?;
        let target_module = target_module.ty.or(target_module.value)?;
        self.resolve_export_symbol_for_target(
            module.id,
            expression_id.into_global_any(module.id),
            target_module,
            profile,
            SymbolSpaceOrder::TypeThenValue,
            member_key,
        )
        .ok()
        .flatten()
    }

    /// Select a projected static member symbol from a nominal receiver.
    pub(crate) fn select_associated_projection_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, GlobalSymbolId, Vec<StaticArgument>)>> {
        // evaluate the receiver to a reference-like type
        let left_ty_id = self.try_evaluate_expression_to_type(
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
        let receiver_reference = match left_ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => Some((symbol, static_arguments.unwrap_or_default())),
            Type::Value { value } => {
                if let Type::Reference {
                    symbol,
                    static_arguments,
                } = types.get_type(value)
                {
                    Some((*symbol, static_arguments.clone().unwrap_or_default()))
                } else {
                    None
                }
            }
            _ => None,
        };
        let Some((mut lookup_symbol, mut lookup_arguments)) = receiver_reference else {
            return Ok(None);
        };

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
            if let Some((alias_symbol, alias_arguments, _)) =
                self.unwrap_type_symbol(types, mapped_alias_target)
            {
                lookup_symbol = alias_symbol;
                lookup_arguments = alias_arguments.unwrap_or_default();
            }
        }

        // resolve the projected member symbol on the normalized receiver symbol
        let projected_symbol = self.with_module_tree_symbols_or_local(
            module,
            profile,
            lookup_symbol.module_id,
            tree,
            symbols,
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
        );
        let Some(projected_symbol) = projected_symbol else {
            return Ok(None);
        };

        Ok(Some((projected_symbol, lookup_symbol, lookup_arguments)))
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

        // collect direct heritage expressions for this symbol
        let heritage_expressions = self.heritage_expressions_for_associated_projection(
            module,
            profile,
            current_symbol,
            tree,
            symbols,
        );
        for heritage_expression_id in heritage_expressions {
            // resolve the heritage target and applied arguments
            let resolved_heritage = self.with_module_tree_symbols_or_local(
                module,
                profile,
                heritage_expression_id.module_id,
                tree,
                symbols,
                |owner_module,
                 owner_tree,
                 owner_symbols|
                 -> AnalyzeResult<Option<(GlobalSymbolId, Vec<StaticArgument>)>> {
                    let expression_id = heritage_expression_id.local_id;
                    let expression = owner_tree.get(expression_id);
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
                        self.try_evaluate_expression_to_type(
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

                    if let Some((resolved_symbol, resolved_arguments, _)) =
                        self.unwrap_type_symbol(types, heritage_type_id)
                    {
                        let resolved_arguments = resolved_arguments.unwrap_or_default();
                        return Ok(Some((resolved_symbol, resolved_arguments)));
                    }

                    Ok(Some((target_symbol, Vec::new())))
                },
            )?;
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
    ) -> Vec<GlobalNodeId<Expression>> {
        self.with_module_tree_symbols_or_local(
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
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
    }

    /// Build interface substitutions for a projected interface member on a receiver type.
    pub(crate) fn interface_member_substitutions_for_receiver(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        receiver_symbol: GlobalSymbolId,
        receiver_arguments: &[StaticArgument],
        member_symbol: GlobalSymbolId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, LocalTypeId>>> {
        // resolve the owning declaration for the projected member
        let Some(interface_symbol) =
            self.owner_symbol_for_member_symbol(module, profile, member_symbol, symbols)
        else {
            return Ok(None);
        };
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
        let declared_extension_symbols = self.with_module_tree_symbols_or_local(
            module,
            profile,
            canonical_receiver_symbol.module_id,
            tree,
            symbols,
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
        );

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
            let substitutions = self.with_module_tree_symbols_or_local(
                module,
                profile,
                extension_symbol.module_id,
                tree,
                symbols,
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
                    let Declaration::Extension { heritage, .. } = owner_tree.get(declaration_id)
                    else {
                        return Ok(None);
                    };
                    let Some(implements_types) = heritage.implements_types.as_ref() else {
                        return Ok(None);
                    };

                    let receiver_substitutions = self
                        .build_type_parameter_substitutions_for_symbol(
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
            )?;
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
        self.with_module_tree_symbols_or_local(
            module,
            profile,
            receiver_symbol.module_id,
            tree,
            symbols,
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
        for interface_expression_id in implements_types {
            let Some(target_symbol) = owner_tree.get(*interface_expression_id).target_symbol()
            else {
                continue;
            };
            let canonical_target = self.canonical_symbol_id(
                owner_module,
                owner_symbols,
                profile,
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            if canonical_target != interface_symbol {
                continue;
            }

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
                    interface_symbol,
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
                interface_symbol,
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

            return Ok(Some(substitutions));
        }

        Ok(None)
    }

    /// Materialize associated member projections with receiver substitutions.
    #[allow(clippy::too_many_arguments)]
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
        let mut substitutions = HashMap::new();
        let owner_symbol =
            self.owner_symbol_for_member_symbol(module, profile, target_symbol, symbols);

        // map owner parameters from receiver substitutions
        if let Some(owner_symbol) = owner_symbol
            && let Some(receiver_symbol) = receiver_symbol
            && let Some(owner_substitutions) = self
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
                )?
        {
            substitutions.extend(owner_substitutions);
        }

        // map interface substitutions for projected members
        let options = self.analyze_context_options_for_module(module.id);
        if let Some(receiver_symbol) = receiver_symbol
            && let Some(interface_substitutions) = self
                .interface_member_substitutions_for_receiver(
                    module,
                    profile,
                    source_id,
                    receiver_symbol,
                    receiver_arguments,
                    target_symbol,
                    &options,
                    tree,
                    symbols,
                    types,
                )?
        {
            substitutions.extend(interface_substitutions);
        }

        // map extension substitutions when the member is extension-owned
        if let Ok(Some(extension_context)) = self.resolve_extension_member_context(
            module,
            profile,
            source_id,
            target_symbol,
            receiver_arguments,
            &options,
            tree,
            symbols,
            types,
        ) {
            substitutions.extend(extension_context.substitutions);
        }

        // map member parameters from explicit member arguments
        if let Some(member_arguments) = static_arguments
            && !member_arguments.is_empty()
        {
            let parameter_count = self.with_module_tree_symbols_or_local(
                module,
                profile,
                target_symbol.module_id,
                tree,
                symbols,
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
            );

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

        let resolved_member_arguments = self.resolve_type_reference_static_arguments_for_symbol(
            module,
            profile,
            source_id,
            target_symbol,
            static_arguments,
            true,
            &options,
            tree,
            symbols,
            types,
        )?;
        if let Some(member_arguments) = resolved_member_arguments.as_deref().or(static_arguments) {
            let member_substitutions = self.build_type_parameter_substitutions_for_symbol(
                module,
                profile,
                target_symbol,
                source_id,
                member_arguments,
                tree,
                symbols,
                types,
            );
            substitutions.extend(member_substitutions);
        }
        // fall back to reference-carried arguments when present
        else if let Type::Reference {
            symbol,
            static_arguments: Some(member_arguments),
        } = &member_ty
        {
            let member_substitutions = self.build_type_parameter_substitutions_for_symbol(
                module,
                profile,
                *symbol,
                source_id,
                member_arguments,
                tree,
                symbols,
                types,
            );
            substitutions.extend(member_substitutions);
        }

        // materialize associated type alias targets with merged substitutions
        let Some(alias_target_id) = self.alias_target_type_id_for_symbol(
            module,
            profile,
            target_symbol,
            source_id,
            symbols,
            types,
        ) else {
            return Ok(member_ty);
        };

        let mapped_alias = if substitutions.is_empty() {
            alias_target_id
        } else {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(alias_target_id, &substitutions, types, &mut cache)
        };
        let mut materialize_cache = TypeRewriteCache::new();
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
    ) -> bool {
        // only associated type aliases can require projection arguments
        self.with_module_tree_symbols_or_local(
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
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
    }

    /// Resolve the owning declaration symbol for a member symbol.
    pub(crate) fn owner_symbol_for_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        self.with_module_symbols_or_local(
            module,
            profile,
            member_symbol.module_id,
            symbols,
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
    }
}
