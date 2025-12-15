use std::sync::Arc;

use destack_dir::{LocalNodeId, Node, NodeTree, NodeTreeImpl};
use destack_source::{FileId, ModuleId, Span};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};
use parking_lot::RwLock;

use crate::{LintDiagnostic, LintMeta};

/// Context for DIR-level linting of a single module.
pub struct LintModuleDirContext {
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

impl std::fmt::Debug for LintModuleDirContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleDirContext")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl LintModuleDirContext {
    /// Create a new DIR lint context for a module.
    pub fn new(program: Arc<Program>, module: Arc<RwLock<Module>>, options: LinterOptions) -> Self {
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
    pub fn get_span<T: Node>(&self, id: LocalNodeId<T>) -> Span {
        let module = self.module.read();
        let dir_tree = module.dir.tree.read();
        let ast_node_id = dir_tree.get_source(id.id);
        module.ast.tree.get_span_by_id(ast_node_id)
    }

    /// Iterate all nodes of a given type and call the callback for each.
    pub fn for_each<N, F>(&mut self, mut callback: F)
    where
        N: Node + Clone,
        NodeTree: NodeTreeImpl<N>,
        F: for<'a> FnMut(&'a NodeTree, LocalNodeId<N>, &'a N, Span) -> Option<LintDiagnostic>,
    {
        let diagnostics: Vec<_> = {
            let module = self.module.read();
            let dir_tree = module.dir.tree.read();
            let ast_tree = &module.ast.tree;
            dir_tree
                .iter_nodes_of_type::<N>()
                .filter_map(|(node_id, node)| {
                    let ast_node_id = dir_tree.get_source(node_id.id);
                    let span = ast_tree.get_span_by_id(ast_node_id);
                    callback(&dir_tree, node_id, node, span)
                })
                .collect()
        };

        for diagnostic in diagnostics {
            self.report(diagnostic);
        }
    }
}
