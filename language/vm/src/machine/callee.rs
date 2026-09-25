use tspp_bytecode::{Code, CodeRange, Function};
use tspp_program::{Binding, FunctionId, Program};

use crate::diagnostic::{Error, Result};

/// One resolved Program function call destination.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Callee<'a> {
    /// Linked bytecode function.
    Bytecode {
        /// Bytecode function metadata.
        function: &'a Function,
        /// Encoded function body.
        code: CodeRange,
    },
    /// Runtime binding declaration.
    Binding(&'a Binding),
}

impl<'a> Callee<'a> {
    /// Resolve one Program function call destination.
    pub(crate) fn resolve(
        program: &'a Program,
        bytecode: Code,
        function: FunctionId,
    ) -> Result<Self> {
        let linked = bytecode
            .function(program.sections(), function.index())
            .ok_or_else(|| Error::undefined_function(function))?;
        if let Some(code) = linked.code() {
            return Ok(Self::Bytecode {
                function: linked,
                code,
            });
        }

        // otherwise resolve one runtime binding declaration
        let Some(binding) = program.function_binding(function) else {
            return Err(Error::undefined_function(function));
        };

        Ok(Self::Binding(binding))
    }
}
