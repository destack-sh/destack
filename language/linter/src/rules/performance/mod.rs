mod no_accumulating_spread;
mod no_array_for_each;
mod no_array_unshift_loop;
mod no_await_in_loop;
mod no_barrel_file;
mod no_json_clone;
mod no_nested_array_includes;
mod no_object_spread_in_reduce;
mod no_regex_in_loop;
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

pub use no_accumulating_spread::*;
pub use no_array_for_each::*;
pub use no_array_unshift_loop::*;
pub use no_await_in_loop::*;
pub use no_barrel_file::*;
pub use no_json_clone::*;
pub use no_nested_array_includes::*;
pub use no_object_spread_in_reduce::*;
pub use no_regex_in_loop::*;
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
        boxed(NoAccumulatingSpread),
        boxed(NoArrayForEach),
        boxed(NoArrayUnshiftLoop),
        boxed(NoAwaitInLoop),
        boxed(NoBarrelFile),
        boxed(NoJsonClone),
        boxed(NoNestedArrayIncludes),
        boxed(NoObjectSpreadInReduce),
        boxed(NoRegexInLoop),
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
