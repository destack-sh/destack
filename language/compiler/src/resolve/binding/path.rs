use destack_dir::{
    Argument, Declaration, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalScopeId,
    LocalScopeMark, LocalSymbolId, NodeTree, NodeType, Path, Scope, ScopeKind, StaticKey, StringId,
    SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use destack_workspace::{Module, ProfileId};

use crate::resolve::binding::cache::{
    ResolveAbsoluteSymbolCacheKey, ResolveExpressionCache, ResolveScopeIndexCache,
};
use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    fn allow_runtime_namespace_member_fallback(
        &self,
        module: &Module,
        space_order: SymbolSpaceOrder,
    ) -> bool {
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return false;
        }

        matches!(
            space_order,
            SymbolSpaceOrder::ValueOnly | SymbolSpaceOrder::ValueThenType
        )
    }

    /// Resolve CommonJS runtime paths (`module` and `exports`) for CommonJS modules.
    fn resolve_commonjs_runtime_path(
        &self,
        module: &Module,
        profile_id: ProfileId,
        expression_id: LocalNodeId<Expression>,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        tree: &mut NodeTree,
    ) -> Option<Expression> {
        // only value space lookups can resolve runtime commonjs names
        if !space_order.spaces().contains(&SymbolSpace::Value) {
            return None;
        }

        // only commonjs modules expose these runtime bindings by default
        if !module.module_format.is_commonjs() {
            return None;
        }

        // only handle top-level runtime roots
        let first_segment = path.first_segment()?;
        let first_segment_str = self.program.strings.get(first_segment);
        let is_module_root = first_segment_str.as_str() == "module";
        let is_exports_root = first_segment_str.as_str() == "exports";
        let is_self_root = first_segment_str.as_str() == "self";
        let is_define_root = first_segment_str.as_str() == "define";
        if !is_module_root && !is_exports_root && !is_self_root && !is_define_root {
            return None;
        }

        // resolve both roots against the current module namespace symbol
        let namespace_symbol = module
            .dir(profile_id)
            .namespace_symbol
            .into_global(module.id);

        // map `module` directly to the runtime module object
        if is_module_root {
            let root_path = Path {
                segments: vec![first_segment].into(),
            };
            let root_expression = Expression::GlobalReference {
                path: root_path,
                static_arguments: None,
                target_symbol: namespace_symbol,
            };

            if path.segments.len() == 1 {
                return Some(Expression::GlobalReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: namespace_symbol,
                });
            }

            return Some(self.build_member_chain(
                expression_id,
                root_expression,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // map `self` and `define` to unresolved runtime globals in commonjs wrappers
        if is_self_root || is_define_root {
            let root_path = Path {
                segments: vec![first_segment].into(),
            };
            let root_expression = Expression::GlobalReference {
                path: root_path.clone(),
                static_arguments: None,
                target_symbol: namespace_symbol,
            };

            if path.segments.len() == 1 {
                return Some(Expression::GlobalReference {
                    path: root_path,
                    static_arguments,
                    target_symbol: namespace_symbol,
                });
            }

            return Some(self.build_member_chain(
                expression_id,
                root_expression,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // map `exports` to the commonjs runtime alias binding
        let exports_name = self.program.strings.intern("exports");
        let exports_path = Path {
            segments: vec![exports_name].into(),
        };
        let exports_expression = Expression::GlobalReference {
            path: exports_path.clone(),
            static_arguments: None,
            target_symbol: namespace_symbol,
        };

        if path.segments.len() == 1 {
            return Some(Expression::GlobalReference {
                path: exports_path,
                static_arguments,
                target_symbol: namespace_symbol,
            });
        }

        Some(self.build_member_chain(
            expression_id,
            exports_expression,
            &path.slice(1..),
            static_arguments,
            tree,
        ))
    }

    /// Resolve an inherited associated type name from an enclosing declaration heritage.
    fn resolve_heritage_associated_type_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        _origin: GlobalNodeIdAny,
        expression_id: LocalNodeId<Expression>,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        member_name: StringId,
        symbols: &SymbolTable,
        tree: &NodeTree,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        let mut current_scope = scope;
        let member_key = StaticKey::Name(member_name);

        // search enclosing declaration scopes from inner to outer
        loop {
            let Some(owner_id) = current_scope.1.owner_id else {
                if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                    current_scope = (
                        parent_scope_id,
                        symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            };

            let owner_symbol = symbols.get_symbol(owner_id);
            let Some(primary_declaration) = owner_symbol.primary_declaration else {
                if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                    current_scope = (
                        parent_scope_id,
                        symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            };
            if primary_declaration.local_id.ty != NodeType::Declaration {
                if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                    current_scope = (
                        parent_scope_id,
                        symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            }

            let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
            let declaration = tree.get(declaration_id);
            let heritage = match declaration {
                Declaration::Struct { heritage, .. }
                | Declaration::Class { heritage, .. }
                | Declaration::Enum { heritage, .. }
                | Declaration::Interface { heritage, .. }
                | Declaration::Extension { heritage, .. } => Some(heritage),
                _ => None,
            };
            let Some(heritage) = heritage else {
                if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                    current_scope = (
                        parent_scope_id,
                        symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            };

            // try heritage targets in declaration order
            let extends_types = heritage.extends_types.as_deref().unwrap_or_default();
            let implements_types = heritage.implements_types.as_deref().unwrap_or_default();
            for heritage_expression_id in extends_types.iter().chain(implements_types.iter()) {
                let Some(heritage_symbol) = tree.get(*heritage_expression_id).target_symbol()
                else {
                    continue;
                };

                let resolved_member = self.resolve_static_member_symbol(
                    module,
                    profile,
                    expression_id,
                    heritage_symbol,
                    member_key,
                    tree,
                    symbols,
                );
                match resolved_member {
                    Ok(symbol) => return Ok(Some(symbol)),
                    Err(ResolveError::UnsupportedConstruct { .. }) => continue,
                    Err(ResolveError::MissingSymbol { .. }) => continue,
                    Err(error) => return Err(error),
                }
            }

            return Ok(None);
        }
    }

    /// Build a Member expression chain from a root expression with remaining path segments.
    pub(crate) fn resolve_absolute_symbol(
        &self,
        module: &Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        key: StaticKey,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<LocalSymbolId> {
        // track the nearest fallback symbol
        let mut scope = scope;
        let mut fallback = None;

        // walk scopes from inner to outer
        loop {
            // scan the current scope for a preferred match
            let (preferred, scope_fallback) = self.find_symbol_in_scope_cached(
                scope.0,
                scope.1,
                key,
                space_order,
                symbols,
                scope.2,
                scope_cache.as_deref_mut(),
            );

            // return the preferred match when found
            if let Some(symbol_id) = preferred {
                return Ok(symbol_id);
            }

            // remember the nearest fallback symbol
            if fallback.is_none() {
                fallback = scope_fallback;
            }

            // resolve JS/TS hoisted value bindings
            if let Some(hoisted_symbol) = self.find_hoisted_symbol_in_scope(
                module,
                scope.1,
                key,
                space_order,
                symbols,
                scope.2,
            ) {
                return Ok(hoisted_symbol);
            }

            // allow same-scope forward references for JS/TS bindings
            if self.allow_forward_binding_lookup(module, space_order) {
                let (forward_preferred, _) =
                    self.find_symbol_in_scope(scope.1, key, space_order, symbols, None);
                if let Some(forward_symbol) = forward_preferred {
                    return Ok(forward_symbol);
                }
            }

            // move to the parent scope when available
            if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            } else {
                break;
            }
        }

        // fall back to the nearest match when no preferred match exists
        if let Some(fallback) = fallback {
            return Ok(fallback);
        }

        // report missing symbol after walking all scopes
        Err(ResolveError::MissingSymbol {
            node: node.into_anchored(Some(profile_id)),
            scope: scope.0.into_global(module.id),
            via_module: None,
            key,
        })
    }

    /// Resolve a path against merged ambient namespace scopes when inside a namespace.
    fn resolve_ambient_namespace_path(
        &self,
        module: &Module,
        profile_id: ProfileId,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<Option<Expression>> {
        if !self.module_is_ambient_lib(module) {
            return Ok(None);
        }

        let mut scope = scope;
        loop {
            if scope.1.kind == ScopeKind::Namespace
                && let Some(owner_id) = scope.1.owner_id
            {
                match self.resolve_relative_symbol_with_ambient_merge(
                    module,
                    profile_id,
                    node,
                    owner_id,
                    path,
                    space_order,
                    symbols,
                    scope_cache.as_deref_mut(),
                ) {
                    Ok((resolved_id, None)) => {
                        if resolved_id.module_id == module.id {
                            return Ok(Some(self.resolve_symbol_to_expression(
                                module,
                                resolved_id.local_id,
                                path,
                                static_arguments,
                                symbols,
                            )));
                        }

                        return Ok(Some(Expression::GlobalReference {
                            path: path.clone(),
                            static_arguments,
                            target_symbol: resolved_id,
                        }));
                    }
                    Ok((resolved_id, Some(remaining))) => {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        let root_expr = if resolved_id.module_id == module.id {
                            self.resolve_symbol_to_expression(
                                module,
                                resolved_id.local_id,
                                &resolved_path,
                                None,
                                symbols,
                            )
                        } else {
                            Expression::GlobalReference {
                                path: resolved_path,
                                static_arguments: None,
                                target_symbol: resolved_id,
                            }
                        };
                        return Ok(Some(self.build_member_chain(
                            expression_id,
                            root_expr,
                            &remaining,
                            static_arguments,
                            tree,
                        )));
                    }
                    Err(ResolveError::MissingSymbol { .. }) => {}
                    Err(error) => return Err(error),
                }
            }

            if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            } else {
                break;
            }
        }

        Ok(None)
    }

    /// Resolve a symbol from the prelude by name.
    pub(crate) fn resolve_prelude_symbol(
        &self,
        name: StringId,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // check if prelude injection is enabled
        if !self.options.inject_prelude {
            return Ok(None);
        }

        // get the prelude module ID from builtins
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(None);
        };
        let prelude_module_id = builtins.prelude_module_id;

        // ensure the prelude module has been resolved (may yield)
        self.require_resolve_module(prelude_module_id, profile)?;

        // get the prelude module
        let prelude_module = self.program.modules.get(prelude_module_id);
        let prelude_module = prelude_module.read();
        let prelude_dir = prelude_module.dir(profile);

        // look up symbol by name in prelude's export table
        let key = StaticKey::Name(name);
        let export_spaces = SymbolSpaceOrder::ValueThenType;
        let exports = prelude_dir.exported_symbols.read();
        let tree = prelude_dir.tree.read();
        let Some(symbol_id) = self.resolve_exported_symbol(
            &prelude_module,
            profile,
            &exports,
            &tree,
            export_spaces,
            key,
        ) else {
            return Ok(None);
        };

        Ok(Some(symbol_id))
    }

    /// Resolve a path starting from a prelude symbol.
    /// Similar to resolve_local_path but for symbols from the prelude module.
    fn resolve_prelude_path(
        &self,
        _module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        prelude_symbol: GlobalSymbolId,
        profile: ProfileId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        tree: &mut NodeTree,
    ) -> ResolveResult<Expression> {
        let remaining_segments = &path.segments[1..];

        // single-segment path: just return the GlobalReference
        if remaining_segments.is_empty() {
            return Ok(Expression::GlobalReference {
                path: path.clone(),
                static_arguments,
                target_symbol: prelude_symbol,
            });
        }

        // multi-segment path: need to check if prelude symbol is a namespace
        let prelude_module_id = prelude_symbol.module_id;
        let prelude_module = self.program.modules.get(prelude_module_id);
        let prelude_module = prelude_module.read();
        let prelude_symbols = prelude_module.dir(profile).symbols.read();

        let local_symbol_id = prelude_symbol.local_id;
        let symbol = prelude_symbols.get_symbol(local_symbol_id);

        if symbol.kind == SymbolKind::Namespace {
            // resolve remaining path within the prelude module's namespace
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol_with_ambient_merge(
                &prelude_module,
                profile,
                node,
                local_symbol_id,
                &remaining_path,
                space_order,
                &prelude_symbols,
                None,
            ) {
                Ok((resolved_id, None)) => {
                    // fully resolved within prelude
                    return Ok(Expression::GlobalReference {
                        path: path.clone(),
                        static_arguments,
                        target_symbol: resolved_id,
                    });
                }
                Ok((resolved_id, Some(remaining))) => {
                    // partially resolved, build Member chain for remaining
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = Expression::GlobalReference {
                        path: resolved_path,
                        static_arguments: None,
                        target_symbol: resolved_id,
                    };
                    return Ok(self.build_member_chain(
                        expression_id,
                        root_expr,
                        &remaining,
                        static_arguments,
                        tree,
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        // non-namespace symbol: remaining segments become Member chain
        let root_path = Path {
            segments: vec![path.first_segment().unwrap()].into(),
        };
        let root_expr = Expression::GlobalReference {
            path: root_path,
            static_arguments: None,
            target_symbol: prelude_symbol,
        };
        Ok(self.build_member_chain(
            expression_id,
            root_expr,
            &path.slice(1..),
            static_arguments,
            tree,
        ))
    }

    /// Resolve a path from ambient lib modules, if available.
    fn resolve_ambient_path(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        profile_id: ProfileId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        tree: &mut NodeTree,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<Option<Expression>> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(None);
        };
        let profile = self.program.profile(profile_id);
        let profile_key = &profile.key;
        let Some(ambient_modules) = builtins.ambient_libs(profile_key) else {
            return Ok(None);
        };
        let first_segment = path.first_segment().expect("path is empty");
        let key = StaticKey::Name(first_segment);

        // prefer cached declared lib symbols when available
        if let Some(symbol_id) =
            builtins.get_declared_lib_symbol_from(profile_key, first_segment, space_order)
        {
            self.require_resolve_module_prepare_if_needed(
                module.id,
                symbol_id.module_id,
                profile_id,
            )?;
            let ambient_module = self.program.modules.get(symbol_id.module_id);
            let ambient_module = ambient_module.read();
            let ambient_dir = ambient_module.dir(profile_id);
            let symbols = ambient_dir.symbols.read();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            let global_this_name = self.program.strings.intern("globalThis");
            let matches_space = symbol.space == SymbolSpace::TypeValue
                || space_order.spaces().contains(&symbol.space)
                || (symbol.space == SymbolSpace::Value && first_segment == global_this_name);
            if matches_space {
                if path.segments.len() == 1 {
                    return Ok(Some(Expression::GlobalReference {
                        path: path.clone(),
                        static_arguments,
                        target_symbol: symbol_id,
                    }));
                }

                if symbol.kind == SymbolKind::Namespace {
                    let remaining_path = path.slice(1..);
                    match self.resolve_relative_symbol_with_ambient_merge(
                        &ambient_module,
                        profile_id,
                        node,
                        symbol_id.local_id,
                        &remaining_path,
                        space_order,
                        &symbols,
                        scope_cache.as_deref_mut(),
                    ) {
                        Ok((resolved_id, None)) => {
                            return Ok(Some(Expression::GlobalReference {
                                path: path.clone(),
                                static_arguments,
                                target_symbol: resolved_id,
                            }));
                        }
                        Ok((resolved_id, Some(remaining))) => {
                            let resolved_path =
                                path.slice(0..path.segments.len() - remaining.segments.len());
                            let root_expr = Expression::GlobalReference {
                                path: resolved_path,
                                static_arguments: None,
                                target_symbol: resolved_id,
                            };
                            return Ok(Some(self.build_member_chain(
                                expression_id,
                                root_expr,
                                &remaining,
                                static_arguments,
                                tree,
                            )));
                        }
                        Err(e) => return Err(e),
                    }
                }

                let root_path = Path {
                    segments: vec![first_segment].into(),
                };
                let root_expr = Expression::GlobalReference {
                    path: root_path,
                    static_arguments: None,
                    target_symbol: symbol_id,
                };
                return Ok(Some(self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(1..),
                    static_arguments,
                    tree,
                )));
            }
        }

        // search ambient lib namespace scopes in order
        for module_id in ambient_modules {
            // skip self
            if module_id == module.id {
                continue;
            }

            // read the ambient module's symbols
            self.require_resolve_module_prepare_if_needed(module.id, module_id, profile_id)?;
            let ambient_module = self.program.modules.get(module_id);
            let ambient_module = ambient_module.read();
            let ambient_dir = ambient_module.dir(profile_id);
            let symbols = ambient_dir.symbols.read();

            // find symbol in ambient lib global augmentation scope
            let global_scope = symbols.get_scope_by_id(ambient_dir.global_augmentation_scope);
            let symbol_id = self.resolve_absolute_symbol(
                &ambient_module,
                profile_id,
                node,
                (
                    ambient_dir.global_augmentation_scope,
                    global_scope,
                    LocalScopeMark::end(),
                ),
                key,
                space_order,
                &symbols,
                scope_cache.as_deref_mut(),
            );
            let symbol_id = match symbol_id {
                Ok(symbol_id) => symbol_id,
                Err(_) => continue,
            };

            // single-segment path: just return the GlobalReference
            if path.segments.len() == 1 {
                return Ok(Some(Expression::GlobalReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: symbol_id.into_global(module_id),
                }));
            }

            // multi-segment path: resolve in nested namespace scope
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.kind == SymbolKind::Namespace {
                let remaining_path = path.slice(1..);
                match self.resolve_relative_symbol_with_ambient_merge(
                    &ambient_module,
                    profile_id,
                    node,
                    symbol_id,
                    &remaining_path,
                    space_order,
                    &symbols,
                    scope_cache.as_deref_mut(),
                ) {
                    Ok((resolved_id, None)) => {
                        return Ok(Some(Expression::GlobalReference {
                            path: path.clone(),
                            static_arguments,
                            target_symbol: resolved_id,
                        }));
                    }
                    Ok((resolved_id, Some(remaining))) => {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            static_arguments: None,
                            target_symbol: resolved_id,
                        };
                        return Ok(Some(self.build_member_chain(
                            expression_id,
                            root_expr,
                            &remaining,
                            static_arguments,
                            tree,
                        )));
                    }
                    Err(e) => return Err(e),
                }
            }

            // fall back to member chain for non-namespace symbols
            let root_path = Path {
                segments: vec![first_segment].into(),
            };
            let root_expr = Expression::GlobalReference {
                path: root_path,
                static_arguments: None,
                target_symbol: symbol_id.into_global(module_id),
            };
            return Ok(Some(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            )));
        }

        Ok(None)
    }

    /// Resolve a relative path starting from a symbol.
    /// Returns the resolved symbol and any remaining path segments that couldn't be resolved
    /// (e.g., when hitting a non-namespace symbol with more segments to go).
    pub(crate) fn resolve_relative_symbol_with_ambient_merge(
        &self,
        module: &Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
        path: &Path,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<(GlobalSymbolId, Option<Path>)> {
        // try resolving within the current module first
        let resolved = self.resolve_relative_symbol(
            module,
            profile_id,
            node,
            symbol_id,
            path,
            space_order,
            symbols,
            scope_cache.as_deref_mut(),
        );
        let missing = match resolved {
            Ok((resolved_id, remaining)) => {
                return Ok((resolved_id.into_global(module.id), remaining));
            }
            Err(error @ ResolveError::MissingSymbol { .. }) => error,
            Err(error) => return Err(error),
        };

        // stop if the module is not an ambient lib module
        if !self.module_is_ambient_lib(module) {
            return Err(missing);
        }

        // gather ambient merge sources for the symbol key
        let symbol_entry = symbols.get_symbol(symbol_id);
        let Some(key) = symbol_entry.key else {
            return Err(missing);
        };
        let Some(ambient_sources) =
            self.get_ambient_lib_symbol_sources_for_space_order(profile_id, key, space_order)
        else {
            return Err(missing);
        };

        // search ambient sources for a matching path
        for source_symbol in ambient_sources {
            // avoid re locking the same module while holding its symbols lock
            if source_symbol.module_id == module.id {
                // skip the original symbol, then try resolving within this module scope
                if source_symbol.local_id == symbol_id {
                    continue;
                }

                // resolve using the existing symbols table
                match self.resolve_relative_symbol(
                    module,
                    profile_id,
                    node,
                    source_symbol.local_id,
                    path,
                    space_order,
                    symbols,
                    scope_cache.as_deref_mut(),
                ) {
                    Ok((resolved_id, remaining)) => {
                        return Ok((resolved_id.into_global(source_symbol.module_id), remaining));
                    }
                    Err(ResolveError::MissingSymbol { .. }) => {}
                    Err(error) => return Err(error),
                }
                continue;
            }

            // prepare and read the source module before resolving
            self.require_resolve_module_prepare_if_needed(
                module.id,
                source_symbol.module_id,
                profile_id,
            )?;
            let source_module = self.program.modules.get(source_symbol.module_id);
            let source_module = source_module.read();
            let source_dir = source_module.dir(profile_id);
            let source_symbols = source_dir.symbols.read();

            // resolve using the source module symbols table
            match self.resolve_relative_symbol(
                &source_module,
                profile_id,
                node,
                source_symbol.local_id,
                path,
                space_order,
                &source_symbols,
                scope_cache.as_deref_mut(),
            ) {
                Ok((resolved_id, remaining)) => {
                    return Ok((resolved_id.into_global(source_symbol.module_id), remaining));
                }
                Err(ResolveError::MissingSymbol { .. }) => {}
                Err(error) => return Err(error),
            }
        }

        Err(missing)
    }

    pub(crate) fn resolve_relative_symbol(
        &self,
        module: &Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
        path: &Path,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<(LocalSymbolId, Option<Path>)> {
        // track the current symbol as we walk segments
        let mut current_symbol_id = symbol_id;
        let segments = &path.segments;

        // walk the path segments
        for (i, &segment) in segments.iter().enumerate() {
            let symbol = symbols.get_symbol(current_symbol_id);

            // stop traversing when the symbol is not a namespace
            if symbol.kind != SymbolKind::Namespace {
                let remaining = path.slice(i..);
                return Ok((current_symbol_id, Some(remaining)));
            }

            // resolve the next segment in the namespace scope
            let key = StaticKey::Name(segment);
            let scope = symbols.get_scope_by_id(symbol.scope.0);
            let (preferred, fallback) = self.find_symbol_in_scope_cached(
                symbol.scope.0,
                scope,
                key,
                space_order,
                symbols,
                LocalScopeMark::end(),
                scope_cache.as_deref_mut(),
            );

            // advance to the next symbol when possible
            if let Some(symbol_id) = preferred.or(fallback) {
                current_symbol_id = symbol_id;
                continue;
            }

            // allow merged symbols to satisfy value or type paths
            if let Some(group_id) = symbol.merge_group {
                let group_symbols = symbols.merge_group_symbols(group_id);
                let match_space = |requested: SymbolSpace, actual: SymbolSpace| match requested {
                    SymbolSpace::Type => {
                        matches!(actual, SymbolSpace::Type | SymbolSpace::TypeValue)
                    }
                    SymbolSpace::Value => {
                        matches!(actual, SymbolSpace::Value | SymbolSpace::TypeValue)
                    }
                    SymbolSpace::TypeValue => matches!(actual, SymbolSpace::TypeValue),
                    SymbolSpace::Label => false,
                };

                for requested_space in space_order.spaces() {
                    let candidate = group_symbols.iter().copied().find(|group_symbol| {
                        let merged_symbol = symbols.get_symbol(*group_symbol);
                        merged_symbol.kind != SymbolKind::Namespace
                            && match_space(*requested_space, merged_symbol.space)
                    });
                    if let Some(group_symbol) = candidate {
                        let remaining = path.slice(i..);
                        return Ok((group_symbol, Some(remaining)));
                    }
                }
            }

            // fall back to runtime member access in JS/TS value paths
            if self.allow_runtime_namespace_member_fallback(module, space_order) {
                let remaining = path.slice(i..);
                return Ok((current_symbol_id, Some(remaining)));
            }

            // report a missing symbol in the namespace scope
            return Err(ResolveError::MissingSymbol {
                node: node.into_anchored(Some(profile_id)),
                scope: symbol.scope.0.into_global(module.id),
                via_module: None,
                key,
            });
        }

        Ok((current_symbol_id, None))
    }

    /// Resolve an absolute path.
    /// For non-namespace symbols with remaining path segments, creates Member expression chains.
    /// Falls back to prelude lookup if local lookup fails and inject_prelude is enabled.
    pub(crate) fn resolve_absolute_path(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
        cache: &mut ResolveExpressionCache,
    ) -> ResolveResult<Expression> {
        let first_segment = path.first_segment().expect("path is empty in {node:?}");
        let first_segment_str = self.program.strings.get(first_segment);
        let scope_mark = if module.language_type.is_declaration() {
            LocalScopeMark::end()
        } else {
            scope.2
        };
        let scope = (scope.0, scope.1, scope_mark);

        // resolve import.meta intrinsic
        if first_segment_str.as_str() == "import" && path.segments.len() >= 2 {
            let second_segment = path.segments[1];
            let second_segment_str = self.program.strings.get(second_segment);
            if second_segment_str.as_str() == "meta" {
                let root_expr = Expression::ImportMeta;
                if path.segments.len() == 2 {
                    return Ok(root_expr);
                }
                return Ok(self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(2..),
                    static_arguments,
                    tree,
                ));
            }
        }

        // keep `new.target` unresolved so validate can enforce lexical context rules
        if first_segment_str.as_str() == "new" && path.segments.len() >= 2 {
            let second_segment = path.segments[1];
            let second_segment_str = self.program.strings.get(second_segment);
            if second_segment_str.as_str() == "target" {
                return Ok(Expression::UnresolvedPath {
                    path: path.clone(),
                    static_arguments,
                    space_order,
                });
            }
        }

        // resolve this intrinsic
        if first_segment_str.as_str() == "this" {
            let root_expr = Expression::This;
            if path.segments.len() == 1 {
                return Ok(root_expr);
            }
            return Ok(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // resolve super intrinsic
        if first_segment_str.as_str() == "super" {
            let root_expr = Expression::Super;
            if path.segments.len() == 1 {
                return Ok(root_expr);
            }
            return Ok(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // try to resolve root symbol locally
        let cache_key = ResolveAbsoluteSymbolCacheKey::new(
            module.id,
            scope.0,
            scope.2,
            first_segment,
            space_order,
        );
        let local_result = if let Some(local_id) = cache.absolute_symbol(cache_key) {
            Ok(local_id)
        } else {
            let local_result = {
                let scope_cache = cache.scope_indices();
                self.resolve_absolute_symbol(
                    module,
                    profile,
                    node,
                    scope,
                    StaticKey::Name(first_segment),
                    space_order,
                    symbols,
                    Some(scope_cache),
                )
            };
            if let Ok(local_id) = local_result {
                cache.insert_absolute_symbol(cache_key, local_id);
            }
            local_result
        };

        // if local lookup succeeded, use the local symbol
        if let Ok(local_id) = local_result {
            return self.resolve_local_path(
                module,
                profile,
                expression_id,
                node,
                local_id,
                path,
                static_arguments,
                space_order,
                symbols,
                tree,
                Some(cache.scope_indices()),
            );
        }

        // resolve commonjs runtime paths when local resolution failed
        if let Some(expression) = self.resolve_commonjs_runtime_path(
            module,
            profile,
            expression_id,
            path,
            static_arguments.clone(),
            space_order,
            tree,
        ) {
            return Ok(expression);
        }

        // resolve inherited associated type names from enclosing declaration heritage
        if space_order.spaces().contains(&SymbolSpace::Type)
            && let Some(associated_symbol) = self.resolve_heritage_associated_type_symbol(
                module,
                profile,
                node,
                expression_id,
                scope,
                first_segment,
                symbols,
                tree,
            )?
        {
            if path.segments.len() == 1 {
                if associated_symbol.module_id == module.id {
                    return Ok(self.resolve_symbol_to_expression(
                        module,
                        associated_symbol.local_id,
                        path,
                        static_arguments,
                        symbols,
                    ));
                }

                return Ok(Expression::GlobalReference {
                    path: path.clone(),
                    static_arguments,
                    target_symbol: associated_symbol,
                });
            }

            let root_path = Path {
                segments: vec![first_segment].into(),
            };
            let root_expr = if associated_symbol.module_id == module.id {
                self.resolve_symbol_to_expression(
                    module,
                    associated_symbol.local_id,
                    &root_path,
                    None,
                    symbols,
                )
            } else {
                Expression::GlobalReference {
                    path: root_path,
                    static_arguments: None,
                    target_symbol: associated_symbol,
                }
            };
            return Ok(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // check for builtin types (boolean, int, string, etc.)
        if let Some(ty) = self.resolve_string_to_type(first_segment_str.as_str()) {
            let root_expr = Expression::TypeLiteral { value: ty };
            if path.segments.len() == 1 {
                return Ok(root_expr);
            }
            // multi-segment paths like `int.MAX` become member chains
            return Ok(self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                static_arguments,
                tree,
            ));
        }

        // try module binding scope for global augmentations
        if let Some(module_scope_id) =
            self.module_binding_scope_for_global_expression(tree, expression_id)
        {
            let module_scope = symbols.get_scope_by_id(module_scope_id);
            let module_mark = LocalScopeMark::end();
            let module_result = {
                let scope_cache = cache.scope_indices();
                self.resolve_absolute_symbol(
                    module,
                    profile,
                    node,
                    (module_scope_id, module_scope, module_mark),
                    StaticKey::Name(first_segment),
                    space_order,
                    symbols,
                    Some(scope_cache),
                )
            };

            // return module binding symbols when present
            if let Ok(local_id) = module_result {
                return self.resolve_local_path(
                    module,
                    profile,
                    expression_id,
                    node,
                    local_id,
                    path,
                    static_arguments,
                    space_order,
                    symbols,
                    tree,
                    Some(cache.scope_indices()),
                );
            }
        }

        // try ambient namespace merges when inside a namespace
        if let Some(expr) = self.resolve_ambient_namespace_path(
            module,
            profile,
            expression_id,
            node,
            scope,
            path,
            static_arguments.clone(),
            space_order,
            symbols,
            tree,
            Some(cache.scope_indices()),
        )? {
            return Ok(expr);
        }

        // avoid prelude lookup inside the prelude module itself
        let is_prelude_module = self
            .program
            .builtins
            .as_ref()
            .is_some_and(|builtins| builtins.prelude_module_id == module.id);

        if !is_prelude_module
            && let Some(prelude_symbol) = self.resolve_prelude_symbol(first_segment, profile)?
        {
            // canonicalize prelude symbols to avoid alias identity mismatches
            let prelude_symbol =
                self.resolve_canonical_symbol_chain(profile, node, prelude_symbol)?;
            return self.resolve_prelude_path(
                module,
                expression_id,
                node,
                prelude_symbol,
                profile,
                path,
                static_arguments,
                space_order,
                tree,
            );
        }

        // resolve global symbols
        if let Some(expr) = self.resolve_global_path(
            module,
            expression_id,
            node,
            profile,
            path,
            static_arguments.clone(),
            space_order,
            Some(cache.scope_indices()),
            tree,
        )? {
            return Ok(expr);
        }

        // resolve ambient lib symbols
        if let Some(expr) = self.resolve_ambient_path(
            module,
            expression_id,
            node,
            profile,
            path,
            static_arguments,
            space_order,
            tree,
            Some(cache.scope_indices()),
        )? {
            return Ok(expr);
        }

        // neither local nor prelude found, return the original error
        local_result.map(|_| unreachable!())
    }

    /// Resolve a path starting from a local symbol.
    fn resolve_local_path(
        &self,
        module: &Module,
        profile_id: ProfileId,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        local_id: LocalSymbolId,
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
        scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<Expression> {
        let first_segment = path.first_segment().expect("path is empty");
        let remaining_segments = &path.segments[1..];

        // single-segment path: just return the resolved expression
        if remaining_segments.is_empty() {
            return Ok(self.resolve_symbol_to_expression(
                module,
                local_id,
                path,
                static_arguments,
                symbols,
            ));
        }

        // multi-segment path: check if first segment is a namespace
        let symbol = symbols.get_symbol(local_id);
        if symbol.kind == SymbolKind::Namespace {
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol_with_ambient_merge(
                module,
                profile_id,
                node,
                local_id,
                &remaining_path,
                space_order,
                symbols,
                scope_cache,
            ) {
                Ok((resolved_id, None)) => {
                    if resolved_id.module_id == module.id {
                        return Ok(self.resolve_symbol_to_expression(
                            module,
                            resolved_id.local_id,
                            path,
                            static_arguments,
                            symbols,
                        ));
                    }

                    return Ok(Expression::GlobalReference {
                        path: path.clone(),
                        static_arguments,
                        target_symbol: resolved_id,
                    });
                }
                Ok((resolved_id, Some(remaining))) => {
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = if resolved_id.module_id == module.id {
                        self.resolve_symbol_to_expression(
                            module,
                            resolved_id.local_id,
                            &resolved_path,
                            None,
                            symbols,
                        )
                    } else {
                        Expression::GlobalReference {
                            path: resolved_path,
                            static_arguments: None,
                            target_symbol: resolved_id,
                        }
                    };
                    return Ok(self.build_member_chain(
                        expression_id,
                        root_expr,
                        &remaining,
                        static_arguments,
                        tree,
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        // non-namespace symbol: remaining segments become Member chain
        let root_path = Path {
            segments: vec![first_segment].into(),
        };
        let root_expr =
            self.resolve_symbol_to_expression(module, local_id, &root_path, None, symbols);
        let remaining_path = path.slice(1..);
        Ok(self.build_member_chain(
            expression_id,
            root_expr,
            &remaining_path,
            static_arguments,
            tree,
        ))
    }
}
