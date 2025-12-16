mod no_await_in_loop;

use crate::{BoxedLintRule, boxed};

pub use no_await_in_loop::*;

/// Get all performance rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![boxed(NoAwaitInLoop)]
}
