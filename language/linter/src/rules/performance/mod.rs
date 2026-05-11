mod no_array_for_each;
mod no_array_unshift_loop;
mod no_await_in_loop;
mod no_barrel_file;
mod no_json_clone;
mod no_nested_array_includes;
mod no_regex_in_loop;
mod no_sequential_independent_await;
mod no_string_concat_in_loop;
mod no_super_linear_regex;
mod prefer_array_every;
mod prefer_array_literal;
mod prefer_for_of;
mod prefer_includes;
mod prefer_string_endswith;
mod prefer_string_startswith;
mod require_unicode_regexp;

use crate::{BoxedLintRule, boxed};

pub use no_array_for_each::*;
pub use no_array_unshift_loop::*;
pub use no_await_in_loop::*;
pub use no_barrel_file::*;
pub use no_json_clone::*;
pub use no_nested_array_includes::*;
pub use no_regex_in_loop::*;
pub use no_sequential_independent_await::*;
pub use no_string_concat_in_loop::*;
pub use no_super_linear_regex::*;
pub use prefer_array_every::*;
pub use prefer_array_literal::*;
pub use prefer_for_of::*;
pub use prefer_includes::*;
pub use prefer_string_endswith::*;
pub use prefer_string_startswith::*;
pub use require_unicode_regexp::*;

/// Get all performance rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoArrayForEach),
        boxed(NoArrayUnshiftLoop),
        boxed(NoAwaitInLoop),
        boxed(NoBarrelFile),
        boxed(NoJsonClone),
        boxed(NoNestedArrayIncludes),
        boxed(NoRegexInLoop),
        boxed(NoSequentialIndependentAwait),
        boxed(NoStringConcatInLoop),
        boxed(NoSuperLinearRegex),
        boxed(PreferArrayEvery),
        boxed(PreferArrayLiteral),
        boxed(PreferForOf),
        boxed(PreferIncludes),
        boxed(PreferStringEndsWith),
        boxed(PreferStringStartsWith),
        boxed(RequireUnicodeRegexp),
    ]
}
