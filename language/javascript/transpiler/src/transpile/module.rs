use std::sync::Arc;

use dyst_dir::{self as dir, ModuleId, NodeTree, Program, SymbolTable, TypeTable};
use dyst_javascript_ast::{self as ast, LocalNodeIdAny};
use dyst_source::{DiagnosticCollector, StringPool, Uri};

use crate::{TranspileDiagnostic, TranspileError, TranspileOptions, TranspileWarning, Transpiler};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TranspilerUnitId(pub u32);

impl TranspilerUnitId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A transpiled module.
#[derive(Debug, Clone)]
pub struct TranspilerUnit {
    /// The id of the transpiled module.
    pub id: TranspilerUnitId,
    /// The program.
    pub program: Arc<Program>,
    /// The options for transpilation.
    pub options: TranspileOptions,
    /// The URI of the transpiled module (excluding extension).
    pub uri: Uri,
    /// The AST of the transpiled module.
    pub ast: ast::NodeTree,
    /// The root nodes of the transpiled module.
    pub roots: Vec<LocalNodeIdAny>,
    /// The string pool.
    pub strings: StringPool,
    /// The source modules.
    pub sources: Vec<ModuleId>,
    /// The pending diagnostics encountered during transpilation.
    pub pending_diagnostics: DiagnosticCollector,
    /// The artifacts produced by the transpilation unit.
    pub artifacts: Vec<Uri>,
}

#[allow(unused)]
impl TranspilerUnit {
    /// Add an error to the transpilation unit.
    pub(crate) fn error(&mut self, error: TranspileError) {
        let diagnostic: TranspileDiagnostic = error.into();
        let diagnostic = diagnostic.to_diagnostic(self.program.as_ref());
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Add a warning to the transpilation unit.
    pub(crate) fn warning(&mut self, warning: TranspileWarning) {
        let diagnostic: TranspileDiagnostic = warning.into();
        let diagnostic = diagnostic.to_diagnostic(self.program.as_ref());
        self.pending_diagnostics.insert(diagnostic);
    }
}

impl Transpiler {
    /// Transpile the modules into AST.
    pub fn transpile_module(
        &self,
        module: &dir::Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        unit: &mut TranspilerUnit,
    ) {
        for expression_id in module.roots.iter() {
            match self.transpile_expression(module, tree, symbols, types, *expression_id, unit) {
                Ok(root_id) => unit.roots.push(root_id),
                Err(error) => unit.error(error),
            }
        }
    }
}
