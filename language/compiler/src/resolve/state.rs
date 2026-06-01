use std::collections::HashMap;

use destack_artifact::{DiagnosticAnchor, DirResolved};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ArtifactReader;
use indexmap::IndexSet;

use crate::resolve::resolve::ExportLookup;
use crate::{CompilerResult, ResolveError};

/// Resolve phase state for one module.
pub(in crate::resolve) struct ResolveState<'a> {
    /// Provider-scoped artifact reader.
    pub(in crate::resolve) artifacts: ArtifactReader<'a>,
    /// The active profile.
    pub(in crate::resolve) profile: ProfileId,
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
    /// The recoverable diagnostics produced while resolving.
    pub(in crate::resolve) diagnostics: Vec<ResolveError>,
    /// The number of active expression roots walked.
    pub(in crate::resolve) roots: usize,
    /// The number of expression nodes visited.
    pub(in crate::resolve) expressions: usize,
    /// The number of type expression nodes visited.
    pub(in crate::resolve) type_expressions: usize,
    /// The module clauses collected from active roots.
    pub(in crate::resolve) module_clauses: Vec<ModuleClause>,
    /// The number of import clauses collected.
    pub(in crate::resolve) import_clauses: usize,
    /// The number of re-export clauses collected.
    pub(in crate::resolve) reexport_clauses: usize,
    /// Bare global keys required by active roots.
    pub(in crate::resolve) required_global_keys: IndexSet<dir::StaticKey>,
    /// The number of profile global modules read.
    pub(in crate::resolve) global_modules: usize,
    /// Language items required by syntax in active roots.
    pub(in crate::resolve) syntax_language_items: IndexSet<dir::LanguageItem>,
    /// Function contexts visible while walking active roots.
    pub(in crate::resolve) function_stack: Vec<FunctionContext>,
    /// Export lookups already computed during this provider run.
    pub(in crate::resolve) export_lookups: HashMap<ExportLookupKey, ExportLookupState>,
    /// The number of export lookup cache hits.
    pub(in crate::resolve) export_cache_hits: usize,
    /// The number of export lookup cycle hits.
    pub(in crate::resolve) export_cycle_hits: usize,
    /// The DIR visitor options.
    pub(in crate::resolve) options: dir::NodeVisitorOptions,
}

/// Cache key for one exported name in one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::resolve) struct ExportLookupKey {
    /// The module that owns the export table.
    pub(in crate::resolve) module: ModuleId,
    /// The export key being looked up.
    pub(in crate::resolve) key: dir::ExportKey,
}

/// Memoized export lookup state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resolve) enum ExportLookupState {
    /// The lookup is currently being computed.
    Resolving,
    /// The lookup has been computed.
    Resolved(ExportLookup),
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
            profile,
            module,
            view,
            bindings,
            modules,
            strings,
            imports: dir::ImportTable::new(module),
            diagnostics: Vec::new(),
            roots: 0,
            expressions: 0,
            type_expressions: 0,
            module_clauses: Vec::new(),
            import_clauses: 0,
            reexport_clauses: 0,
            required_global_keys: IndexSet::new(),
            global_modules: 0,
            syntax_language_items: IndexSet::new(),
            function_stack: Vec::new(),
            export_lookups: HashMap::new(),
            export_cache_hits: 0,
            export_cycle_hits: 0,
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Add one module clause for later target lookup.
    pub(in crate::resolve) fn add_module_clause(&mut self, clause: ModuleClause) {
        match &clause {
            ModuleClause::Import { .. } => self.import_clauses += 1,
            ModuleClause::ReExport { .. } => self.reexport_clauses += 1,
        }

        self.module_clauses.push(clause);
    }

    /// Require one syntax-required language item.
    pub(in crate::resolve) fn require_language_item(&mut self, item: dir::LanguageItem) {
        self.syntax_language_items.insert(item);
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

    /// Require a global key when one source reference is not locally resolved.
    pub(in crate::resolve) fn require_global_reference(
        &mut self,
        source: dir::LocalNodeIdAny,
        key: dir::StaticKey,
        space: dir::SymbolSpace,
    ) {
        let source = source.into_global(self.module);
        if !matches!(
            self.bindings.lookup_symbol_at(source, key, space),
            dir::SymbolLookup::Missing,
        ) {
            return;
        }

        self.required_global_keys.insert(key);
    }

    /// Drain recoverable diagnostics.
    pub(in crate::resolve) fn take_diagnostics(&mut self) -> Vec<ResolveError> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Finish resolved DIR.
    pub(in crate::resolve) fn finish(self) -> DirResolved {
        DirResolved {
            imports: self.imports,
        }
    }

    /// Report one missing export diagnostic.
    pub(in crate::resolve) fn report_missing_export(
        &mut self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        key: dir::ExportKey,
        specifier: dir::StringId,
    ) -> CompilerResult<()> {
        // render diagnostic payload
        let anchor = self.anchor_node(item_id.id)?;
        let name = self.export_key_text(key);
        let target = self.strings.get(specifier).to_string();
        let diagnostic = ResolveError::MissingExport {
            anchor,
            name,
            target,
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
}
