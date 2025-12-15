use std::sync::Arc;

use destack_ast::{self as ast, StringPool};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};

use crate::{LintDiagnostic, LintMeta};

/// Context for AST-level linting of a single module. Unfurls ModuleAst.
pub struct LintModuleAstContext<'a> {
    /// The program containing this module.
    pub program: Arc<Program>,
    /// The module being linted.
    pub module: &'a Module,

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

impl<'a> LintModuleAstContext<'a> {
    /// Create a new AST lint context for a module.
    pub fn new(
        program: Arc<Program>,
        module: &'a Module,
        tree: &'a ast::NodeTree,
        parents: &'a ast::NodeParentIndex,
        roots: &'a Vec<ast::LocalNodeId<ast::Expression>>,
        strings: &'a StringPool,
        options: &'a LinterOptions,
    ) -> Self {
        Self {
            program,
            module,
            tree,
            parents,
            roots,
            strings,
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
}
