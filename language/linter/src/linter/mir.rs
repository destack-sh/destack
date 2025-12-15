use std::sync::Arc;

use destack_mir as mir;
use destack_source::{FileId, ModuleId};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};
use parking_lot::RwLock;
use crate::{LintDiagnostic, LintMeta};

/// Context for MIR-level linting of a single module.
pub struct LintModuleMirContext {
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

impl std::fmt::Debug for LintModuleMirContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleMirContext")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl LintModuleMirContext {
    /// Create a new MIR lint context for a module.
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

    /// Iterate all nodes of a given type and call the callback for each.
    pub fn for_each<N, F>(&mut self, mut callback: F)
    where
        N: mir::Node,
        mir::NodeTree: mir::NodeTreeImpl<N>,
        F: FnMut(&N, mir::LocalNodeId<N>) -> Option<LintDiagnostic>,
    {
        let diagnostics: Vec<_> = {
            let module = self.module.read();
            let tree = module.mir.tree.read();
            tree.iter_nodes::<N>()
                .filter_map(|(id, node)| callback(node, id))
                .collect()
        };

        for diagnostic in diagnostics {
            self.report(diagnostic);
        }
    }
}
