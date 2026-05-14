use crate::LintMeta;
use destack_dir::{self as dir, Argument, Expression, ScalarLiteral};
use destack_source::LanguageType;
use destack_workspace::LintSeverity;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest tuple type for fixed-length heterogeneous arrays.
    ///
    /// In Destack, tuples are the preferred way to represent fixed-length
    /// collections with different element types. This lint detects array
    /// literals with clearly heterogeneous elements and suggests using
    /// tuple form instead.
    ///
    /// ## Bad
    /// ```
    /// let data = ["hello", 42, true]
    /// ```
    ///
    /// ## Good
    /// ```
    /// let data = ("hello", 42, true)
    /// ```
    ///
    /// This lint only applies to Destack files (.ds).
    #[lint(
        id = "prefer-tuple",
        code = "LY058",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferTuple,
    "Suggest tuple type for fixed-length heterogeneous arrays"
}

impl LintRule for PreferTuple {
    fn meta(&self) -> &'static LintMeta {
        PreferTuple::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // only applies to Destack files
        if !matches!(
            ctx.module.language_type,
            Some(LanguageType::Destack | LanguageType::DestackDeclaration)
        ) {
            return;
        }

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let Expression::ArrayExpression { elements } = ctx.dir.get(node_id) else {
                continue;
            };

            // need at least 2 elements to be heterogeneous
            if elements.len() < 2 {
                continue;
            }

            // limit to reasonably sized arrays (tuples with too many elements are rare)
            if elements.len() > 10 {
                continue;
            }

            // get literal types of elements
            let element_types: Vec<_> = elements
                .iter()
                .filter_map(|argument_id| {
                    let arg = ctx.dir.get(*argument_id);
                    get_argument_literal_type(ctx, arg)
                })
                .collect();

            // all elements must be literals for this lint to apply
            if element_types.len() != elements.len() {
                continue;
            }

            // check if there are different types
            if !has_different_types(&element_types) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.dir.get_span(node_id);
            let array_text = ctx.get_span_text(span);

            // create fix: replace [ ] with ( )
            let tuple_text = if let Some(inner) = array_text.strip_prefix('[') {
                if let Some(content) = inner.strip_suffix(']') {
                    format!("({content})")
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let edits = ctx.edit_builder().replace(span, tuple_text).into_edits();
            let fix = LintFix::safe("Convert to tuple").with_edits(edits);

            ctx.report(
                LintReport::new(
                    PREFER_TUPLE.id,
                    PREFER_TUPLE.code,
                    PREFER_TUPLE.category,
                    severity,
                    "array has heterogeneous element types",
                    span,
                )
                .label("use tuple form instead")
                .fix(fix),
            );
        }
    }
}

/// Literal type categories for comparison.
#[derive(Debug, PartialEq)]
enum LiteralType {
    String,
    Number,
    Boolean,
    Character,
}

/// Get the literal type of an argument if it's a literal.
fn get_argument_literal_type(ctx: &LintModuleContext<'_>, arg: &Argument) -> Option<LiteralType> {
    let value_id = match arg {
        Argument::Positional { value, .. } => *value,
        _ => return None,
    };

    let value = ctx.dir.get(value_id);
    match value {
        Expression::ScalarLiteral(literal) => match literal {
            ScalarLiteral::Null => None,
            ScalarLiteral::String(_) => Some(LiteralType::String),
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Bigint(_) => {
                Some(LiteralType::Number)
            }
            ScalarLiteral::Boolean(_) => Some(LiteralType::Boolean),
            ScalarLiteral::Character(_) => Some(LiteralType::Character),
            ScalarLiteral::RegexString { .. } => None, // regex is complex, skip
        },
        _ => None,
    }
}

/// Check if the literal types contain different types.
fn has_different_types(types: &[LiteralType]) -> bool {
    if types.is_empty() {
        return false;
    }

    let first = &types[0];
    types.iter().any(|t| t != first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_heterogeneous_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_detects_heterogeneous_array.ds",
            r#"
let data = ["hello", 42, true]
"#,
        );
        test.result(result).assert_lint("prefer-tuple");
    }

    #[test]
    fn test_detects_string_number_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_detects_string_number_array.ds",
            r#"
let pair = ["name", 123]
"#,
        );
        test.result(result).assert_lint("prefer-tuple");
    }

    #[test]
    fn test_allows_homogeneous_string_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_allows_homogeneous_string_array.ds",
            r#"
let names = ["alice", "bob", "charlie"]
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple");
    }

    #[test]
    fn test_allows_homogeneous_number_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_allows_homogeneous_number_array.ds",
            r#"
let nums = [1, 2, 3]
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple");
    }

    #[test]
    fn test_allows_single_element_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_allows_single_element_array.ds",
            r#"
let single = [42]
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple");
    }

    #[test]
    fn test_allows_empty_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_allows_empty_array.ds",
            r#"
let empty = []
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple");
    }

    #[test]
    fn test_allows_non_literal_array() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_allows_non_literal_array.ds",
            r#"
let data = [x, y, z]
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple");
    }

    #[test]
    fn test_fix_converts_to_tuple() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_fix_converts_to_tuple.ds",
            r#"
let data = ["hello", 42];
"#,
        );
        test.result(result)
            .assert_lint("prefer-tuple")
            .assert_safe_fixed(
                r#"
let data = ("hello", 42,);
"#,
            );
    }

    #[test]
    fn test_ignores_typescript_files() {
        let test = TestProgram::for_rule_without_prelude(PreferTuple);
        let result = test.lint(
            "prefer_tuple/test_ignores_typescript_files.ts",
            r#"
let data = ["hello", 42, true]
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple");
    }
}
