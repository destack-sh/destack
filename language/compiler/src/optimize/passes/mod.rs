mod constant_fold;
mod dead_code_eliminate;

use crate::optimize::{BoxedFunctionPass, OptimizationLevel, boxed_function};

pub use constant_fold::*;
pub use dead_code_eliminate::*;

/// Get all available function passes.
pub fn all_function_passes() -> Vec<BoxedFunctionPass> {
    vec![
        boxed_function(ConstantFold),
        boxed_function(DeadCodeEliminate),
    ]
}

/// Get function passes that run at a given optimization level.
pub fn passes_for_level(level: OptimizationLevel) -> Vec<BoxedFunctionPass> {
    match level {
        // O0: no optimizations
        OptimizationLevel::O0 => vec![],

        // O1: fast local passes
        OptimizationLevel::O1 => vec![
            boxed_function(ConstantFold),
            boxed_function(DeadCodeEliminate),
        ],

        // O2: full suite (includes O1 passes)
        OptimizationLevel::O2 => vec![
            boxed_function(ConstantFold),
            boxed_function(DeadCodeEliminate),
            // NOTE #Incomplete: add inlining, escape analysis, devirt
        ],

        // O3: aggressive (includes O2 passes with higher thresholds)
        OptimizationLevel::O3 => vec![
            boxed_function(ConstantFold),
            boxed_function(DeadCodeEliminate),
            // NOTE #Incomplete: add aggressive inlining, loop unrolling
        ],
    }
}
