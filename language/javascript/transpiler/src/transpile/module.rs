use dyst_dir::{self as dir, ModuleId};
use dyst_javascript_ast::{self as ast, NodeIdAny};
use dyst_source::{SharedStringPool, Uri};

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
    /// The options for transpilation.
    pub options: TranspileOptions,
    /// The id of the transpiled module.
    pub id: TranspilerUnitId,
    /// The URI of the transpiled module (excluding extension).
    pub uri: Uri,
    /// The AST of the transpiled module.
    pub ast: ast::MutableNodeTree,
    /// The root nodes of the transpiled module.
    pub roots: Vec<NodeIdAny>,
    /// The string pool.
    pub strings: SharedStringPool,
    /// The source modules.
    pub sources: Vec<ModuleId>,
    /// The diagnostics encountered during transpilation.
    pub diagnostics: Vec<TranspileDiagnostic>,
    /// The artifacts produced by the transpilation unit.
    pub artifacts: Vec<Uri>,
}

#[allow(unused)]
impl TranspilerUnit {
    /// Add an error to the transpilation unit.
    pub(crate) fn error(&mut self, error: TranspileError) {
        self.diagnostics.push(error.into());
    }

    /// Add a warning to the transpilation unit.
    pub(crate) fn warning(&mut self, warning: TranspileWarning) {
        self.diagnostics.push(warning.into());
    }
}

impl<'a> Transpiler<'a> {
    /// Transpile the modules into AST.
    pub fn transpile_module(&self, module: &'a dir::Module, unit: &mut TranspilerUnit) {
        for expression_id in module.expressions.iter() {
            match self.transpile_expression(module, *expression_id, unit) {
                Ok(root_id) => unit.roots.push(root_id),
                Err(error) => unit.error(error),
            }
        }
    }
}
