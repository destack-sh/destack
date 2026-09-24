use destack_artifact::DirView;
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};
use destack_source::{ModuleId, SourceIndex};

/// Builder for one module symbol index.
pub(crate) struct SymbolIndexer<'a> {
    /// The indexed module id.
    module_id: ModuleId,
    /// The visible expanded tree.
    view: dir::View<'a>,
    /// The visible expanded bindings.
    symbols: dir::BindingTable<'static>,
    /// The parsed source spans.
    source_index: &'a SourceIndex,
    /// The module namespace scope.
    namespace_scope: dir::LocalScopeId,
    /// The shared string pool.
    strings: &'a StringPool,
    /// The collected index entries.
    entries: Vec<dir::SymbolEntry>,
}

impl<'a> SymbolIndexer<'a> {
    /// Build the symbol index from expanded declarations.
    pub(crate) fn build(
        stages: &'a DirView,
        strings: &'a StringPool,
    ) -> ProviderResult<dir::SymbolIndex> {
        let parsed = &stages.parsed;
        let bound = &stages.bound;
        let symbols = stages.bindings().clone();
        let mut indexer = Self {
            module_id: symbols.module_id,
            view: stages.tree(),
            symbols,
            source_index: &parsed.tree.source_index,
            namespace_scope: bound.namespace_scope,
            strings,
            entries: Vec::new(),
        };

        // collect declaration symbols visible after expansion
        indexer.collect_symbols()?;

        Ok(dir::SymbolIndex::new(indexer.entries))
    }

    /// Collect symbol index entries.
    fn collect_symbols(&mut self) -> ProviderResult<()> {
        // collect expanded declaration symbols
        for (source, symbol_id) in self.symbols.declaration_symbols() {
            // keep only declarations owned by this module
            if source.module_id != self.module_id {
                continue;
            }

            // exclude imports and control labels from program declarations
            let symbol = self.symbols.get_symbol(symbol_id);
            if matches!(
                symbol.kind,
                dir::SymbolKind::Import | dir::SymbolKind::Label
            ) {
                continue;
            }
            if symbol.role == dir::SymbolRole::Local
                && symbol.scope.id != self.namespace_scope
                && !symbol.origin.is_global()
            {
                continue;
            }

            // skip declarations without a stable queried name
            let Some(name_id) = symbol.name() else {
                continue;
            };

            // emit one declaration entry
            if let Some(entry) = self.symbol_entry(source, symbol_id, name_id)? {
                self.entries.push(entry);
            }
        }

        Ok(())
    }

    /// Build one symbol index entry.
    fn symbol_entry(
        &self,
        source: dir::GlobalNodeIdAny,
        symbol_id: dir::LocalSymbolId,
        name_id: dir::StringId,
    ) -> ProviderResult<Option<dir::SymbolEntry>> {
        let symbol = self.symbols.get_symbol(symbol_id);
        let member_kind = self.member_kind(source.local_id)?;
        let source_id = self.view.get_source_any(source.local_id);

        // omit generated declarations without source positions
        if self.source_index.try_get(source_id).is_none() {
            return Ok(None);
        }

        // require the authored declaration and its name
        let span = self
            .view
            .get_span_by_id(source.local_id.id)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "indexed symbol has no declaration span: {source:?}"
                ))
            })?;
        let selection = self.source_index.get_main(source_id).ok_or_else(|| {
            ProviderError::internal(format!(
                "indexed symbol has no declaration name span: {source:?}"
            ))
        })?;

        // record the declaration entry
        let name = self.strings.get(name_id).to_string();
        let container = self.symbol_container_name(symbol_id);
        let global_symbol = symbol_id.into_global(self.module_id);

        Ok(Some(dir::SymbolEntry {
            name,
            kind: symbol.kind,
            member_kind,
            declaration: source.local_id,
            symbol: global_symbol,
            span,
            selection,
            container,
            mutability: symbol.binding_mutability,
        }))
    }

    /// Return the named lexical owner of one symbol.
    fn symbol_container_name(&self, symbol_id: dir::LocalSymbolId) -> Option<String> {
        let symbol = self.symbols.get_symbol(symbol_id);
        let scope = self.symbols.get_scope_by_id(symbol.scope.id);
        let owner_id = scope.owner?;
        let owner = self.symbols.get_symbol(owner_id);
        let name_id = owner.name()?;

        Some(self.strings.get(name_id).to_string())
    }

    /// Return the member kind for one declaration.
    fn member_kind(&self, node_id: dir::LocalNodeIdAny) -> ProviderResult<Option<dir::MemberKind>> {
        let kind = match node_id.ty {
            dir::NodeType::Member => {
                let node_id = node_id
                    .try_into_typed::<dir::Member>()
                    .map_err(ProviderError::internal)?;

                match self.view.get(node_id) {
                    dir::Member::AssociatedType { .. } => Some(dir::MemberKind::AssociatedType),
                    dir::Member::AssociatedConst { .. } => Some(dir::MemberKind::AssociatedConst),
                    dir::Member::Field { is_accessor, .. } => Some(if *is_accessor {
                        dir::MemberKind::Property
                    } else {
                        dir::MemberKind::Field
                    }),
                    dir::Member::Method {
                        signature,
                        is_accessor,
                        ..
                    } => Some(if *is_accessor {
                        dir::MemberKind::Property
                    } else {
                        match signature.role {
                            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New) => {
                                dir::MemberKind::Constructor
                            }
                            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter) => {
                                dir::MemberKind::Property
                            }
                            Some(dir::FunctionRole::Call) | None => dir::MemberKind::Method,
                        }
                    }),
                    dir::Member::StaticBlock { .. }
                    | dir::Member::ConstBlock { .. }
                    | dir::Member::Error => None,
                }
            }
            dir::NodeType::TypeMember => {
                let node_id = node_id
                    .try_into_typed::<dir::TypeMember>()
                    .map_err(ProviderError::internal)?;

                match self.view.get(node_id) {
                    dir::TypeMember::Field { .. } => Some(dir::MemberKind::Field),
                    dir::TypeMember::Method { .. } | dir::TypeMember::CallSignature { .. } => {
                        Some(dir::MemberKind::Method)
                    }
                    dir::TypeMember::AssociatedType { .. } => Some(dir::MemberKind::AssociatedType),
                    dir::TypeMember::AssociatedConst { .. } => {
                        Some(dir::MemberKind::AssociatedConst)
                    }
                    dir::TypeMember::ConstructSignature { .. } => {
                        Some(dir::MemberKind::Constructor)
                    }
                    dir::TypeMember::IndexSignature { .. } => Some(dir::MemberKind::Field),
                    dir::TypeMember::Error => None,
                }
            }
            dir::NodeType::EnumField => Some(dir::MemberKind::Variant),
            _ => None,
        };

        Ok(kind)
    }
}
