use std::sync::Arc;

use destack_artifact::ExportedSymbolTable;
use destack_builtin::BuiltinLibraryKind;
use destack_dir::{
    Declaration, Expression, GenericArgument, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId,
    LocalScopeId, LocalScopeMark, LocalSymbolId, Node, NodeTree, NodeType, Path, Scope, ScopeKind,
    StaticKey, StringId, SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::Revision;
use destack_workspace::workspace::{Module, ModuleSource, ProfileId};
use smallvec::SmallVec;

use crate::resolve::binding::cache::{
    ResolveAbsoluteSymbolCacheKey, ResolveExpressionCache, ResolveScopeIndexCache,
};
use crate::{Compiler, ResolveError, ResolveResult};

/// One resolved symbol target for a path segment prefix.
pub(crate) type ResolvedPathSymbolTargets = SmallVec<[GlobalSymbolId; 4]>;

/// Lift local path segment targets into one global target list.
fn lift_local_path_segment_targets(
    module_id: ModuleId,
    targets: SmallVec<[LocalSymbolId; 4]>,
) -> ResolvedPathSymbolTargets {
    let mut lifted = SmallVec::new();
    for target in targets {
        lifted.push(target.into_global(module_id));
    }

    lifted
}

/// Build one single-segment target list.
fn single_path_segment_target(symbol_id: GlobalSymbolId) -> ResolvedPathSymbolTargets {
    let mut targets = SmallVec::new();
    targets.push(symbol_id);
    targets
}

/// Shared resolve state for one unresolved path pass.
#[derive(Clone, Copy)]
pub(crate) struct ResolveState<'a> {
    /// The pinned revision for remote module reads.
    revision: Revision,
    /// The module being resolved.
    module: &'a Module,
    /// The current module namespace symbol when resolving inside the active module.
    current_namespace_symbol: Option<LocalSymbolId>,
    /// The current module namespace scope when resolving inside the active module.
    current_namespace_scope: Option<LocalScopeId>,
    /// The immutable artifact namespace symbol for remote module reads.
    artifact_namespace_symbol: Option<LocalSymbolId>,
    /// The immutable artifact namespace scope for remote module reads.
    artifact_namespace_scope: Option<LocalScopeId>,
    /// The active profile id.
    profile_id: ProfileId,
    /// The origin node used for diagnostics.
    node: GlobalNodeIdAny,
    /// The requested symbol-space resolution order.
    space_order: SymbolSpaceOrder,
    /// The active symbol table.
    symbols: &'a SymbolTable,
    /// The current-module tree when one local resolve pass needs syntax reads.
    current_tree: Option<&'a NodeTree>,
}

impl<'a> ResolveState<'a> {
    /// Build resolve state for one current-module pass.
    pub(crate) fn current(
        revision: Revision,
        module: &'a Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        space_order: SymbolSpaceOrder,
        symbols: &'a SymbolTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        _global_augmentation_scope: LocalScopeId,
        _exported_symbols: &'a indexmap::IndexMap<(SymbolSpace, StaticKey), destack_dir::Export>,
        tree: Option<&'a NodeTree>,
    ) -> Self {
        Self {
            revision,
            module,
            current_namespace_symbol: Some(namespace_symbol),
            current_namespace_scope: Some(namespace_scope),
            artifact_namespace_symbol: None,
            artifact_namespace_scope: None,
            profile_id,
            node,
            space_order,
            symbols,
            current_tree: tree,
        }
    }

    /// Build resolve state for one artifact-backed pass.
    pub(crate) fn artifact(
        revision: Revision,
        module: &'a Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        space_order: SymbolSpaceOrder,
        symbols: &'a SymbolTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        _global_augmentation_scope: LocalScopeId,
        _exported_symbols: &'a indexmap::IndexMap<(SymbolSpace, StaticKey), destack_dir::Export>,
        tree: Option<&'a NodeTree>,
    ) -> Self {
        Self {
            revision,
            module,
            current_namespace_symbol: None,
            current_namespace_scope: None,
            artifact_namespace_symbol: Some(namespace_symbol),
            artifact_namespace_scope: Some(namespace_scope),
            profile_id,
            node,
            space_order,
            symbols,
            current_tree: tree,
        }
    }

    /// Return the current namespace symbol.
    fn namespace_symbol(self) -> LocalSymbolId {
        self.current_namespace_symbol
            .or(self.artifact_namespace_symbol)
            .expect("resolve state is missing namespace symbol")
    }

    /// Return the current namespace scope.
    fn namespace_scope(self) -> LocalScopeId {
        self.current_namespace_scope
            .or(self.artifact_namespace_scope)
            .expect("resolve state is missing namespace scope")
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the target symbol for one bound path.
    pub(crate) fn resolve_path_target_symbol<T: Node>(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        exported_symbols: &ExportedSymbolTable,
        node_id: LocalNodeId<T>,
        path: &Path,
        space_order: SymbolSpaceOrder,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // bound paths should always have at least one segment
        let Some(first_segment) = path.first_segment() else {
            return Ok(None);
        };

        // resolve the root symbol in the local scope
        let key = StaticKey::Name(first_segment);
        let scope = symbols.get_scope(LocalNodeId::<T>::new(node_id.id), tree);
        let node_id_for_root = LocalNodeId::<T>::new(node_id.id);
        let node_id_for_library = LocalNodeId::<T>::new(node_id.id);
        let node_id_for_local_relative = LocalNodeId::<T>::new(node_id.id);
        let mut scope_cache = ResolveScopeIndexCache::default();
        let current_pass = ResolveState::current(
            revision,
            module,
            profile,
            node_id_for_root.into_global_any(module.id),
            space_order,
            symbols,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            exported_symbols,
            Some(tree),
        );
        let local_target =
            match self.resolve_absolute_symbol(current_pass, scope, key, Some(&mut scope_cache)) {
                Ok(symbol_id) => Some(symbol_id.into_global(module.id)),
                Err(ResolveError::MissingSymbol { .. }) => None,
                Err(error) => return Err(error),
            };

        // fall back to selected library symbols when the local scope misses
        let library_target = if local_target.is_none() {
            self.resolve_selected_lib_symbol(
                revision,
                module,
                profile,
                node_id_for_library.into_global_any(module.id),
                key,
                space_order,
                None,
            )?
        } else {
            None
        };
        let Some(mut target_symbol) = local_target.or(library_target) else {
            return Ok(None);
        };

        // stop after the root when the path has one segment
        if path.segments.len() == 1 {
            return Ok(Some(target_symbol));
        }

        // resolve namespace segments before switching to static member lookup
        let remaining_path = path.slice(1..);
        let resolved_relative = if target_symbol.module_id == module.id {
            let current_pass = ResolveState::current(
                revision,
                module,
                profile,
                node_id_for_local_relative.into_global_any(module.id),
                space_order,
                symbols,
                namespace_symbol,
                namespace_scope,
                global_augmentation_scope,
                exported_symbols,
                Some(tree),
            );

            self.resolve_relative_symbol_with_ambient_merge(
                current_pass,
                target_symbol.local_id,
                &remaining_path,
                Some(&mut scope_cache),
            )?
        } else {
            let target_module = self
                .cache_module_snapshot(revision, target_symbol.module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let target_dir =
                self.require_artifact_dir_prepared(revision, target_symbol.module_id, profile)?;
            let artifact_pass = ResolveState::artifact(
                revision,
                &target_module,
                profile,
                node_id.into_global_any(module.id),
                space_order,
                &target_dir.symbols,
                target_dir.namespace_symbol,
                target_dir.namespace_scope,
                target_dir.global_augmentation_scope,
                &target_dir.exported_symbols,
                Some(&target_dir.tree),
            );

            self.resolve_relative_symbol_with_ambient_merge(
                artifact_pass,
                target_symbol.local_id,
                &remaining_path,
                None,
            )?
        };
        target_symbol = resolved_relative.0;

        // walk the remaining static member segments on the resolved owner
        let Some(remaining_path) = resolved_relative.1 else {
            return Ok(Some(target_symbol));
        };
        for segment in remaining_path.segments.iter().copied() {
            let member_key = StaticKey::Name(segment);
            let Some(member_symbol) = self.query_static_member_symbol(
                revision,
                module,
                profile,
                target_symbol,
                member_key,
                tree,
                symbols,
            ) else {
                return Ok(None);
            };

            target_symbol = member_symbol;
        }

        Ok(Some(target_symbol))
    }

    /// Return true when one module should receive implicit prelude lookup.
    fn module_uses_prelude(&self, module: &Module) -> bool {
        match module.source {
            ModuleSource::User => true,
            ModuleSource::Builtin(BuiltinLibraryKind::Language | BuiltinLibraryKind::Library) => {
                true
            }
            ModuleSource::Builtin(BuiltinLibraryKind::Intrinsic) => false,
        }
    }

    /// Return whether namespace lookup may continue as value member access.
    fn namespace_lookup_may_continue_as_value_member_access(
        &self,
        module: &Module,
        space_order: SymbolSpaceOrder,
    ) -> bool {
        // namespace values are only modeled as runtime objects in JS and TS value space
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return false;
        }

        matches!(
            space_order,
            SymbolSpaceOrder::ValueOnly | SymbolSpaceOrder::ValueThenType
        )
    }

    /// Return the prelude module context and resolved DIR artifact for one profile.
    fn prelude_resolved_artifact(
        &self,
        revision: Revision,
        profile: ProfileId,
    ) -> ResolveResult<Option<(Arc<Module>, Arc<destack_artifact::DirResolved>)>> {
        // check if prelude injection is enabled
        if !self.options.inject_prelude {
            return Ok(None);
        }

        // get the prelude module ID from builtins
        let builtins = self.repository.builtins.as_ref();
        let prelude_module_id = builtins.prelude_module_id();

        // prelude symbols can come from reexports, so require resolved exports
        let prelude_context = self
            .cache_module_snapshot(revision, prelude_module_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?;
        let prelude_dir = self
            .require_artifact_dir_resolved(revision, prelude_module_id, profile)
            .map_err(ResolveError::from)?;

        Ok(Some((prelude_context, prelude_dir)))
    }

    /// Resolve CommonJS runtime paths (`module` and `exports`) for CommonJS modules.
    fn resolve_commonjs_runtime_path(
        &self,
        pass: ResolveState<'_>,
        expression_id: LocalNodeId<Expression>,
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        tree: &mut NodeTree,
    ) -> Option<Expression> {
        // only value space lookups can resolve runtime commonjs names
        if !pass.space_order.spaces().contains(&SymbolSpace::Value) {
            return None;
        }

        // only commonjs modules expose these runtime bindings by default
        if !pass.module.module_format.is_commonjs() {
            return None;
        }

        // only handle top-level runtime roots
        let first_segment = path.first_segment()?;
        let first_segment_str = self.repository.strings.get(first_segment);
        let is_module_root = first_segment_str.as_str() == "module";
        let is_exports_root = first_segment_str.as_str() == "exports";
        let is_self_root = first_segment_str.as_str() == "self";
        let is_define_root = first_segment_str.as_str() == "define";
        if !is_module_root && !is_exports_root && !is_self_root && !is_define_root {
            return None;
        }

        // resolve both roots against the current module namespace symbol
        let namespace_symbol = pass.namespace_symbol().into_global(pass.module.id);

        // map `module` directly to the runtime module object
        if is_module_root {
            let root_path = Path {
                segments: vec![first_segment].into(),
            };
            let root_expression = Expression::GlobalReference {
                path: root_path,
                generic_arguments: Vec::new(),
                target_symbol: namespace_symbol,
            };

            if path.segments.len() == 1 {
                return Some(Expression::GlobalReference {
                    path: path.clone(),
                    generic_arguments: generic_arguments.clone().unwrap_or_default(),
                    target_symbol: namespace_symbol,
                });
            }

            return Some(self.build_member_chain(
                expression_id,
                root_expression,
                &path.slice(1..),
                generic_arguments.clone(),
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
                generic_arguments: Vec::new(),
                target_symbol: namespace_symbol,
            };

            if path.segments.len() == 1 {
                return Some(Expression::GlobalReference {
                    path: root_path,
                    generic_arguments: generic_arguments.clone().unwrap_or_default(),
                    target_symbol: namespace_symbol,
                });
            }

            return Some(self.build_member_chain(
                expression_id,
                root_expression,
                &path.slice(1..),
                generic_arguments.clone(),
                tree,
            ));
        }

        // map `exports` to the commonjs runtime alias binding
        let exports_name = self.repository.strings.intern("exports");
        let exports_path = Path {
            segments: vec![exports_name].into(),
        };
        let exports_expression = Expression::GlobalReference {
            path: exports_path.clone(),
            generic_arguments: Vec::new(),
            target_symbol: namespace_symbol,
        };

        if path.segments.len() == 1 {
            return Some(Expression::GlobalReference {
                path: exports_path,
                generic_arguments: generic_arguments.clone().unwrap_or_default(),
                target_symbol: namespace_symbol,
            });
        }

        Some(self.build_member_chain(
            expression_id,
            exports_expression,
            &path.slice(1..),
            generic_arguments,
            tree,
        ))
    }

    /// Resolve an inherited associated type name from an enclosing declaration heritage.
    fn resolve_heritage_associated_type_symbol(
        &self,
        pass: ResolveState<'_>,
        expression_id: LocalNodeId<Expression>,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        member_name: StringId,
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
                        pass.symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            };

            let owner_symbol = pass.symbols.get_symbol(owner_id);
            let Some(primary_declaration) = owner_symbol.primary_declaration else {
                if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                    current_scope = (
                        parent_scope_id,
                        pass.symbols.get_scope_by_id(parent_scope_id),
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
                        pass.symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            }

            let declaration_id = primary_declaration.local_id.into_typed::<Declaration>();
            let declaration = tree.get(declaration_id);
            let heritage_symbols: Vec<GlobalSymbolId> = match declaration {
                Declaration::Struct(declaration) => declaration
                    .implements_types
                    .iter()
                    .chain(declaration.embedded_types.iter())
                    .filter_map(|type_id| tree.get(*type_id).target_symbol())
                    .collect(),
                Declaration::Class(declaration) => declaration
                    .implements_types
                    .iter()
                    .filter_map(|type_id| tree.get(*type_id).target_symbol())
                    .collect(),
                Declaration::Enum(declaration) => declaration
                    .implements_types
                    .iter()
                    .filter_map(|type_id| tree.get(*type_id).target_symbol())
                    .collect(),
                Declaration::Interface(declaration) => declaration
                    .extends
                    .iter()
                    .filter_map(|heritage| tree.get(heritage.expression).target_symbol())
                    .collect(),
                Declaration::Extension(declaration) => declaration
                    .implements_types
                    .iter()
                    .filter_map(|type_id| tree.get(*type_id).target_symbol())
                    .collect(),
                _ => Vec::new(),
            };
            if heritage_symbols.is_empty() {
                if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                    current_scope = (
                        parent_scope_id,
                        pass.symbols.get_scope_by_id(parent_scope_id),
                        parent_mark,
                    );
                    continue;
                }
                return Ok(None);
            }

            // try heritage targets in declaration order
            for heritage_symbol in &heritage_symbols {
                let module = self
                    .cache_module_snapshot(pass.revision, pass.module.id)
                    .map_err(|error| ResolveError::Internal {
                        message: format!("failed to load module snapshot: {error}"),
                    })?;
                let module = module.as_ref();
                let resolved_member = self.resolve_static_member_symbol(
                    pass.revision,
                    module,
                    pass.profile_id,
                    expression_id,
                    *heritage_symbol,
                    member_key,
                    tree,
                    pass.symbols,
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

    /// Resolve an absolute symbol from one pass state.
    pub(crate) fn resolve_absolute_symbol(
        &self,
        pass: ResolveState<'_>,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        key: StaticKey,
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
                pass.space_order,
                pass.symbols,
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
                pass.module,
                scope.1,
                key,
                pass.space_order,
                pass.symbols,
                scope.2,
            ) {
                return Ok(hoisted_symbol);
            }

            // allow same-scope forward references for JS/TS bindings
            if self.allow_forward_binding_lookup(pass.module, pass.space_order) {
                let (forward_preferred, _) =
                    self.find_symbol_in_scope(scope.1, key, pass.space_order, pass.symbols, None);
                if let Some(forward_symbol) = forward_preferred {
                    return Ok(forward_symbol);
                }
            }

            // move to the parent scope when available
            if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    pass.symbols.get_scope_by_id(parent_scope_id),
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
            node: pass.node.into_anchored(Some(pass.profile_id)),
            scope: scope.0.into_global(pass.module.id),
            via_module: None,
            key,
        })
    }

    /// Resolve a path against merged ambient namespace scopes when inside a namespace.
    fn resolve_ambient_namespace_path(
        &self,
        pass: ResolveState<'_>,
        expression_id: LocalNodeId<Expression>,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        tree: &mut NodeTree,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<Option<(Expression, ResolvedPathSymbolTargets)>> {
        if !self.is_selected_library_module(pass.profile_id, pass.module.id) {
            return Ok(None);
        }

        let mut scope = scope;
        loop {
            if scope.1.kind == ScopeKind::Namespace
                && let Some(owner_id) = scope.1.owner_id
            {
                match self.resolve_relative_symbol_with_ambient_merge(
                    ResolveState {
                        current_tree: Some(&*tree),
                        ..pass
                    },
                    owner_id,
                    path,
                    scope_cache.as_deref_mut(),
                ) {
                    Ok((resolved_id, None, resolved_targets)) => {
                        if resolved_id.module_id == pass.module.id {
                            return Ok(Some((
                                self.resolve_symbol_to_expression(
                                    pass.module,
                                    resolved_id.local_id,
                                    path,
                                    generic_arguments.clone(),
                                    pass.symbols,
                                ),
                                resolved_targets,
                            )));
                        }

                        return Ok(Some((
                            Expression::GlobalReference {
                                path: path.clone(),
                                generic_arguments: generic_arguments.clone().unwrap_or_default(),
                                target_symbol: resolved_id,
                            },
                            resolved_targets,
                        )));
                    }
                    Ok((resolved_id, Some(remaining), resolved_targets)) => {
                        if remaining.segments.len() < path.segments.len() {
                            let resolved_path =
                                path.slice(0..path.segments.len() - remaining.segments.len());
                            let root_expr = if resolved_id.module_id == pass.module.id {
                                self.resolve_symbol_to_expression(
                                    pass.module,
                                    resolved_id.local_id,
                                    &resolved_path,
                                    None,
                                    pass.symbols,
                                )
                            } else {
                                Expression::GlobalReference {
                                    path: resolved_path,
                                    generic_arguments: Vec::new(),
                                    target_symbol: resolved_id,
                                }
                            };
                            return Ok(Some((
                                self.build_member_chain(
                                    expression_id,
                                    root_expr,
                                    &remaining,
                                    generic_arguments.clone(),
                                    tree,
                                ),
                                resolved_targets,
                            )));
                        }
                    }
                    Err(ResolveError::MissingSymbol { .. }) => {}
                    Err(error) => return Err(error),
                }
            }

            if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    pass.symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            } else {
                break;
            }
        }

        Ok(None)
    }

    /// Resolve selected lib merge sources for one symbol key and space order.
    fn resolve_selected_lib_symbol_sources_for_space_order(
        &self,
        pass: ResolveState<'_>,
        profile_id: ProfileId,
        key: StaticKey,
        order: SymbolSpaceOrder,
    ) -> ResolveResult<Option<Vec<GlobalSymbolId>>> {
        self.require_selected_library_environment(pass.revision, profile_id)
            .map_err(ResolveError::from)?;

        if let Some(sources) =
            self.get_library_symbol_sources_for_space_order(profile_id, key, order)
        {
            return Ok(Some(sources));
        }

        let selected_library_modules = self.selected_library_modules(profile_id);
        if selected_library_modules.is_empty() {
            return Ok(None);
        }

        Ok(None)
    }

    /// Resolve a symbol from the prelude by name.
    pub(crate) fn resolve_prelude_symbol(
        &self,
        revision: Revision,
        name: StringId,
        profile: ProfileId,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        // builtin environment is the primary source of implicit language item symbols
        self.require_language_environment(revision, profile)?;
        if let Some(symbol_id) = self.get_builtin_symbol(profile, name) {
            return Ok(Some(symbol_id));
        }

        // the prelude module can also contribute reexported names
        let Some((prelude_context, prelude_dir)) =
            self.prelude_resolved_artifact(revision, profile)?
        else {
            return Ok(None);
        };

        // look up symbol by name in prelude's export table
        let key = StaticKey::Name(name);
        let export_spaces = SymbolSpaceOrder::ValueThenType;
        let Some(symbol_id) = self.resolve_exported_symbol(
            &prelude_context,
            profile,
            &prelude_dir.exported_symbols,
            &prelude_dir.tree,
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
        pass: ResolveState<'_>,
        expression_id: LocalNodeId<Expression>,
        prelude_symbol: GlobalSymbolId,
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        tree: &mut NodeTree,
    ) -> ResolveResult<(Expression, ResolvedPathSymbolTargets)> {
        let remaining_segments = &path.segments[1..];
        let receiver_targets = single_path_segment_target(prelude_symbol);

        // single-segment path: just return the GlobalReference
        if remaining_segments.is_empty() {
            return Ok((
                Expression::GlobalReference {
                    path: path.clone(),
                    generic_arguments: generic_arguments.clone().unwrap_or_default(),
                    target_symbol: prelude_symbol,
                },
                receiver_targets,
            ));
        }

        // multi-segment path: need to check if prelude symbol is a namespace
        let Some((prelude_context, prelude_dir)) =
            self.prelude_resolved_artifact(pass.revision, pass.profile_id)?
        else {
            return Err(ResolveError::MissingSymbol {
                node: pass.node.into_anchored(Some(pass.profile_id)),
                scope: pass.namespace_scope().into_global(pass.module.id),
                via_module: None,
                key: StaticKey::Name(path.segments[0]),
            });
        };
        // canonicalized prelude symbols may be defined in another module
        let (target_context, target_dir) = if prelude_symbol.module_id == prelude_context.id {
            (prelude_context, prelude_dir)
        } else {
            let target_context = self
                .cache_module_snapshot(pass.revision, prelude_symbol.module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let target_dir = self
                .require_artifact_dir_resolved(
                    pass.revision,
                    prelude_symbol.module_id,
                    pass.profile_id,
                )
                .map_err(ResolveError::from)?;
            (target_context, target_dir)
        };
        let target_symbols = &target_dir.symbols;

        let local_symbol_id = prelude_symbol.local_id;
        let symbol = target_symbols.get_symbol(local_symbol_id);

        if symbol.kind == SymbolKind::Namespace {
            // resolve remaining path within the prelude module's namespace
            let remaining_path = path.slice(1..);
            let prelude_pass = ResolveState::artifact(
                pass.revision,
                &target_context,
                pass.profile_id,
                pass.node,
                pass.space_order,
                target_symbols,
                target_dir.namespace_symbol,
                target_dir.namespace_scope,
                target_dir.global_augmentation_scope,
                &target_dir.exported_symbols,
                Some(&target_dir.tree),
            );
            match self.resolve_relative_symbol_with_ambient_merge(
                prelude_pass,
                local_symbol_id,
                &remaining_path,
                None,
            ) {
                Ok((resolved_id, None, resolved_targets)) => {
                    // fully resolved within prelude
                    return Ok((
                        Expression::GlobalReference {
                            path: path.clone(),
                            generic_arguments: generic_arguments.clone().unwrap_or_default(),
                            target_symbol: resolved_id,
                        },
                        resolved_targets,
                    ));
                }
                Ok((resolved_id, Some(remaining), resolved_targets)) => {
                    // partially resolved, build Member chain for remaining
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = Expression::GlobalReference {
                        path: resolved_path,
                        generic_arguments: Vec::new(),
                        target_symbol: resolved_id,
                    };
                    return Ok((
                        self.build_member_chain(
                            expression_id,
                            root_expr,
                            &remaining,
                            generic_arguments.clone(),
                            tree,
                        ),
                        resolved_targets,
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
            generic_arguments: Vec::new(),
            target_symbol: prelude_symbol,
        };
        Ok((
            self.build_member_chain(
                expression_id,
                root_expr,
                &path.slice(1..),
                generic_arguments,
                tree,
            ),
            receiver_targets,
        ))
    }

    /// Resolve a path from ambient lib modules, if available.
    fn resolve_ambient_path(
        &self,
        pass: ResolveState<'_>,
        expression_id: LocalNodeId<Expression>,
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        tree: &mut NodeTree,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<Option<(Expression, ResolvedPathSymbolTargets)>> {
        let selected_library_modules = self.selected_library_modules(pass.profile_id);
        if selected_library_modules.is_empty() {
            return Ok(None);
        }

        self.require_selected_library_environment(pass.revision, pass.profile_id)
            .map_err(ResolveError::from)?;

        let first_segment = path.first_segment().expect("path is empty");
        let key = StaticKey::Name(first_segment);

        // prefer cached declared lib symbols when available
        if let Some(symbol_id) =
            self.get_declared_library_symbol_from(pass.profile_id, first_segment, pass.space_order)
        {
            let is_current_module = symbol_id.module_id == pass.module.id;
            let ambient_artifact = if is_current_module {
                None
            } else {
                Some(self.require_artifact_dir_prepared(
                    pass.revision,
                    symbol_id.module_id,
                    pass.profile_id,
                )?)
            };
            let ambient_module_owner = if is_current_module {
                None
            } else {
                Some(
                    self.cache_module_snapshot(pass.revision, symbol_id.module_id)
                        .map_err(|error| ResolveError::Internal {
                            message: format!("failed to load module snapshot: {error}"),
                        })?,
                )
            };
            let ambient_module = ambient_module_owner.as_deref().unwrap_or(pass.module);
            let symbols = ambient_artifact
                .as_ref()
                .map_or(pass.symbols, |dir| &dir.symbols);
            let symbol = symbols.get_symbol(symbol_id.local_id);
            let global_this_name = self.repository.strings.intern("globalThis");
            let matches_space = symbol.space == SymbolSpace::TypeValue
                || pass.space_order.spaces().contains(&symbol.space)
                || (symbol.space == SymbolSpace::Value && first_segment == global_this_name);
            if matches_space {
                if path.segments.len() == 1 {
                    return Ok(Some((
                        Expression::GlobalReference {
                            path: path.clone(),
                            generic_arguments: generic_arguments.clone().unwrap_or_default(),
                            target_symbol: symbol_id,
                        },
                        single_path_segment_target(symbol_id),
                    )));
                }

                if symbol.kind == SymbolKind::Namespace {
                    let remaining_path = path.slice(1..);
                    let mut ambient_pass = if let Some(dir) = ambient_artifact.as_ref() {
                        ResolveState::artifact(
                            pass.revision,
                            ambient_module,
                            pass.profile_id,
                            pass.node,
                            pass.space_order,
                            symbols,
                            dir.namespace_symbol,
                            dir.namespace_scope,
                            dir.global_augmentation_scope,
                            &dir.exported_symbols,
                            Some(&dir.tree),
                        )
                    } else {
                        pass
                    };
                    ambient_pass.current_tree = ambient_artifact
                        .as_ref()
                        .map(|dir| &*dir.tree)
                        .or(pass.current_tree)
                        .or(Some(&*tree));
                    match self.resolve_relative_symbol_with_ambient_merge(
                        ambient_pass,
                        symbol_id.local_id,
                        &remaining_path,
                        scope_cache.as_deref_mut(),
                    ) {
                        Ok((resolved_id, None, resolved_targets)) => {
                            return Ok(Some((
                                Expression::GlobalReference {
                                    path: path.clone(),
                                    generic_arguments: generic_arguments
                                        .clone()
                                        .unwrap_or_default(),
                                    target_symbol: resolved_id,
                                },
                                resolved_targets,
                            )));
                        }
                        Ok((resolved_id, Some(remaining), resolved_targets)) => {
                            let resolved_path =
                                path.slice(0..path.segments.len() - remaining.segments.len());
                            let root_expr = Expression::GlobalReference {
                                path: resolved_path,
                                generic_arguments: Vec::new(),
                                target_symbol: resolved_id,
                            };
                            return Ok(Some((
                                self.build_member_chain(
                                    expression_id,
                                    root_expr,
                                    &remaining,
                                    generic_arguments.clone(),
                                    tree,
                                ),
                                resolved_targets,
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
                    generic_arguments: Vec::new(),
                    target_symbol: symbol_id,
                };
                return Ok(Some((
                    self.build_member_chain(
                        expression_id,
                        root_expr,
                        &path.slice(1..),
                        generic_arguments.clone(),
                        tree,
                    ),
                    single_path_segment_target(symbol_id),
                )));
            }
        }

        // search selected lib namespace scopes in order
        for module_id in selected_library_modules.iter().copied() {
            // skip self
            if module_id == pass.module.id {
                continue;
            }

            // read the ambient module's symbols
            let ambient_context = self
                .cache_module_snapshot(pass.revision, module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let ambient_dir = self
                .require_artifact_dir_prepared(pass.revision, module_id, pass.profile_id)
                .map_err(ResolveError::from)?;
            let symbols = &ambient_dir.symbols;
            let ambient_pass = ResolveState::artifact(
                pass.revision,
                &ambient_context,
                pass.profile_id,
                pass.node,
                pass.space_order,
                symbols,
                ambient_dir.namespace_symbol,
                ambient_dir.namespace_scope,
                ambient_dir.global_augmentation_scope,
                &ambient_dir.exported_symbols,
                Some(&ambient_dir.tree),
            );

            // find symbol in the ambient module namespace scope first
            let namespace_scope = symbols.get_scope_by_id(ambient_dir.namespace_scope);
            let symbol_id = self.resolve_absolute_symbol(
                ambient_pass,
                (
                    ambient_dir.namespace_scope,
                    namespace_scope,
                    LocalScopeMark::end(),
                ),
                key,
                scope_cache.as_deref_mut(),
            );
            let symbol_id = match symbol_id {
                Ok(symbol_id) => symbol_id,
                Err(ResolveError::MissingSymbol { .. }) => {
                    // otherwise fall back to ambient global augmentation scope
                    let global_scope =
                        symbols.get_scope_by_id(ambient_dir.global_augmentation_scope);
                    match self.resolve_absolute_symbol(
                        ambient_pass,
                        (
                            ambient_dir.global_augmentation_scope,
                            global_scope,
                            LocalScopeMark::end(),
                        ),
                        key,
                        scope_cache.as_deref_mut(),
                    ) {
                        Ok(symbol_id) => symbol_id,
                        Err(ResolveError::MissingSymbol { .. }) => continue,
                        Err(error) => return Err(error),
                    }
                }
                Err(error) => return Err(error),
            };

            // single-segment path: just return the GlobalReference
            if path.segments.len() == 1 {
                return Ok(Some((
                    Expression::GlobalReference {
                        path: path.clone(),
                        generic_arguments: generic_arguments.clone().unwrap_or_default(),
                        target_symbol: symbol_id.into_global(module_id),
                    },
                    single_path_segment_target(symbol_id.into_global(module_id)),
                )));
            }

            // multi-segment path: resolve in nested namespace scope
            let symbol = symbols.get_symbol(symbol_id);
            if symbol.kind == SymbolKind::Namespace {
                let remaining_path = path.slice(1..);
                match self.resolve_relative_symbol_with_ambient_merge(
                    ambient_pass,
                    symbol_id,
                    &remaining_path,
                    scope_cache.as_deref_mut(),
                ) {
                    Ok((resolved_id, None, resolved_targets)) => {
                        return Ok(Some((
                            Expression::GlobalReference {
                                path: path.clone(),
                                generic_arguments: generic_arguments.clone().unwrap_or_default(),
                                target_symbol: resolved_id,
                            },
                            resolved_targets,
                        )));
                    }
                    Ok((resolved_id, Some(remaining), resolved_targets)) => {
                        let resolved_path =
                            path.slice(0..path.segments.len() - remaining.segments.len());
                        let root_expr = Expression::GlobalReference {
                            path: resolved_path,
                            generic_arguments: Vec::new(),
                            target_symbol: resolved_id,
                        };
                        return Ok(Some((
                            self.build_member_chain(
                                expression_id,
                                root_expr,
                                &remaining,
                                generic_arguments.clone(),
                                tree,
                            ),
                            resolved_targets,
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
                generic_arguments: Vec::new(),
                target_symbol: symbol_id.into_global(module_id),
            };
            return Ok(Some((
                self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(1..),
                    generic_arguments.clone(),
                    tree,
                ),
                single_path_segment_target(symbol_id.into_global(module_id)),
            )));
        }

        Ok(None)
    }

    /// Resolve one top-level symbol from the selected lib modules for a profile.
    pub(crate) fn resolve_selected_lib_symbol(
        &self,
        revision: Revision,
        module: &Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        key: StaticKey,
        space_order: SymbolSpaceOrder,
        _scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<Option<GlobalSymbolId>> {
        let _ = node;

        if module.is_builtin() {
            return Ok(None);
        }

        self.require_selected_library_environment(revision, profile_id)
            .map_err(ResolveError::from)?;

        let StaticKey::Name(name) = key else {
            return Ok(None);
        };

        Ok(self.get_library_symbol_from(profile_id, name, space_order))
    }

    /// Resolve a relative symbol with ambient namespace merge sources.
    pub(crate) fn resolve_relative_symbol_with_ambient_merge(
        &self,
        pass: ResolveState<'_>,
        symbol_id: LocalSymbolId,
        path: &Path,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<(GlobalSymbolId, Option<Path>, ResolvedPathSymbolTargets)> {
        // try resolving within the current module first
        let resolved =
            self.resolve_relative_symbol(pass, symbol_id, path, scope_cache.as_deref_mut());
        let missing = match resolved {
            Ok((resolved_id, remaining, traversed_targets)) => {
                return Ok((
                    resolved_id.into_global(pass.module.id),
                    remaining,
                    lift_local_path_segment_targets(pass.module.id, traversed_targets),
                ));
            }
            Err(error @ ResolveError::MissingSymbol { .. }) => error,
            Err(error) => return Err(error),
        };

        // stop if the module is not in the selected library environment
        if !self.is_selected_library_module(pass.profile_id, pass.module.id) {
            return Err(missing);
        }

        // gather selected lib merge sources for the symbol key
        let symbol_entry = pass.symbols.get_symbol(symbol_id);
        let Some(key) = symbol_entry.key else {
            return Err(missing);
        };
        let Some(lib_sources) = self.resolve_selected_lib_symbol_sources_for_space_order(
            pass,
            pass.profile_id,
            key,
            pass.space_order,
        )?
        else {
            return Err(missing);
        };

        // search lib sources for a matching path
        for source_symbol in lib_sources {
            // avoid re locking the same module while holding its symbols lock
            if source_symbol.module_id == pass.module.id {
                // skip the original symbol, then try resolving within this module scope
                if source_symbol.local_id == symbol_id {
                    continue;
                }

                // resolve using the existing symbols table
                match self.resolve_relative_symbol(
                    pass,
                    source_symbol.local_id,
                    path,
                    scope_cache.as_deref_mut(),
                ) {
                    Ok((resolved_id, remaining, traversed_targets)) => {
                        return Ok((
                            resolved_id.into_global(source_symbol.module_id),
                            remaining,
                            lift_local_path_segment_targets(
                                source_symbol.module_id,
                                traversed_targets,
                            ),
                        ));
                    }
                    Err(ResolveError::MissingSymbol { .. }) => {}
                    Err(error) => return Err(error),
                }
                continue;
            }

            // prepare and read the source module before resolving
            let source_context = self
                .cache_module_snapshot(pass.revision, source_symbol.module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;
            let source_dir = self
                .require_artifact_dir_prepared(
                    pass.revision,
                    source_symbol.module_id,
                    pass.profile_id,
                )
                .map_err(ResolveError::from)?;
            let source_symbols = &source_dir.symbols;
            let source_pass = ResolveState::artifact(
                pass.revision,
                &source_context,
                pass.profile_id,
                pass.node,
                pass.space_order,
                source_symbols,
                source_dir.namespace_symbol,
                source_dir.namespace_scope,
                source_dir.global_augmentation_scope,
                &source_dir.exported_symbols,
                Some(&source_dir.tree),
            );

            // resolve using the source module symbols table
            match self.resolve_relative_symbol(
                source_pass,
                source_symbol.local_id,
                path,
                scope_cache.as_deref_mut(),
            ) {
                Ok((resolved_id, remaining, traversed_targets)) => {
                    return Ok((
                        resolved_id.into_global(source_symbol.module_id),
                        remaining,
                        lift_local_path_segment_targets(source_symbol.module_id, traversed_targets),
                    ));
                }
                Err(ResolveError::MissingSymbol { .. }) => {}
                Err(error) => return Err(error),
            }
        }

        Err(missing)
    }

    /// Resolve a relative symbol from one pass state.
    fn resolve_relative_symbol(
        &self,
        pass: ResolveState<'_>,
        symbol_id: LocalSymbolId,
        path: &Path,
        mut scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<(LocalSymbolId, Option<Path>, SmallVec<[LocalSymbolId; 4]>)> {
        // track the current symbol as we walk segments
        let mut current_symbol_id = symbol_id;
        let mut traversed_targets = SmallVec::new();
        traversed_targets.push(symbol_id);
        let segments = &path.segments;

        // walk the path segments
        for (i, &segment) in segments.iter().enumerate() {
            let symbol = pass.symbols.get_symbol(current_symbol_id);

            // stop traversing when the symbol is not a namespace
            if symbol.kind != SymbolKind::Namespace {
                let remaining = path.slice(i..);
                return Ok((current_symbol_id, Some(remaining), traversed_targets));
            }

            // resolve the next segment in the namespace scope
            let key = StaticKey::Name(segment);
            let scope = pass.symbols.get_scope_by_id(symbol.scope.0);
            let (preferred, fallback) = self.find_symbol_in_scope_cached(
                symbol.scope.0,
                scope,
                key,
                pass.space_order,
                pass.symbols,
                LocalScopeMark::end(),
                scope_cache.as_deref_mut(),
            );

            // advance to the next symbol when possible
            if let Some(symbol_id) = preferred.or(fallback) {
                current_symbol_id = symbol_id;
                traversed_targets.push(symbol_id);
                continue;
            }

            // allow merged symbols to satisfy value or type paths
            if let Some(group_id) = symbol.merge_group {
                let group_symbols = pass.symbols.merge_group_symbols(group_id);
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

                for requested_space in pass.space_order.spaces() {
                    let candidate = group_symbols.iter().copied().find(|group_symbol| {
                        let merged_symbol = pass.symbols.get_symbol(*group_symbol);
                        merged_symbol.kind != SymbolKind::Namespace
                            && match_space(*requested_space, merged_symbol.space)
                    });
                    if let Some(group_symbol) = candidate {
                        let remaining = path.slice(i..);
                        return Ok((group_symbol, Some(remaining), traversed_targets));
                    }
                }
            }

            // preserve the remaining suffix as value member access
            if self
                .namespace_lookup_may_continue_as_value_member_access(pass.module, pass.space_order)
            {
                let remaining = path.slice(i..);
                return Ok((current_symbol_id, Some(remaining), traversed_targets));
            }

            // report a missing symbol in the namespace scope
            return Err(ResolveError::MissingSymbol {
                node: pass.node.into_anchored(Some(pass.profile_id)),
                scope: symbol.scope.0.into_global(pass.module.id),
                via_module: None,
                key,
            });
        }

        Ok((current_symbol_id, None, traversed_targets))
    }

    /// Resolve an absolute path.
    /// For non-namespace symbols with remaining path segments, creates Member expression chains.
    /// Falls back to prelude lookup if local lookup fails and inject_prelude is enabled.
    pub(crate) fn resolve_absolute_path(
        &self,
        revision: Revision,
        module: &Module,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        exported_symbols: &indexmap::IndexMap<(SymbolSpace, StaticKey), destack_dir::Export>,
        expression_id: LocalNodeId<Expression>,
        node: GlobalNodeIdAny,
        profile: ProfileId,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        tree: &mut NodeTree,
        cache: &mut ResolveExpressionCache,
    ) -> ResolveResult<(Expression, ResolvedPathSymbolTargets)> {
        let pass = ResolveState::current(
            revision,
            module,
            profile,
            node,
            space_order,
            symbols,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            exported_symbols,
            None,
        );
        let first_segment = path.first_segment().expect("path is empty in {node:?}");
        let first_segment_str = self.repository.strings.get(first_segment);
        let scope_mark = if module.language_type.is_declaration() {
            LocalScopeMark::end()
        } else {
            scope.2
        };
        let scope = (scope.0, scope.1, scope_mark);

        // resolve import.meta intrinsic
        if first_segment_str.as_str() == "import" && path.segments.len() >= 2 {
            let second_segment = path.segments[1];
            let second_segment_str = self.repository.strings.get(second_segment);
            if second_segment_str.as_str() == "meta" {
                let root_expr = Expression::ImportMeta;
                if path.segments.len() == 2 {
                    return Ok((root_expr, ResolvedPathSymbolTargets::new()));
                }
                return Ok((
                    self.build_member_chain(
                        expression_id,
                        root_expr,
                        &path.slice(2..),
                        generic_arguments.clone(),
                        tree,
                    ),
                    ResolvedPathSymbolTargets::new(),
                ));
            }
        }

        // keep `new.target` unresolved so validate can enforce lexical context rules
        if first_segment_str.as_str() == "new" && path.segments.len() >= 2 {
            let second_segment = path.segments[1];
            let second_segment_str = self.repository.strings.get(second_segment);
            if second_segment_str.as_str() == "target" {
                let expression = Expression::UnresolvedPath {
                    path: path.clone(),
                    generic_arguments: generic_arguments.clone().unwrap_or_default(),
                    space_order,
                };

                return Ok((expression, ResolvedPathSymbolTargets::new()));
            }
        }

        // resolve this intrinsic
        if first_segment_str.as_str() == "this" {
            let root_expr = Expression::This;
            if path.segments.len() == 1 {
                return Ok((root_expr, ResolvedPathSymbolTargets::new()));
            }
            return Ok((
                self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(1..),
                    generic_arguments.clone(),
                    tree,
                ),
                ResolvedPathSymbolTargets::new(),
            ));
        }

        // resolve super intrinsic
        if first_segment_str.as_str() == "super" {
            let root_expr = Expression::Super;
            if path.segments.len() == 1 {
                return Ok((root_expr, ResolvedPathSymbolTargets::new()));
            }
            return Ok((
                self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(1..),
                    generic_arguments.clone(),
                    tree,
                ),
                ResolvedPathSymbolTargets::new(),
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
                let pass = ResolveState {
                    current_tree: Some(&*tree),
                    ..pass
                };
                self.resolve_absolute_symbol(
                    pass,
                    scope,
                    StaticKey::Name(first_segment),
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
                pass,
                expression_id,
                local_id,
                path,
                generic_arguments.clone(),
                tree,
                Some(cache.scope_indices()),
            );
        }

        // resolve commonjs runtime paths when local resolution failed
        if let Some(expression) = self.resolve_commonjs_runtime_path(
            pass,
            expression_id,
            path,
            generic_arguments.clone(),
            tree,
        ) {
            return Ok((expression, ResolvedPathSymbolTargets::new()));
        }

        // resolve inherited associated type names from enclosing declaration heritage
        if space_order.spaces().contains(&SymbolSpace::Type)
            && let Some(associated_symbol) = self.resolve_heritage_associated_type_symbol(
                pass,
                expression_id,
                scope,
                first_segment,
                tree,
            )?
        {
            if path.segments.len() == 1 {
                if associated_symbol.module_id == module.id {
                    return Ok((
                        self.resolve_symbol_to_expression(
                            module,
                            associated_symbol.local_id,
                            path,
                            generic_arguments.clone(),
                            symbols,
                        ),
                        single_path_segment_target(associated_symbol),
                    ));
                }

                return Ok((
                    Expression::GlobalReference {
                        path: path.clone(),
                        generic_arguments: generic_arguments.clone().unwrap_or_default(),
                        target_symbol: associated_symbol,
                    },
                    single_path_segment_target(associated_symbol),
                ));
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
                    generic_arguments: Vec::new(),
                    target_symbol: associated_symbol,
                }
            };
            return Ok((
                self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(1..),
                    generic_arguments.clone(),
                    tree,
                ),
                single_path_segment_target(associated_symbol),
            ));
        }

        // check for builtin types (boolean, int, string, etc.)
        if let Some(ty) = self.resolve_string_to_type(first_segment_str.as_str()) {
            let root_expr = Expression::TypeLiteral { value: ty };
            if path.segments.len() == 1 {
                return Ok((root_expr, ResolvedPathSymbolTargets::new()));
            }
            // multi-segment paths like `int.MAX` become member chains
            return Ok((
                self.build_member_chain(
                    expression_id,
                    root_expr,
                    &path.slice(1..),
                    generic_arguments.clone(),
                    tree,
                ),
                ResolvedPathSymbolTargets::new(),
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
                let pass = ResolveState {
                    current_tree: Some(&*tree),
                    ..pass
                };
                self.resolve_absolute_symbol(
                    pass,
                    (module_scope_id, module_scope, module_mark),
                    StaticKey::Name(first_segment),
                    Some(scope_cache),
                )
            };

            // return module binding symbols when present
            if let Ok(local_id) = module_result {
                return self.resolve_local_path(
                    pass,
                    expression_id,
                    local_id,
                    path,
                    generic_arguments.clone(),
                    tree,
                    Some(cache.scope_indices()),
                );
            }
        }

        // try ambient namespace merges when inside a namespace
        if let Some(resolved) = self.resolve_ambient_namespace_path(
            pass,
            expression_id,
            scope,
            path,
            generic_arguments.clone(),
            tree,
            Some(cache.scope_indices()),
        )? {
            return Ok(resolved);
        }

        // resolve global symbols
        if let Some(resolved) = self.resolve_global_path(
            revision,
            module,
            expression_id,
            node,
            profile,
            path,
            generic_arguments.clone(),
            space_order,
            Some(cache.scope_indices()),
            tree,
        )? {
            return Ok(resolved);
        }

        // prelude injection applies to modules that consume the builtin environment
        if self.module_uses_prelude(module)
            && let Some(prelude_symbol) =
                self.resolve_prelude_symbol(revision, first_segment, profile)?
        {
            // canonicalize prelude symbols to avoid alias identity mismatches
            let prelude_symbol = self.resolve_canonical_symbol_chain(
                revision,
                profile,
                node,
                prelude_symbol,
                pass.symbols,
            )?;
            return self.resolve_prelude_path(
                pass,
                expression_id,
                prelude_symbol,
                path,
                generic_arguments.clone(),
                tree,
            );
        }

        // resolve ambient lib symbols
        if let Some(resolved) = self.resolve_ambient_path(
            pass,
            expression_id,
            path,
            generic_arguments,
            tree,
            Some(cache.scope_indices()),
        )? {
            return Ok(resolved);
        }

        // neither local nor prelude found, return the original error
        local_result.map(|_| unreachable!())
    }

    /// Resolve a path starting from a local symbol.
    fn resolve_local_path(
        &self,
        pass: ResolveState<'_>,
        expression_id: LocalNodeId<Expression>,
        local_id: LocalSymbolId,
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        tree: &mut NodeTree,
        scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> ResolveResult<(Expression, ResolvedPathSymbolTargets)> {
        let first_segment = path.first_segment().expect("path is empty");
        let remaining_segments = &path.segments[1..];
        let receiver_targets = single_path_segment_target(local_id.into_global(pass.module.id));

        // single-segment path: just return the resolved expression
        if remaining_segments.is_empty() {
            return Ok((
                self.resolve_symbol_to_expression(
                    pass.module,
                    local_id,
                    path,
                    generic_arguments.clone(),
                    pass.symbols,
                ),
                receiver_targets,
            ));
        }

        // multi-segment path: check if first segment is a namespace
        let symbol = pass.symbols.get_symbol(local_id);
        if symbol.kind == SymbolKind::Namespace {
            let remaining_path = path.slice(1..);
            match self.resolve_relative_symbol_with_ambient_merge(
                ResolveState {
                    current_tree: Some(&*tree),
                    ..pass
                },
                local_id,
                &remaining_path,
                scope_cache,
            ) {
                Ok((resolved_id, None, resolved_targets)) => {
                    if resolved_id.module_id == pass.module.id {
                        return Ok((
                            self.resolve_symbol_to_expression(
                                pass.module,
                                resolved_id.local_id,
                                path,
                                generic_arguments.clone(),
                                pass.symbols,
                            ),
                            resolved_targets,
                        ));
                    }

                    return Ok((
                        Expression::GlobalReference {
                            path: path.clone(),
                            generic_arguments: generic_arguments.clone().unwrap_or_default(),
                            target_symbol: resolved_id,
                        },
                        resolved_targets,
                    ));
                }
                Ok((resolved_id, Some(remaining), resolved_targets)) => {
                    let resolved_path =
                        path.slice(0..path.segments.len() - remaining.segments.len());
                    let root_expr = if resolved_id.module_id == pass.module.id {
                        self.resolve_symbol_to_expression(
                            pass.module,
                            resolved_id.local_id,
                            &resolved_path,
                            None,
                            pass.symbols,
                        )
                    } else {
                        Expression::GlobalReference {
                            path: resolved_path,
                            generic_arguments: Vec::new(),
                            target_symbol: resolved_id,
                        }
                    };
                    return Ok((
                        self.build_member_chain(
                            expression_id,
                            root_expr,
                            &remaining,
                            generic_arguments.clone(),
                            tree,
                        ),
                        resolved_targets,
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        // non-namespace symbol: remaining segments become Member chain
        let root_path = Path {
            segments: vec![first_segment].into(),
        };
        let root_expr = self.resolve_symbol_to_expression(
            pass.module,
            local_id,
            &root_path,
            None,
            pass.symbols,
        );
        let remaining_path = path.slice(1..);
        Ok((
            self.build_member_chain(
                expression_id,
                root_expr,
                &remaining_path,
                generic_arguments,
                tree,
            ),
            receiver_targets,
        ))
    }
}
