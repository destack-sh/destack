mod no_console;

use crate::{BoxedLintRule, boxed};

pub use no_console::*;

/// Get all restriction rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoConsole),
    ]
}
