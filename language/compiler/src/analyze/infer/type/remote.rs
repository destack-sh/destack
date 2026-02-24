use super::*;
use destack_dir::{AnchoredGlobalNodeId, DependencyItem, NodeType, Symbol, SymbolSpace};

/// Select the cross-module read domain used for remote value type resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteValueTypeReadDomain {
    /// Resolve against provisional declare-owned commitments for interface fixed-point solving.
    Surface,
    /// Resolve against committed interface-published commitments.
    Interface,
}

/// One lookup result while resolving remote value types across alias and export links.
#[derive(Debug, Default)]
struct RemoteValueTypeLookupResult {
    /// The imported local type id, when one resolvable remote type was found.
    imported_type_id: Option<LocalTypeId>,
    /// Symbols that should be continued in the outer cross-module queue.
    forwarded_symbols: Vec<GlobalSymbolId>,
    /// Whether one interface-published value symbol was observed on this lookup path.
    saw_interface_published_symbol: bool,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve one remote value type for the current infer context ownership mode.
    pub(crate) fn resolve_remote_symbol_value_type_for_context(
        &self,
        module: &Module,
        ctx: &InferContext,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if ctx.is_surface_inference {
            self.resolve_remote_symbol_value_type_for_surface(
                module,
                ctx.profile,
                node_id,
                target_symbol,
                types,
            )
        } else {
            self.resolve_remote_symbol_value_type_for_interface(
                module,
                ctx.profile,
                node_id,
                target_symbol,
                types,
            )
        }
    }

    /// Resolve one remote value type from interface-published commitments.
    pub(crate) fn resolve_remote_symbol_value_type_for_interface(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        self.resolve_remote_symbol_value_type_in_domain(
            module,
            profile,
            node_id,
            target_symbol,
            RemoteValueTypeReadDomain::Interface,
            types,
        )
    }

    /// Resolve one remote value type from declare-surface commitments.
    pub(crate) fn resolve_remote_symbol_value_type_for_surface(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        self.resolve_remote_symbol_value_type_in_domain(
            module,
            profile,
            node_id,
            target_symbol,
            RemoteValueTypeReadDomain::Surface,
            types,
        )
    }

    /// Resolve a declared value type id for a symbol from remote declaration commitments.
    fn query_remote_declared_value_type_id(
        &self,
        remote_module: &Module,
        target_symbol: GlobalSymbolId,
        remote_tree: &NodeTree,
        remote_symbols: &SymbolTable,
        remote_types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer direct binding declarator declarations
        if let Some(declarator_id) = self.direct_binding_declarator_for_symbol(
            remote_module,
            target_symbol,
            remote_tree,
            remote_symbols,
        ) {
            let declarator_global = declarator_id.into_global_any(remote_module.id);
            if let Some(declared_type_id) = remote_types.get_declared_type_id(declarator_global) {
                return Some(declared_type_id);
            }
        }

        // otherwise fall back to the primary declaration type
        let primary_declaration = remote_symbols
            .get_symbol(target_symbol.local_id)
            .primary_declaration?;
        remote_types.get_declared_type_id(primary_declaration)
    }

    /// Resolve a remote symbol value type with explicit read-domain ownership.
    fn resolve_remote_symbol_value_type_in_domain(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        read_domain: RemoteValueTypeReadDomain,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let error_node = node_id.into_global(module.id).into_anchored(Some(profile));
        let mut pending_symbols = vec![target_symbol];
        let mut visited_symbols = HashSet::new();
        let mut saw_interface_published_symbol = false;

        while let Some(candidate_symbol) = pending_symbols.pop() {
            if !visited_symbols.insert(candidate_symbol) {
                continue;
            }

            let read_stage = match read_domain {
                RemoteValueTypeReadDomain::Surface => AnalyzeDependencyStage::Declare,
                RemoteValueTypeReadDomain::Interface => AnalyzeDependencyStage::Interface,
            };

            let lookup = self
                .with_module_tree_symbols_at_stage(
                    module,
                    profile,
                    candidate_symbol.module_id,
                    read_stage,
                    |remote_module,
                     remote_tree,
                     remote_symbols|
                     -> AnalyzeResult<RemoteValueTypeLookupResult> {
                        let remote_types = remote_module.dir(profile).types.read();
                        self.query_remote_symbol_value_type_in_snapshot(
                            profile,
                            node_id,
                            error_node,
                            candidate_symbol,
                            read_domain,
                            remote_module,
                            remote_tree,
                            remote_symbols,
                            &remote_types,
                            types,
                        )
                    },
                )
                .map_err(AnalyzeError::from)??;
            if let Some(imported_type_id) = lookup.imported_type_id {
                return Ok(imported_type_id);
            }
            if lookup.saw_interface_published_symbol {
                saw_interface_published_symbol = true;
            }

            for forwarded_symbol in lookup.forwarded_symbols {
                if !visited_symbols.contains(&forwarded_symbol) {
                    pending_symbols.push(forwarded_symbol);
                }
            }
        }

        self.resolve_missing_remote_value_type(
            module,
            node_id,
            target_symbol,
            read_domain,
            saw_interface_published_symbol,
            types,
        )
    }

    /// Query one remote value type in one remote module snapshot.
    fn query_remote_symbol_value_type_in_snapshot(
        &self,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        error_node: AnchoredGlobalNodeId,
        candidate_symbol: GlobalSymbolId,
        read_domain: RemoteValueTypeReadDomain,
        remote_module: &Module,
        remote_tree: &NodeTree,
        remote_symbols: &SymbolTable,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<RemoteValueTypeLookupResult> {
        let mut lookup = RemoteValueTypeLookupResult::default();
        let mut local_pending_symbols = vec![candidate_symbol];
        let mut local_visited_symbols = HashSet::new();

        while let Some(local_symbol) = local_pending_symbols.pop() {
            if !local_visited_symbols.insert(local_symbol) {
                continue;
            }

            if local_symbol.module_id != remote_module.id {
                lookup.forwarded_symbols.push(local_symbol);
                continue;
            }

            let local_entry = remote_symbols.get_symbol(local_symbol.local_id);
            let resolved_symbol = GlobalSymbolId::new(
                remote_module.id,
                local_symbol.local_id.with_type(local_entry.ty),
            );
            let is_value_capable = self.symbol_entry_is_value_capable(local_entry);

            if is_value_capable {
                let is_interface_published_value = self.remote_symbol_is_interface_published_value(
                    remote_module,
                    profile,
                    remote_symbols,
                    resolved_symbol,
                );
                if read_domain == RemoteValueTypeReadDomain::Interface
                    && is_interface_published_value
                    && resolved_symbol.ty() != SymbolType::Void
                {
                    lookup.saw_interface_published_symbol = true;
                }

                let remote_type_id = match read_domain {
                    RemoteValueTypeReadDomain::Surface => self.query_remote_surface_value_type_id(
                        remote_module,
                        resolved_symbol,
                        remote_tree,
                        remote_symbols,
                        remote_types,
                    ),
                    RemoteValueTypeReadDomain::Interface => {
                        if is_interface_published_value && resolved_symbol.ty() != SymbolType::Void
                        {
                            Some(self.require_remote_interface_value_type_id(
                                resolved_symbol,
                                remote_symbols,
                                remote_types,
                            )?)
                        } else {
                            self.query_remote_surface_value_type_id(
                                remote_module,
                                resolved_symbol,
                                remote_tree,
                                remote_symbols,
                                remote_types,
                            )
                        }
                    }
                };
                if let Some(remote_type_id) = remote_type_id {
                    lookup.imported_type_id = Some(self.import_remote_type_for_node(
                        node_id,
                        remote_types.get_type(remote_type_id),
                        remote_types,
                        types,
                    ));
                    lookup.forwarded_symbols.clear();
                    return Ok(lookup);
                }
            }

            let pending_count_before = local_pending_symbols.len();
            self.extend_remote_value_linked_symbols(
                &mut local_pending_symbols,
                local_symbol,
                local_entry,
                remote_tree,
            );
            let has_forward_links = local_pending_symbols.len() > pending_count_before;
            if !is_value_capable && !has_forward_links {
                return Err(AnalyzeError::TypeOnlyValue { node: error_node });
            }
        }

        Ok(lookup)
    }

    /// Finalize one unresolved remote read at the selected read domain.
    fn resolve_missing_remote_value_type(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        read_domain: RemoteValueTypeReadDomain,
        saw_interface_published_symbol: bool,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // surface reads participate in interface fixed-point convergence:
        // unresolved remote value commitments are modeled as semantic unknown
        if read_domain == RemoteValueTypeReadDomain::Surface {
            return Ok(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                node_id,
            ));
        }

        // interface reads for non-published value symbols can remain indeterminate:
        // model this as semantic unknown instead of an internal error
        if read_domain == RemoteValueTypeReadDomain::Interface && !saw_interface_published_symbol {
            return Ok(types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                node_id,
            ));
        }

        let read_domain_name = match read_domain {
            RemoteValueTypeReadDomain::Surface => "surface",
            RemoteValueTypeReadDomain::Interface => "interface",
        };

        Err(AnalyzeError::Internal {
            message: format!(
                "missing remote value type for {read_domain_name} read: local_module={:?}, remote_module={:?}, symbol={target_symbol:?}",
                module.id, target_symbol.module_id,
            ),
        })
    }

    /// Extend one pending symbol set with alias and export links for one symbol entry.
    fn extend_remote_value_linked_symbols(
        &self,
        pending_symbols: &mut Vec<GlobalSymbolId>,
        symbol: GlobalSymbolId,
        symbol_entry: &Symbol,
        tree: &NodeTree,
    ) {
        if let Some(target_symbol) = symbol_entry.target_symbol {
            pending_symbols.push(target_symbol);
        }
        if let Some(canonical_symbol) = symbol_entry.canonical_symbol {
            pending_symbols.push(canonical_symbol);
        }

        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return;
        };

        if primary_declaration.local_id.ty == NodeType::DependencyItem {
            let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
            if let DependencyItem::Local { target_symbol, .. }
            | DependencyItem::Remote { target_symbol, .. } = tree.get(dependency_id)
            {
                pending_symbols.push(*target_symbol);
            }
        }

        if primary_declaration.local_id.ty == NodeType::Expression {
            let expression_id = primary_declaration.local_id.into_typed::<Expression>();
            if let Expression::Export { items, .. } = tree.get(expression_id) {
                for item_id in items {
                    match tree.get(*item_id) {
                        DependencyItem::Local {
                            symbol: Some(local_symbol),
                            target_symbol,
                            ..
                        }
                        | DependencyItem::Remote {
                            symbol: Some(local_symbol),
                            target_symbol,
                            ..
                        } if *local_symbol == symbol.local_id => {
                            pending_symbols.push(*target_symbol);
                        }
                        // export-star items can still carry the symbol path for one re-exported name
                        DependencyItem::Local {
                            symbol: None,
                            target_symbol,
                            ..
                        }
                        | DependencyItem::Remote {
                            symbol: None,
                            target_symbol,
                            ..
                        } => {
                            pending_symbols.push(*target_symbol);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Return true when one symbol table entry can be used as a value.
    fn symbol_entry_is_value_capable(&self, symbol: &Symbol) -> bool {
        if symbol.space == SymbolSpace::Value {
            return true;
        }

        if matches!(
            symbol.ty,
            SymbolType::Struct
                | SymbolType::Class
                | SymbolType::Enum
                | SymbolType::Newtype
                | SymbolType::Function
        ) {
            return true;
        }

        if symbol.space == SymbolSpace::TypeValue {
            return !matches!(
                symbol.ty,
                SymbolType::Interface | SymbolType::TypeAlias | SymbolType::Extension
            );
        }

        false
    }

    /// Query one remote value type id for interface surface convergence.
    fn query_remote_surface_value_type_id(
        &self,
        remote_module: &Module,
        target_symbol: GlobalSymbolId,
        remote_tree: &NodeTree,
        remote_symbols: &SymbolTable,
        remote_types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer value types already assigned by interface or declare
        if let Some(value_type_id) = remote_types.get_value_type_id(target_symbol) {
            return Some(value_type_id);
        }

        // normalize the symbol entry and follow direct alias links
        if target_symbol.module_id == remote_module.id {
            let symbol_entry = remote_symbols.get_symbol(target_symbol.local_id);
            let normalized_symbol = GlobalSymbolId::new(
                remote_module.id,
                target_symbol.local_id.with_type(symbol_entry.ty),
            );

            if let Some(value_type_id) = remote_types.get_value_type_id(normalized_symbol) {
                return Some(value_type_id);
            }
            if let Some(target_symbol) = symbol_entry.target_symbol
                && let Some(value_type_id) = remote_types.get_value_type_id(target_symbol)
            {
                return Some(value_type_id);
            }
            if let Some(canonical_symbol) = symbol_entry.canonical_symbol
                && let Some(value_type_id) = remote_types.get_value_type_id(canonical_symbol)
            {
                return Some(value_type_id);
            }
        }

        // then use declared binding annotations
        if let Some(declared_type_id) = self.query_remote_declared_value_type_id(
            remote_module,
            target_symbol,
            remote_tree,
            remote_symbols,
            remote_types,
        ) {
            return Some(declared_type_id);
        }

        // finally use declared signature commitments
        let primary_declaration = remote_symbols
            .get_symbol(target_symbol.local_id)
            .primary_declaration?;
        if primary_declaration.module_id != remote_module.id {
            return None;
        }

        remote_types.get_signature_type_for_node(primary_declaration)
    }

    /// Return true when a symbol is one published interface value of the remote module.
    fn remote_symbol_is_interface_published_value(
        &self,
        remote_module: &Module,
        profile: ProfileId,
        remote_symbols: &SymbolTable,
        target_symbol: GlobalSymbolId,
    ) -> bool {
        let dir = remote_module.dir(profile);
        let exported_symbols = dir.exported_symbols.read();
        for export in exported_symbols.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(remote_symbols, remote_module.id, export)
            else {
                continue;
            };
            let export_matches = export_symbol.module_id == target_symbol.module_id
                && export_symbol.local_id == target_symbol.local_id;
            let value_matches = value_symbol.module_id == target_symbol.module_id
                && value_symbol.local_id == target_symbol.local_id;
            if export_matches || value_matches {
                return true;
            }
        }

        let binding_exports = dir.module_binding_exports.read();
        for binding in binding_exports.values() {
            for export in binding.exports.values() {
                let Some((export_symbol, value_symbol)) = self.interface_value_symbol_for_export(
                    remote_symbols,
                    remote_module.id,
                    export,
                ) else {
                    continue;
                };
                let export_matches = export_symbol.module_id == target_symbol.module_id
                    && export_symbol.local_id == target_symbol.local_id;
                let value_matches = value_symbol.module_id == target_symbol.module_id
                    && value_symbol.local_id == target_symbol.local_id;
                if export_matches || value_matches {
                    return true;
                }
            }
        }

        false
    }

    /// Require one published interface value type id for a remote symbol.
    fn require_remote_interface_value_type_id(
        &self,
        target_symbol: GlobalSymbolId,
        remote_symbols: &SymbolTable,
        remote_types: &TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // interface reads are fail-closed: imported values must have one published boundary type
        if let Some(value_type_id) = remote_types.get_value_type_id(target_symbol) {
            return Ok(value_type_id);
        }

        if target_symbol.module_id == remote_symbols.module_id {
            let symbol_entry = remote_symbols.get_symbol(target_symbol.local_id);
            let normalized_symbol = GlobalSymbolId::new(
                target_symbol.module_id,
                target_symbol.local_id.with_type(symbol_entry.ty),
            );
            if let Some(value_type_id) = remote_types.get_value_type_id(normalized_symbol) {
                return Ok(value_type_id);
            }

            if let Some(target_symbol) = symbol_entry.target_symbol
                && let Some(value_type_id) = remote_types.get_value_type_id(target_symbol)
            {
                return Ok(value_type_id);
            }

            if let Some(canonical_symbol) = symbol_entry.canonical_symbol
                && let Some(value_type_id) = remote_types.get_value_type_id(canonical_symbol)
            {
                return Ok(value_type_id);
            }
        }

        Err(AnalyzeError::Internal {
            message: format!(
                "missing published interface value type: remote_module={:?}, symbol={target_symbol:?}",
                target_symbol.module_id,
            ),
        })
    }
}
