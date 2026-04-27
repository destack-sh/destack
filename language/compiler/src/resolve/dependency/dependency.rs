use destack_artifact::{ExportedSymbolTable, ImportedModuleTable};
use destack_ast::StringId;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, GlobalNodeIdAny, GlobalSymbolId, ImportSource,
    LocalNodeId, LocalScopeId, LocalScopeMark, LocalSymbolId, ModuleTarget, Name, NamespaceExport,
    NodeTree, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::workspace::{Module, ProfileId};
use rustc_hash::FxHashSet;

use crate::common::dir::{SymbolDescriptor, can_merge_declarations};
use crate::resolve::binding::ResolveState;
use crate::resolve::dependency::cache::{ResolveDependencyItemCache, TargetCacheKey};
use crate::timing::tags;
use crate::{Compiler, ImportError, ResolveError, ResolveResult};

/// A resolved export symbol with its originating export space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ResolvedExportSymbol {
    /// The resolved symbol id.
    pub(super) symbol: GlobalSymbolId,
    /// The export space used to resolve the symbol.
    pub(super) export_space: SymbolSpace,
}

/// Track visited entries while walking reexport chains.
#[derive(Debug, Default)]
pub(super) struct ReexportVisitStack {
    /// The current path of visited entries.
    stack: Vec<ReexportVisitKey>,
    /// The set of visited entries for fast lookup.
    seen: FxHashSet<ReexportVisitKey>,
}

/// Key for reexport chain visitation.
pub(super) type ReexportVisitKey = (ModuleTarget, StaticKey, SymbolSpace);

impl ReexportVisitStack {
    /// Check whether a key has been visited.
    pub(super) fn contains(&self, key: &ReexportVisitKey) -> bool {
        self.seen.contains(key)
    }

    /// Record a new visit.
    pub(super) fn push(&mut self, key: ReexportVisitKey) {
        self.stack.push(key);
        self.seen.insert(key);
    }

    /// Drop the most recent visit.
    pub(super) fn pop(&mut self) {
        if let Some(key) = self.stack.pop() {
            self.seen.remove(&key);
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Select an origin module for module binding cache entries.
    pub(super) fn cache_origin_module_id(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
    ) -> Option<ModuleId> {
        match target {
            ModuleTarget::Binding(_) | ModuleTarget::External(_) => Some(origin_module_id),
            ModuleTarget::Module(_) => None,
        }
    }

    /// Build a cache key for a module target.
    pub(super) fn cache_target_key(
        &self,
        origin_module_id: ModuleId,
        target: ModuleTarget,
    ) -> TargetCacheKey {
        TargetCacheKey {
            target,
            origin_module_id: self.cache_origin_module_id(origin_module_id, target),
        }
    }

    /// Select the export spaces to consider for a dependency kind.
    pub(super) fn export_spaces_for_kind(&self, kind: DependencyKind) -> SymbolSpaceOrder {
        // prefer type space for type lookups
        if kind == DependencyKind::Type {
            return SymbolSpaceOrder::TypeThenValue;
        }

        // value lookups still need access to type-only exports for type positions
        SymbolSpaceOrder::ValueThenType
    }

    /// Get the import key for a reexported dependency item.
    pub(super) fn reexport_import_key(
        &self,
        mode: DependencyMode,
        name: Option<Name>,
        default_name: StringId,
    ) -> Option<StaticKey> {
        // map export modes to import keys
        match mode {
            DependencyMode::Item => name.map(|name| StaticKey::Name(name.string())),
            DependencyMode::Default => Some(StaticKey::Name(default_name)),
            DependencyMode::Namespace => None,
        }
    }

    /// Resolve a dependency item, optionally using a cache.
    pub(crate) fn resolve_dependency_item(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        imported_modules: &mut ImportedModuleTable,
        exported_symbols: &mut ExportedSymbolTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        namespace_exports: &mut Vec<NamespaceExport>,
        profile: ProfileId,
        item_id: LocalNodeId<DependencyItem>,
        mut cache: Option<&mut ResolveDependencyItemCache>,
    ) -> ResolveResult<Option<DependencyItem>> {
        let module_handle = module;

        // resolve the dependency item based on its mode
        let item = tree.get(item_id).clone();
        let resolved_item: DependencyItem = match item {
            DependencyItem::UnresolvedRemote {
                source,
                name,
                mode,
                kind,
                alias,
                target,
                target_module,
                symbol,
            } => {
                // capture the origin symbol for cycle reporting
                let origin_symbol = symbol.map(|symbol| symbol.into_global(module.id));

                // preserve any already-resolved target from the item or parent expression
                let item_target_module = target_module;
                let expression_target_module =
                    self.parent_expression_target_module_for_dependency_item(tree, item_id);
                let loader_override = self.parent_expression_loader_override_for_dependency_item(
                    module, profile, tree, item_id,
                )?;

                // resolve the target module or binding
                let remote_target = {
                    let _timing = self.timing_scope(tags::RESOLVE_DEPENDENCY_ITEM_IMPORT);
                    if let Some(target_module) = item_target_module
                        && let Some(remote_target) = target_module.for_kind(kind)
                    {
                        remote_target
                    } else if let Some(remote_target) = expression_target_module {
                        remote_target
                    } else {
                        let Some(remote_target) = self.resolve_import_maybe(
                            revision,
                            module_handle,
                            imported_modules,
                            profile,
                            item_id.into_global_any(module.id),
                            source,
                            target,
                            kind,
                            loader_override,
                        )?
                        else {
                            return Ok(None);
                        };
                        remote_target
                    }
                };

                // preserve the most specific target resolution we can recover
                let target_module = if let Some(target_module) = item_target_module {
                    target_module
                } else if let Some(remote_target) = expression_target_module {
                    destack_dir::ModuleResolution::from_target(remote_target)
                } else {
                    self.imported_module_resolution_for_specifier(
                        module.id,
                        profile,
                        Some(imported_modules),
                        target,
                        self.import_edge_kind(module_handle, source),
                        None,
                    )
                    .unwrap_or_else(|| destack_dir::ModuleResolution::from_target(remote_target))
                };
                let remote_symbol_target = self.select_symbol_target_for_dependency(
                    module_handle,
                    kind,
                    target_module,
                    remote_target,
                );

                // external package targets carry link metadata but no local symbol surface
                if matches!(remote_target, ModuleTarget::External(_)) {
                    return Ok(Some(DependencyItem::UnresolvedRemote {
                        source,
                        name,
                        mode,
                        kind,
                        alias,
                        target,
                        target_module: Some(target_module),
                        symbol,
                    }));
                }

                // resolve target symbol based on mode
                let (target_symbol, resolved_kind) = match mode {
                    DependencyMode::Item => {
                        // resolve a named import from the target
                        let key = name.map(|name| StaticKey::Name(name.string())).ok_or(
                            ResolveError::UnsupportedConstruct {
                                node: item_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            },
                        )?;
                        let (symbol, resolved_kind) = {
                            let _timing = self.timing_scope(tags::RESOLVE_DEPENDENCY_ITEM_SYMBOL);
                            self.resolve_remote_item_symbol_result(
                                revision,
                                module,
                                item_id.into_global_any(module.id),
                                remote_symbol_target,
                                None,
                                profile,
                                kind,
                                origin_symbol,
                                key,
                                Some(target),
                                cache.as_deref_mut(),
                            )?
                        };
                        (symbol, resolved_kind)
                    }
                    DependencyMode::Default => {
                        // resolve the default export from the target
                        let default_name = self.repository.strings.intern("default");
                        let key = StaticKey::Name(default_name);
                        let resolved = {
                            let _timing = self.timing_scope(tags::RESOLVE_DEPENDENCY_ITEM_SYMBOL);
                            self.resolve_remote_item_symbol_result(
                                revision,
                                module,
                                item_id.into_global_any(module.id),
                                remote_symbol_target,
                                self.fallback_target_for_default_dependency(
                                    kind,
                                    remote_symbol_target,
                                    remote_target,
                                ),
                                profile,
                                kind,
                                origin_symbol,
                                key,
                                Some(target),
                                cache.as_deref_mut(),
                            )
                        };

                        let (symbol, resolved_kind) = match resolved {
                            Ok(resolved) => resolved,
                            Err(error) => return Err(error),
                        };
                        (symbol, resolved_kind)
                    }
                    DependencyMode::Namespace => {
                        let _timing = self.timing_scope(tags::RESOLVE_DEPENDENCY_ITEM_NAMESPACE);

                        // prefer export assignment for import equals
                        if source == ImportSource::ImportEquals {
                            if let Some(symbol) = self.resolve_export_assignment_symbol(
                                revision,
                                module.id,
                                remote_symbol_target,
                                profile,
                            )? {
                                (symbol, kind)
                            } else {
                                let symbol = self.resolve_namespace_symbol(
                                    revision,
                                    module.id,
                                    item_id.into_global_any(module.id),
                                    remote_symbol_target,
                                    profile,
                                )?;
                                (symbol, kind)
                            }
                        } else {
                            // check for namespace exports without alias
                            if alias.is_none() && matches!(source, ImportSource::ExportStatement) {
                                // register `export * from` in module scope
                                let (item_scope_id, _) = tree.get_scope(item_id);
                                if item_scope_id == namespace_scope {
                                    namespace_exports.push(destack_dir::NamespaceExport {
                                        module_id: remote_symbol_target,
                                        kind,
                                        item: item_id,
                                    });
                                }
                            }
                            // resolve the namespace symbol
                            let symbol = self.resolve_namespace_symbol(
                                revision,
                                module.id,
                                item_id.into_global_any(module.id),
                                remote_symbol_target,
                                profile,
                            )?;
                            (symbol, kind)
                        }
                    }
                };

                DependencyItem::Remote {
                    mode,
                    kind: resolved_kind,
                    name,
                    alias,
                    target,
                    target_module,
                    symbol,
                    target_symbol,
                }
            }
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol,
            } => {
                // nothing to resolve for unnamed local exports (internal edge case)
                let Some(name_id) = name else {
                    return Ok(None);
                };

                // resolve the local symbol in scope
                let (scope_id, scope, mark) = symbols.get_scope(item_id, tree);
                let is_export_item = self.export_item_parent(tree, item_id).is_some();
                let mark = if is_export_item {
                    // export specifiers can reference later declarations
                    LocalScopeMark::end()
                } else {
                    mark
                };
                let space_order = self.export_spaces_for_kind(kind);
                let key = StaticKey::Name(name_id.string());
                let node = item_id.into_global_any(module.id);

                // resolve in local scope first
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
                    Some(tree),
                );
                let local_symbol_id = self.resolve_absolute_symbol(
                    pass,
                    (scope_id, scope, mark),
                    key,
                    cache.as_mut().map(|cache| cache.scope_indices()),
                );

                // fallback to global augmentation scope for types like AllowSharedBuffer
                // that are defined in `global { }` blocks within module declarations
                let target_symbol_id = match local_symbol_id {
                    Ok(symbol_id) => Ok(symbol_id),
                    Err(ResolveError::MissingSymbol { .. }) => {
                        let global_scope_id = global_augmentation_scope;
                        let global_scope = symbols.get_scope_by_id(global_scope_id);
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
                            Some(tree),
                        );

                        self.resolve_absolute_symbol(
                            pass,
                            (global_scope_id, global_scope, LocalScopeMark::end()),
                            key,
                            cache.as_mut().map(|cache| cache.scope_indices()),
                        )
                    }
                    Err(error) => Err(error),
                };

                // map unresolved local exports to an export-specific resolve error
                let target_symbol_id = match target_symbol_id {
                    Ok(target_symbol_id) => target_symbol_id,
                    Err(ResolveError::MissingSymbol { .. }) if is_export_item => {
                        return Err(ResolveError::MissingExportBinding {
                            node: node.into_anchored(Some(profile)),
                            name: name_id.string(),
                        });
                    }
                    Err(error) => return Err(error),
                };

                DependencyItem::Local {
                    mode,
                    kind,
                    name,
                    alias,
                    symbol,
                    target_symbol: target_symbol_id.into_global(module.id),
                }
            }

            _ => {
                // nothing to do
                return Ok(None);
            }
        };

        Ok(Some(resolved_item))
    }

    /// Report a conflicting export error if two symbols cannot merge.
    pub(super) fn check_can_merge_declarations(
        &self,
        revision: destack_workspace::Revision,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        module: ModuleId,
        name: Option<StaticKey>,
    ) {
        let Ok(left_module) = self.cache_module_snapshot(revision, left.module_id) else {
            return;
        };
        let left_module = left_module.as_ref();
        let left_dir = self.artifact_dir_base(left.module_id).unwrap_or_else(|| {
            panic!(
                "missing committed base dir artifact for {:?}",
                left.module_id
            )
        });
        let left_symbol = left_dir.symbols.get_symbol(left.local_id);

        let right_dir = self.artifact_dir_base(right.module_id).unwrap_or_else(|| {
            panic!(
                "missing committed base dir artifact for {:?}",
                right.module_id
            )
        });
        let right_symbol = right_dir.symbols.get_symbol(right.local_id);

        // check if the symbols can merge (e.g., interface + class)
        let language_type = left_module.language_type;
        let can_merge = can_merge_declarations(
            language_type,
            SymbolDescriptor::from(left_symbol),
            SymbolDescriptor::from(right_symbol),
        );

        // NOTE #Suspicious: should we separate export conflicts per profile? (from ImportError)
        // (also see other usages of ImportError::Conflicting* across resolve)
        if !can_merge {
            self.error(ImportError::ConflictingExport {
                node: node.into(),
                other_node: other_node.into(),
                module,
                name,
            });
        }
    }
}
