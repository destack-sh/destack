use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Argument, NodeId};

use crate::{TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a argument from DIR into JS AST.
    pub fn transpile_argument(
        &self,
        _module: &'a Module,
        argument_id: dir::NodeId<dir::Argument>,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Argument>> {
        todo!("transpile_argument({argument_id:?})");
    }
}
