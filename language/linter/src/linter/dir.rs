use std::sync::Arc;

use destack_ast::StringId;
use destack_source::{EditBuilder, File, ModuleId, Span};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};
use indexmap::IndexMap;
use {destack_ast as ast, destack_dir as dir};

use crate::{LintDiagnostic, LintMeta};

/// Context for DIR-level linting of a single module. Unfurls ModuleDir.
pub struct LintModuleDirContext<'a> {
    /// The program containing this module.
    pub program: Arc<Program>,
    /// The module being linted.
    pub module: &'a Module,
    /// The source file.
    pub file: Arc<File>,

    /// The AST tree.
    pub ast: &'a ast::NodeTree,
    /// The DIR tree.
    pub tree: &'a dir::NodeTree,
    /// The symbol table.
    pub symbols: &'a dir::SymbolTable,
    /// The type table.
    pub types: &'a dir::TypeTable,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,

    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: Vec<ModuleId>,
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier)).
    pub imported_modules: IndexMap<(Option<ModuleId>, StringId), ModuleId>,
    /// Exported symbols by key (space, name).
    pub exported_symbols: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::LocalSymbolId>,

    /// Linter configuration.
    pub options: &'a LinterOptions,

    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> std::fmt::Debug for LintModuleDirContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleDirContext")
            .field("module_id", &self.module.id)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> LintModuleDirContext<'a> {
    /// Create a new DIR lint context for a module.
    pub fn new(
        program: Arc<Program>,
        module: &'a Module,
        file: Arc<File>,
        ast: &'a ast::NodeTree,
        tree: &'a dir::NodeTree,
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        roots: Vec<dir::LocalNodeId<dir::Expression>>,
        namespace_symbol: dir::LocalSymbolId,
        namespace_scope: dir::LocalScopeId,
        default_symbol: dir::LocalSymbolId,
        namespace_exports: Vec<ModuleId>,
        imported_modules: IndexMap<(Option<ModuleId>, StringId), ModuleId>,
        exported_symbols: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::LocalSymbolId>,
        options: &'a LinterOptions,
    ) -> Self {
        Self {
            program,
            module,
            file,
            ast,
            tree,
            symbols,
            types,
            roots,
            namespace_symbol,
            namespace_scope,
            default_symbol,
            namespace_exports,
            imported_modules,
            exported_symbols,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Return the file id.
    pub fn file_id(&self) -> destack_source::FileId {
        self.module.file_id
    }

    /// Return the module id.
    pub fn module_id(&self) -> destack_source::ModuleId {
        self.module.id
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options
            .resolve_severity(meta.id, meta.category, meta.category.default_severity())
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Return a reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }

    /// Return the source span for a DIR node by looking up its AST source node.
    pub fn get_span<T: dir::Node>(&self, id: dir::LocalNodeId<T>) -> Span {
        let ast_node_id = self.tree.get_source(id.id);
        self.ast.get_span_by_id(ast_node_id)
    }

    /// Get the full source text.
    pub fn source_text(&self) -> &str {
        self.file.text()
    }

    /// Get the source text for a span.
    pub fn get_span_text(&self, span: Span) -> &str {
        &self.file.text()[span.start as usize..span.end as usize]
    }

    /// Create an EditBuilder with source text for text-aware operations.
    pub fn edit_builder(&self) -> EditBuilder<'_> {
        EditBuilder::with_source(self.module.file_id, self.file.text())
    }
}
