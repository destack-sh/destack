use std::sync::Arc;

use destack_ast::{Node, NodeTree, NodeTreeImpl};
use destack_source::{FileId, ModuleId, Span};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};
use parking_lot::RwLock;

use crate::{LintDiagnostic, LintMeta};

/// Context for AST-level linting of a single module.
pub struct LintModuleAstContext {
    /// The program containing this module.
    pub program: Arc<Program>,
    /// The id of the module being linted.
    pub module_id: ModuleId,
    /// The file id of the module's source.
    pub file_id: FileId,
    /// The module being linted.
    module: Arc<RwLock<Module>>,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl std::fmt::Debug for LintModuleAstContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleAstContext")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl LintModuleAstContext {
    /// Create a new AST lint context for a module.
    pub fn new(
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        options: LinterOptions,
    ) -> Self {
        let (module_id, file_id) = {
            let m = module.read();
            (m.id, m.file_id)
        };
        Self {
            program,
            module_id,
            file_id,
            module,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Return the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.options
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options.resolve_severity(
            meta.id,
            meta.category.name(),
            meta.category.default_severity(),
            meta.is_recommended(),
        )
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

    /// Iterate all nodes of a given type and call the callback for each.
    pub fn for_each<N, F>(&mut self, mut callback: F)
    where
        N: Node + Clone,
        NodeTree: NodeTreeImpl<N>,
        F: FnMut(&NodeTree, &N, Span) -> Option<LintDiagnostic>,
    {
        let diagnostics: Vec<_> = {
            let module = self.module.read();
            let tree = &module.ast.tree;
            tree.iter_nodes::<N>()
                .filter_map(|id| {
                    let span = tree.get_span(id);
                    let node = tree.get(id);
                    callback(tree, node, span)
                })
                .collect()
        };

        for diagnostic in diagnostics {
            self.report(diagnostic);
        }
    }
}
