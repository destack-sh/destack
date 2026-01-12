/// Declare a benchmark program constant.
macro_rules! declare_program {
    ($(#[$meta:meta])* $vis:vis const $name:ident, $($body:tt)*) => {
        $(#[$meta])*
        $vis const $name: Program = Program {
            $($body)*
            runner: crate::program::ProgramRunner::Function,
        };
    };
}

/// Declare a benchmark program constant with an explicit runner.
macro_rules! declare_program_with_runner {
    ($(#[$meta:meta])* $vis:vis const $name:ident, $($body:tt)*) => {
        $(#[$meta])*
        $vis const $name: Program = Program { $($body)* };
    };
}

pub(crate) mod arithmetic;
pub(crate) mod calls;
mod common;
pub(crate) mod dispatch;
mod index;
pub(crate) mod intrinsics;
pub(crate) mod memory;
mod perf;
mod program;

use destack_mir as mir;
use destack_vm::interpreter::Interpreter;
use destack_vm::memory::Value;

/// Return the function id for a named function in the interpreter.
pub(crate) fn function_id_by_name(
    interp: &Interpreter,
    name: &str,
) -> mir::LocalNodeId<mir::Function> {
    // resolve function id
    let function_id = interp
        .function_id_by_name(name)
        .unwrap_or_else(|_| panic!("function '{name}' not found"));

    // return function id
    function_id
}

/// Return a function pointer value for a named function in the interpreter.
pub(crate) fn function_pointer_by_name(interp: &Interpreter, name: &str) -> Value {
    // resolve function id
    let function_id = function_id_by_name(interp, name);

    // build pointer value
    Value::function_pointer(function_id)
}

pub(crate) use index::*;
