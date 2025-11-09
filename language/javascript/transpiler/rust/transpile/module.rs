use dyst_dir::{self as dir, ModuleId};
use dyst_javascript_ast as ast;

use crate::Transpiler;

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
    /// The AST of the transpiled file.
    pub ast: ast::NodeTree,
    /// The source modules.
    pub sources: Vec<ModuleId>,
}

impl<'a> Transpiler<'a> {
    /// Transpile the modules into AST.
    pub fn transpile_module(&self, module: &'a dir::Module, unit: &mut TranspilerUnit) {
        for expression_id in module.expressions.iter() {
            let expression = self.tree.get(*expression_id);
            self.transpile_expression(module, expression, unit);
        }
    }
}
