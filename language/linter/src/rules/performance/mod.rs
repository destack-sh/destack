mod no_await_in_loop;
mod no_barrel_file;
mod no_super_linear_regex;
mod require_unicode_regexp;

use crate::{BoxedLintRule, boxed};

pub use no_await_in_loop::*;
pub use no_barrel_file::*;
pub use no_super_linear_regex::*;
pub use require_unicode_regexp::*;

/// Get all performance rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoAwaitInLoop),
        boxed(NoBarrelFile),
        boxed(NoSuperLinearRegex),
        boxed(RequireUnicodeRegexp),
    ]
}
