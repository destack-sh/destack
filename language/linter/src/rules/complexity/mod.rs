mod max_depth;
mod max_lines;
mod max_lines_per_function;
mod max_nested_callbacks;
mod max_params;
mod max_statements;
mod no_multi_assign;

use crate::{BoxedLintRule, boxed};

pub use max_depth::*;
pub use max_lines::*;
pub use max_lines_per_function::*;
pub use max_nested_callbacks::*;
pub use max_params::*;
pub use max_statements::*;
pub use no_multi_assign::*;

/// Get all complexity rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(MaxDepth),
        boxed(MaxLines),
        boxed(MaxLinesPerFunction),
        boxed(MaxNestedCallbacks),
        boxed(MaxParams),
        boxed(MaxStatements),
        boxed(NoMultiAssign),
    ]
}
