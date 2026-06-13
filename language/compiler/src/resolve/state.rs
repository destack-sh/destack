use destack_artifact::{DiagnosticAnchor, DirResolved};
use destack_core::{StringPool, closest_string};
use destack_dir as dir;
use destack_repository::ArtifactReader;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexSet;

use crate::export::{ExportLookup, ExportResolver};
use crate::resolve::stats::ResolveStats;
use crate::{CompilerResult, ResolveError, diagnostic_suggestion_distance};

/// Resolve phase state for one module.
pub(in crate::resolve) struct ResolveState<'a> {
    /// Provider-scoped artifact reader.
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
    /// The namespace path table being built.
    pub(in crate::resolve) paths: dir::PathTable,
    /// The recoverable diagnostics produced while resolving.
    pub(in crate::resolve) diagnostics: Vec<ResolveError>,
    /// The work stats accumulated while resolving.
    pub(in crate::resolve) stats: ResolveStats,
    /// Import and re-export clauses collected from active roots.
    pub(in crate::resolve) module_clauses: Vec<ModuleClause>,
    /// Namespace path references collected from active roots.
    pub(in crate::resolve) path_references: Vec<PathReference>,
    /// Member path collection depth during the resolve walk.
    pub(in crate::resolve) member_path_collection_depth: usize,
    /// Bare global keys required by active roots.
    pub(in crate::resolve) global_keys: IndexSet<dir::StaticKey>,
    /// Language items required by syntax in active roots.
    pub(in crate::resolve) language_items: IndexSet<dir::LanguageItem>,
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
}

/// Function context visible to syntax-dependent dependency collection.
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
            paths: dir::PathTable::new(module),
            diagnostics: Vec::new(),
            stats: ResolveStats::default(),
            module_clauses: Vec::new(),
            path_references: Vec::new(),
            member_path_collection_depth: 0,
            global_keys: IndexSet::new(),
            language_items: IndexSet::new(),
            function_stack: Vec::new(),
            exports: ExportResolver::new(profile),
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Collect one namespace path reference for later target lookup.
    pub(in crate::resolve) fn collect_path_reference(&mut self, reference: PathReference) {
        self.path_references.push(reference);
    }

    /// Require one syntax-required language item.
    pub(in crate::resolve) fn require_language_item(&mut self, item: dir::LanguageItem) {
        if self.language_items.insert(item) {
            self.stats.required_language_items += 1;
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

    /// Collect one bare reference for global resolution when no local binding exists.
    pub(in crate::resolve) fn collect_global_reference(
        &mut self,
        source: dir::LocalNodeIdAny,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) {
        let source = source.into_global(self.module);
        self.stats.local_binding_lookups += 1;

        if !matches!(
            self.bindings.lookup_symbol_at(source, key, space),
            dir::SymbolLookup::Missing,
        ) {
            return;
        }

        if self.global_keys.insert(key) {
            self.stats.required_globals += 1;
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
        // precompute the import edges component discovery walks
        let mut import_modules = self.imports.modules().collect::<IndexSet<_>>();
        import_modules.shift_remove(&self.module);

        DirResolved {
            imports: self.imports,
            paths: self.paths,
            import_modules: import_modules.into_iter().collect(),
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
