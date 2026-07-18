use crate::LintDefinition;

use super::{correctness, performance, security, style, suspicious};

macro_rules! declare_lint {
    (
        $(#[$attribute:meta])*
        $visibility:vis $name:ident {
            id: $id:literal,
            code: $code:literal,
            description: $description:literal,
            category: $category:ident,
            level: $level:ident,
            fixable: $fixable:ident,
            check: $check:ident($function:path),
        }
    ) => {
        $(#[$attribute])*
        $visibility static $name: $crate::LintDefinition = $crate::LintDefinition {
            id: std::borrow::Cow::Borrowed($id),
            code: std::borrow::Cow::Borrowed($code),
            description: std::borrow::Cow::Borrowed($description),
            category: $crate::LintCategory::$category,
            default_level: destack_repository::LintLevel::$level,
            fixability: $crate::Fixability::$fixable,
            check: $crate::LintCheck::$check($crate::LintImplementation::Builtin($function)),
        };
    };
}

pub(crate) use declare_lint;

/// The complete accepted built-in lint inventory.
#[rustfmt::skip]
pub static BUILTIN_LINTS: &[&LintDefinition] = &[
    // correctness
    &correctness::AWAIT_THENABLE,
    &correctness::BLOCKING_CALL_IN_ASYNC,
    &correctness::CYCLIC_INITIALIZATION,
    &correctness::DEAD_STORE,
    &correctness::FOR_DIRECTION,
    &correctness::NO_APPROX_CONSTANT,
    &correctness::NO_CONTROL_REGEX,
    &correctness::NO_FLOATING_POINT_EQUALITY,
    &correctness::NO_FLOATING_PROMISES,
    &correctness::NO_ITERATOR_INVALIDATION,
    &correctness::NO_SELF_COMPARE,
    &correctness::NO_UNSAFE_NEGATION,
    &correctness::REQUIRE_ARRAY_SORT_COMPARE,
    &correctness::STALE_UPDATE_ACROSS_SUSPENSION,
    &correctness::SUSPENSION_HOLDING_GUARD,
    &correctness::SWAPPED_ARGUMENTS,
    &correctness::UNCONDITIONAL_RECURSION,
    &correctness::UNMODIFIED_LOOP_CONDITION,
    &correctness::USE_ISNAN,

    // performance
    &performance::ALLOCATION_IN_LOOP,
    &performance::CLONE_ON_COPY,
    &performance::DUPLICATE_MONOMORPHIZATION,
    &performance::INDEPENDENT_AWAIT,
    &performance::LARGE_COROUTINE_STATE,
    &performance::LARGE_PASS_BY_VALUE,
    &performance::LARGE_RETURN_BY_VALUE,
    &performance::LARGE_STACK_FRAME,
    &performance::LARGE_VARIANT,
    &performance::MANUAL_COPY,
    &performance::NEEDLESS_BORROW,
    &performance::NEEDLESS_COLLECT,
    &performance::NEEDLESS_MATERIALIZATION,
    &performance::NEEDLESS_PASS_BY_VALUE,
    &performance::NO_SUPER_LINEAR_REGEX,
    &performance::REDUNDANT_CLONE,
    &performance::REDUNDANT_CLOSURE,
    &performance::REPEATED_LINEAR_OPERATION,
    &performance::REPEATED_REGEX_CONSTRUCTION,
    &performance::REPEATED_STRING_GROWTH,

    // security
    &security::HARDCODED_SECRET,
    &security::NO_REGEX_INJECTION,
    &security::NO_TAINTED_SINK,
    &security::UNDOCUMENTED_UNSAFE,

    // style
    &style::COMMENT_CASING,
    &style::COMMENT_LAYOUT,
    &style::COMMENT_PUNCTUATION,
    &style::CONSISTENT_EXTENSION_STYLE,
    &style::DEFAULT_PARAM_LAST,
    &style::DOT_NOTATION,
    &style::EXPLICIT_PUBLIC_TYPES,
    &style::FILENAME_CASE,
    &style::GROUPED_ACCESSOR_PAIRS,
    &style::NO_BOOLEAN_LITERAL_COMPARE,
    &style::NO_COLLAPSIBLE_IF,
    &style::NO_ELSE_RETURN,
    &style::NO_LONELY_IF,
    &style::NO_NEGATED_CONDITION,
    &style::NO_NESTED_TERNARY,
    &style::NO_UNNECESSARY_TEMPLATE_EXPRESSION,
    &style::NO_UNNEEDED_TERNARY,
    &style::OBJECT_SHORTHAND,
    &style::OPERATOR_ASSIGNMENT,
    &style::PREFER_CONST,
    &style::PREFER_EXPONENTIATION_OPERATOR,
    &style::PREFER_EXPRESSION,
    &style::PREFER_FRAGMENT_SHORTHAND,
    &style::PREFER_IMPLICIT_RETURN,
    &style::PREFER_LOOP,
    &style::PREFER_MATCH,
    &style::PREFER_NULLISH_COALESCING,
    &style::PREFER_OPTIONAL_CHAIN,
    &style::PREFER_READONLY,
    &style::PREFER_SELF_CLOSING_TREE,
    &style::PREFER_STRUCT,
    &style::PREFER_STRUCT_LITERAL,
    &style::PREFER_TEMPLATE,
    &style::PREFER_TUPLE_DESTRUCTURE,
    &style::PREFER_TUPLE_SWAP,
    &style::PREFER_UNARY_NEGATION,
    &style::YODA,

    // suspicious
    &suspicious::AMBIGUOUS_PRECEDENCE,
    &suspicious::CYCLIC_DEPENDENCY,
    &suspicious::DEAD_TARGET_SYMBOL,
    &suspicious::LOOP_SINGLE_ITERATION,
    &suspicious::MIXED_READ_WRITE_EXPRESSION,
    &suspicious::NO_COND_ASSIGN,
    &suspicious::NO_DEBUGGER,
    &suspicious::NO_DUPLICATE_CODE,
    &suspicious::NO_DUPLICATE_ELSE_IF,
    &suspicious::NO_DUPLICATE_MATCH_ARMS,
    &suspicious::NO_EMPTY,
    &suspicious::NO_EMPTY_FUNCTION,
    &suspicious::NO_EMPTY_PATTERN,
    &suspicious::NO_EMPTY_STATIC_BLOCK,
    &suspicious::NO_IDENTICAL_BRANCHES,
    &suspicious::NO_MEANINGLESS_VOID,
    &suspicious::NO_MISLEADING_CHARACTER_CLASS,
    &suspicious::NO_NEGATION_IN_EQUALITY_CHECK,
    &suspicious::NO_REDUNDANT_PATTERN,
    &suspicious::NO_RETURN_ASSIGN,
    &suspicious::NO_SELF_ASSIGN,
    &suspicious::NO_TEMPLATE_CURLY_IN_STRING,
    &suspicious::NO_USELESS_BACKREFERENCE,
    &suspicious::NO_USELESS_COMPUTED_KEY,
    &suspicious::NO_USELESS_CONCAT,
    &suspicious::NO_USELESS_CONSTRUCTOR,
    &suspicious::NO_USELESS_ESCAPE,
    &suspicious::NO_USELESS_LENGTH_CHECK,
    &suspicious::NO_USELESS_RENAME,
    &suspicious::NO_USELESS_RETURN,
    &suspicious::NO_USELESS_SPREAD,
    &suspicious::NO_USELESS_SPREAD_FALLBACK,
    &suspicious::NO_USELESS_SWITCH_CASE,
    &suspicious::ONLY_USED_IN_RECURSION,
    &suspicious::REQUIRE_AWAIT,
    &suspicious::SIGNIFICANT_DROP_IN_SCRUTINEE,
    &suspicious::UNREACHABLE_EXPORT,
    &suspicious::UNUSED_DEPENDENCY,
];

/// Return the complete accepted built-in lint inventory.
pub const fn builtin_inventory() -> &'static [&'static LintDefinition] {
    BUILTIN_LINTS
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use destack_repository::LintLevel;

    use super::*;
    use crate::LintCategory;

    /// Keep the accepted inventory canonical and unambiguous.
    #[test]
    fn test_validate_builtin_inventory() {
        let categories = [
            (LintCategory::Correctness, "LC"),
            (LintCategory::Performance, "LP"),
            (LintCategory::Security, "LS"),
            (LintCategory::Style, "LY"),
            (LintCategory::Suspicious, "LU"),
        ];
        let mut ids = HashSet::new();
        let mut codes = HashSet::new();
        let mut index = 0;

        // validate each category as one alphabetic inventory block
        for (category, prefix) in categories {
            let start = index;
            while index < BUILTIN_LINTS.len() && BUILTIN_LINTS[index].category == category {
                let definition = BUILTIN_LINTS[index];
                assert!(ids.insert(definition.id.as_ref()));
                assert!(codes.insert(definition.code.as_ref()));
                assert!(definition.code.starts_with(prefix));
                assert_ne!(definition.default_level, LintLevel::Off);
                index += 1;
            }
            assert!(index > start);

            for pair in BUILTIN_LINTS[start..index].windows(2) {
                assert!(pair[0].id < pair[1].id);
            }
        }

        assert_eq!(index, BUILTIN_LINTS.len());
    }
}
