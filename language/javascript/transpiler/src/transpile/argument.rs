use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Argument, NodeId, Parameter};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    // nocheckin TODO #Incomplete: transpile parameters and arguments

    /// Transpile a parameter from DIR into JS AST.
    pub fn transpile_parameter(
        &self,
        _module: &'a Module,
        parameter_id: dir::NodeId<dir::Parameter>,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Parameter>> {
        Err(TranspileError::UnsupportedNode {
            node: parameter_id.into_any(),
            message: None,
        })
    }

    /// Transpile a argument from DIR into JS AST.
    pub fn transpile_argument(
        &self,
        _module: &'a Module,
        argument_id: dir::NodeId<dir::Argument>,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Argument>> {
        Err(TranspileError::UnsupportedNode {
            node: argument_id.into_any(),
            message: None,
        })
    }
}
