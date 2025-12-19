use std::sync::Arc;

use destack_ast::{self as ast, StringPool};
use destack_source::{EditBuilder, File};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};

use crate::{LintDiagnostic, LintMeta};

/// Context for AST-level linting of a single module. Unfurls ModuleAst.
pub struct LintModuleAstContext<'a> {
    /// The program containing this module.
    pub program: Arc<Program>,
    /// The module being linted.
    pub module: &'a Module,
    /// The source file.
    pub file: Arc<File>,

    /// The AST tree.
    pub tree: &'a ast::NodeTree,
    /// The parent index.
    pub parents: &'a ast::NodeParentIndex,
    /// The roots.
    pub roots: &'a Vec<ast::LocalNodeId<ast::Expression>>,
    /// The string pool.
    pub strings: &'a StringPool,

    /// Linter configuration.
    pub options: &'a LinterOptions,

    /// Whether to compute fixes for diagnostics.
    pub compute_fixes: bool,

    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> std::fmt::Debug for LintModuleAstContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleAstContext")
            .field("module_id", &self.module.id)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> LintModuleAstContext<'a> {
    /// Create a new AST lint context for a module.
    pub fn new(
        program: Arc<Program>,
        module: &'a Module,
        file: Arc<File>,
        tree: &'a ast::NodeTree,
        parents: &'a ast::NodeParentIndex,
        roots: &'a Vec<ast::LocalNodeId<ast::Expression>>,
        strings: &'a StringPool,
        options: &'a LinterOptions,
        compute_fixes: bool,
    ) -> Self {
        Self {
            program,
            module,
            file,
            tree,
            parents,
            roots,
            strings,
            options,
            compute_fixes,
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

    /// Return the linter options.
    pub fn options(&self) -> &LinterOptions {
        self.options
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

    /// Get effective severity for a rule at a specific node.
    ///
    /// This checks for `@allow`/`@deny`/`@warn`/`@forbid` decorators on the node
    /// and its ancestors, returning the effective severity at that location.
    pub fn get_effective_severity<T: ast::Node>(
        &self,
        meta: &LintMeta,
        _node_id: ast::LocalNodeId<T>,
    ) -> LintSeverity {
        // nocheckin TODO #Incomplete: walk up parents checking for @allow/@deny/@warn/@forbid decorators
        self.get_severity(meta)
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

    /// Get the full source text.
    pub fn source_text(&self) -> &str {
        self.file.text()
    }

    /// Get the source text for a span.
    pub fn get_span_text(&self, span: destack_source::Span) -> &str {
        &self.file.text()[span.start as usize..span.end as usize]
    }

    /// Create an EditBuilder with source text for text-aware operations.
    pub fn edit_builder(&self) -> EditBuilder<'_> {
        EditBuilder::from_file(self.module.file_id, self.file.text())
    }
}
