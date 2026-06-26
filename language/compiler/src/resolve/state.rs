use destack_artifact::{DiagnosticAnchor, DirResolved};
use destack_core::{StringPool, closest_string};
use destack_dir as dir;
use destack_repository::ArtifactReader;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexSet;
use smallvec::{SmallVec, smallvec};

use crate::export::{ExportLookup, ExportResolver};
use crate::resolve::stats::ResolveStats;
use crate::{CompilerResult, ResolveError, diagnostic_suggestion_distance};

/// Resolve phase state for one module.
pub(in crate::resolve) struct ResolveState<'a> {
    /// Revision-bound artifact reader.
    pub(in crate::resolve) artifacts: ArtifactReader<'a>,
    /// The current module.
    pub(in crate::resolve) module: ModuleId,
    /// The expanded DIR view.
    pub(in crate::resolve) view: dir::View<'a>,
    /// The expanded binding table.
    pub(in crate::resolve) bindings: dir::BindingTable<'static>,
    /// The expanded module table.
    pub(in crate::resolve) modules: dir::ModuleTable<'static>,
    /// The shared string pool.
    pub(in crate::resolve) strings: &'a StringPool,
    /// The import table being built.
    pub(in crate::resolve) imports: dir::ImportTable,
    /// The reference resolutions being built.
    pub(in crate::resolve) references: dir::ReferenceTable,
    /// The recoverable diagnostics produced while resolving.
    pub(in crate::resolve) diagnostics: Vec<ResolveError>,
    /// The work stats accumulated while resolving.
    pub(in crate::resolve) stats: ResolveStats,
    /// Import and re-export clauses collected from active roots.
    pub(in crate::resolve) module_clauses: Vec<ModuleClause>,
    /// Namespace path references collected from active roots.
    pub(in crate::resolve) path_references: Vec<PathReference>,
    /// Nesting depth within a member chain, so only its outermost member collects.
    pub(in crate::resolve) member_chain_depth: usize,
    /// Source-visible global keys referenced by active roots.
    pub(in crate::resolve) global_keys: IndexSet<dir::StaticKey>,
    /// Language items used by active roots without source imports.
    pub(in crate::resolve) language_item_uses: IndexSet<dir::LanguageItem>,
    /// Function contexts visible while walking active roots.
    pub(in crate::resolve) function_stack: Vec<FunctionContext>,
    /// The memoized export lookups shared by this provider run.
    pub(in crate::resolve) exports: ExportResolver,
    /// The DIR visitor options.
    pub(in crate::resolve) options: dir::NodeVisitorOptions,
}

/// One import or re-export clause to resolve.
#[derive(Debug, Clone)]
pub(in crate::resolve) enum ModuleClause {
    /// An import declaration.
    Import {
        /// The expression node id.
        expression_id: dir::LocalNodeId<dir::Expression>,
        /// Imported items.
        items: Option<Vec<dir::LocalNodeId<dir::DependencyItem>>>,
    },
    /// A re-export declaration.
    ReExport {
        /// The expression node id.
        expression_id: dir::LocalNodeId<dir::Expression>,
        /// Re-exported items.
        items: Vec<dir::LocalNodeId<dir::DependencyItem>>,
    },
}

/// One namespace path reference to resolve after imports are known.
#[derive(Debug, Clone)]
pub(in crate::resolve) struct PathReference {
    /// The source node that owns the path.
    pub(in crate::resolve) source: dir::GlobalNodeIdAny,
    /// The source path.
    pub(in crate::resolve) path: dir::Path,
    /// The symbol space used by the first segment.
    pub(in crate::resolve) space: dir::SymbolSpace,
}

/// Function context used to choose async and generator language items.
#[derive(Debug, Clone, Copy)]
pub(in crate::resolve) struct FunctionContext {
    /// The function asynchrony.
    pub(in crate::resolve) asynchrony: dir::Asynchrony,
    /// Whether the function is a generator.
    pub(in crate::resolve) is_generator: bool,
}

impl<'a> ResolveState<'a> {
    /// Create resolve state for one module.
    pub(in crate::resolve) fn new(
        artifacts: ArtifactReader<'a>,
        profile: ProfileId,
        module: ModuleId,
        view: dir::View<'a>,
        bindings: dir::BindingTable<'static>,
        modules: dir::ModuleTable<'static>,
        strings: &'a StringPool,
    ) -> Self {
        // initialize phase output
        Self {
            artifacts,
            module,
            view,
            bindings,
            modules,
            strings,
            imports: dir::ImportTable::new(module),
            references: dir::ReferenceTable::new(module),
            diagnostics: Vec::new(),
            stats: ResolveStats::default(),
            module_clauses: Vec::new(),
            path_references: Vec::new(),
            member_chain_depth: 0,
            global_keys: IndexSet::new(),
            language_item_uses: IndexSet::new(),
            function_stack: Vec::new(),
            exports: ExportResolver::new(profile),
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Collect one source path reference for later target lookup.
    pub(in crate::resolve) fn collect_path_reference(&mut self, reference: PathReference) {
        let Some(root) = reference.path.segments.first().copied() else {
            return;
        };

        // expression member chains already visit their root identifier
        if reference.source.local_id.ty != dir::NodeType::TypeExpression
            && reference.path.segments.len() > 1
        {
            self.path_references.push(reference);

            return;
        }

        // collect global keys only when no local root wins
        self.stats.local_binding_lookups += 1;
        let key = dir::StaticKey::Name(root);
        let local_symbols = self.visible_symbols(reference.source.local_id, key, reference.space);
        if local_symbols.is_empty() && self.global_keys.insert(key) {
            self.stats.required_globals += 1;
        }

        self.path_references.push(reference);
    }

    /// Return symbols visible at one source node.
    pub(in crate::resolve) fn visible_symbols(
        &self,
        source: dir::LocalNodeIdAny,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) -> SmallVec<[dir::GlobalSymbolId; 2]> {
        let lookup = self
            .bindings
            .lookup_symbol_at(&self.view, source, key, space);

        self.symbols_from_lookup(lookup)
    }

    /// Return global symbols from one local lookup result.
    fn symbols_from_lookup(&self, lookup: dir::SymbolLookup) -> SmallVec<[dir::GlobalSymbolId; 2]> {
        match lookup {
            dir::SymbolLookup::Found(symbol) => smallvec![symbol.into_global(self.module)],
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| symbol.into_global(self.module))
                .collect(),
            dir::SymbolLookup::Missing => SmallVec::new(),
        }
    }

    /// Record one language item used by active roots.
    pub(in crate::resolve) fn record_language_item(&mut self, item: dir::LanguageItem) {
        if self.language_item_uses.insert(item) {
            self.stats.language_item_uses += 1;
        }
    }

    /// Enter one function context.
    pub(in crate::resolve) fn enter_function(&mut self, signature: &dir::FunctionSignature) {
        self.function_stack.push(FunctionContext {
            asynchrony: signature.asynchrony,
            is_generator: signature.is_generator,
        });
    }

    /// Leave the current function context.
    pub(in crate::resolve) fn leave_function(&mut self) {
        self.function_stack.pop();
    }

    /// Return the current function context.
    pub(in crate::resolve) fn current_function(&self) -> Option<FunctionContext> {
        self.function_stack.last().copied()
    }

    /// Return a reference from one visible symbol set.
    pub(in crate::resolve) fn reference_from_symbols(
        &self,
        symbols: SmallVec<[dir::GlobalSymbolId; 2]>,
    ) -> dir::Reference {
        // resolve local import aliases to their exported targets
        let targets = symbols.into_iter().map(|symbol| {
            if symbol.module_id == self.module
                && let Some(target) = self.imports.symbol_target(symbol.local_id)
            {
                target
            } else {
                dir::ImportTarget::Symbol(symbol)
            }
        });

        self.reference_from_targets(targets)
    }

    /// Return a reference from visible import targets.
    pub(in crate::resolve) fn reference_from_targets(
        &self,
        targets: impl IntoIterator<Item = dir::ImportTarget>,
    ) -> dir::Reference {
        let mut symbols: SmallVec<[dir::GlobalSymbolId; 2]> = SmallVec::new();
        let mut namespace = None;
        let mut is_conflicting_namespace = false;

        // collect symbols and a possible namespace target
        for target in targets {
            match target {
                dir::ImportTarget::Symbol(symbol) if !symbols.contains(&symbol) => {
                    symbols.push(symbol);
                }
                dir::ImportTarget::Symbol(_) => {}
                dir::ImportTarget::Namespace(module) => {
                    is_conflicting_namespace |= namespace.replace(module).is_some();
                }
            }
        }

        // prefer concrete symbols over namespace objects
        if !symbols.is_empty() {
            dir::Reference::Bound(symbols)
        }
        // keep a single namespace reference
        else if let Some(module) = namespace
            && !is_conflicting_namespace
        {
            dir::Reference::Namespace(module)
        }
        // no visible target remains
        else {
            dir::Reference::Missing
        }
    }

    /// Drain recoverable diagnostics.
    pub(in crate::resolve) fn take_diagnostics(&mut self) -> Vec<ResolveError> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Resolve one exported target through this state's export resolver.
    pub(in crate::resolve) fn resolve_export_target(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        self.exports
            .resolve_export_target(&self.artifacts, module, key)
    }

    /// Finish resolved DIR.
    pub(in crate::resolve) fn finish(self) -> DirResolved {
        DirResolved {
            imports: self.imports,
            references: self.references,
        }
    }

    /// Report one missing export diagnostic.
    pub(in crate::resolve) fn report_missing_export(
        &mut self,
        target_module: ModuleId,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        key: dir::ExportKey,
        specifier: dir::StringId,
    ) -> CompilerResult<()> {
        // render diagnostic payload
        let anchor = self.anchor_node(item_id.id)?;
        let name = self.export_key_text(key);
        let target = self.strings.get(specifier).to_string();
        let suggestion = self.closest_export_key(target_module, &name)?;
        let diagnostic = ResolveError::MissingExport {
            anchor,
            name,
            target,
            suggestion,
        };

        // record recoverable error
        self.diagnostics.push(diagnostic);

        Ok(())
    }

    /// Report one ambiguous export diagnostic.
    pub(in crate::resolve) fn report_ambiguous_export(
        &mut self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        key: dir::ExportKey,
        specifier: dir::StringId,
    ) -> CompilerResult<()> {
        // render diagnostic payload
        let anchor = self.anchor_node(item_id.id)?;
        let name = self.export_key_text(key);
        let target = self.strings.get(specifier).to_string();
        let diagnostic = ResolveError::AmbiguousExport {
            anchor,
            name,
            target,
        };

        // record recoverable error
        self.diagnostics.push(diagnostic);

        Ok(())
    }

    /// Return the source anchor for one local node id.
    fn anchor_node(&self, node_id: u32) -> Result<DiagnosticAnchor, ResolveError> {
        self.view
            .get_span_by_id(node_id)
            .map(DiagnosticAnchor::from)
            .ok_or_else(|| ResolveError::Internal {
                anchor: DiagnosticAnchor::from(self.module),
                module: self.module,
                message: format!("missing source span for resolved node {node_id}"),
            })
    }

    /// Render one export key for diagnostics.
    fn export_key_text(&self, key: dir::ExportKey) -> String {
        match key {
            dir::ExportKey::Default => "default".to_string(),
            dir::ExportKey::Named(key) => self.static_key_text(key),
        }
    }

    /// Render one static key for diagnostics.
    fn static_key_text(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings.get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(symbol) => symbol.debug_string(self.strings),
        }
    }

    /// Return the closest exact export key visible from one target module.
    fn closest_export_key(
        &mut self,
        target_module: ModuleId,
        key: &str,
    ) -> CompilerResult<Option<String>> {
        let exported = self
            .exports
            .exported_module(&self.artifacts, target_module)?;
        let candidates = exported
            .exports
            .exports()
            .map(|(key, _)| self.export_key_text(*key));

        Ok(closest_string(
            key,
            candidates,
            diagnostic_suggestion_distance(key),
        ))
    }
}
