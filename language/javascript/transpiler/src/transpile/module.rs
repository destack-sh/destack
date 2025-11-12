use dyst_ast::StringId;
use dyst_dir::{self as dir, ModuleId};
use dyst_javascript_ast::{self as ast, Definition, NodeId, NodeIdAny};
use dyst_source::{StringPool, Uri};

use crate::{TranspileError, TranspileResult, Transpiler};

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
    /// The URI of the transpiled module (excluding extension).
    pub uri: Uri,
    /// The AST of the transpiled module.
    pub ast: ast::NodeTree,
    /// The root nodes of the transpiled module.
    pub roots: Vec<NodeIdAny>,
    /// The string pool.
    pub strings: StringPool,
    /// The source modules.
    pub sources: Vec<ModuleId>,
    /// The errors encountered during transpilation.
    pub errors: Vec<TranspileError>,
    /// The artifacts produced by the transpilation unit.
    pub artifacts: Vec<Uri>,
}

impl TranspilerUnit {
    /// Get alias for a definition from a given node.
    pub fn get_alias_to_definition(
        &self,
        from_id: NodeIdAny,
        to_id: NodeId<Definition>,
    ) -> StringId {
        todo!("get_alias_to_definition: {from_id:?} -> {to_id:?}");
    }

    /// Add an error to the transpilation unit.
    pub(crate) fn add_error(&mut self, error: TranspileError) {
        self.errors.push(error);
    }

    /// Try to do something and remember the TranspilerError if it fails.
    pub(crate) fn try_recoverable<T>(
        &mut self,
        f: impl FnOnce(&mut TranspilerUnit) -> TranspileResult<T>,
    ) -> Option<T> {
        match f(self) {
            Ok(result) => Some(result),
            Err(error) => {
                self.add_error(error);
                None
            }
        }
    }
}

impl<'a> Transpiler<'a> {
    /// Transpile the modules into AST.
    pub fn transpile_module(&self, module: &'a dir::Module, unit: &mut TranspilerUnit) {
        for expression_id in module.expressions.iter() {
            if let Some(root_id) =
                unit.try_recoverable(|unit| self.transpile_expression(module, *expression_id, unit))
            {
                unit.roots.push(root_id.into())
            }
        }
    }
}
