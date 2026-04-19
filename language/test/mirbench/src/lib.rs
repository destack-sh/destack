#![allow(
    clippy::assign_op_pattern,
    clippy::let_and_return,
    clippy::print_literal,
    clippy::too_many_arguments
)]

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

pub mod arithmetic;
pub mod calls;
mod common;
pub mod dispatch;
mod index;
pub mod intrinsics;
pub mod memory;
mod perf;
mod program;

use std::path::{Path, PathBuf};

use destack_mir as mir;
use destack_vm::{Isolate, Value};

/// Return the function id for a named function in the isolate.
pub fn function_id_by_name(isolate: &Isolate, name: &str) -> mir::LocalNodeId<mir::Function> {
    // resolve function id
    let function_id = isolate
        .function_id_by_name(name)
        .unwrap_or_else(|_| panic!("function '{name}' not found"));

    // return function id
    function_id
}

/// Return a function pointer value for a named function in the isolate.
pub fn function_pointer_by_name(isolate: &Isolate, name: &str) -> Value {
    // resolve function id
    let function_id = function_id_by_name(isolate, name);

    // build pointer value
    Value::function_pointer(function_id)
}

pub use index::*;

/// Return all MIR bench program definitions in a stable order.
pub fn all_programs() -> Vec<&'static program::Program> {
    // collect program lists
    let mut programs = Vec::new();

    for entry in dispatch::ALL {
        programs.push(*entry);
    }

    for entry in arithmetic::ALL {
        programs.push(*entry);
    }

    for entry in calls::ALL {
        programs.push(*entry);
    }

    for entry in memory::ALL {
        programs.push(*entry);
    }

    for entry in intrinsics::ALL {
        programs.push(*entry);
    }

    programs
}

/// Return the root directory for the MIR bench fixtures.
pub fn fixtures_root() -> PathBuf {
    // resolve fixtures root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("mirbench")
}
