use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow comparing boolean values to boolean literals.
    pub NO_BOOLEAN_LITERAL_COMPARE {
        id: "no-boolean-literal-compare",
        summary: "Disallow comparing boolean values to boolean literals",
        explanation: r#"
For a boolean operand, comparison with `true` returns the operand and comparison with `false` returns its negation.
Instead, you SHOULD use the boolean value directly or negate it.
"#,
        example: {
            reported: r#"
function active(value: boolean): boolean {
    return value === true;
}
"#,
            accepted: r#"
function active(value: boolean): boolean {
    return value;
}
"#,
        },
        provenance: [
            Clippy("bool_comparison"),
            TypeScriptEslint("no-unnecessary-boolean-literal-compare"),
        ],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report comparisons between boolean values and boolean literals.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect equality expressions
    for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = expression
        else {
            continue;
        };
        if !operator.is_equality() {
            continue;
        }

        // require the compiler's builtin equality selection
        let Some(resolution) = module.operator_decision(expression_id.into_any())? else {
            continue;
        };
        if !resolution.is_builtin() {
            continue;
        }

        // select one literal and its compared operand
        let Some(comparison) = BooleanLiteralComparison::select(view, *left, *right, *operator)
        else {
            continue;
        };

        // require the compared value to contain only boolean runtime values
        let Some(operand) = module.builtin_operand(expression_id.into_any(), comparison.value)?
        else {
            continue;
        };
        if !operand
            .scalar_families
            .as_ref()
            .is_some_and(|families| families.is_only_domain(dir::ScalarDomain::Boolean))
        {
            continue;
        }

        // report the redundant literal comparison
        let literal_span = module.span(comparison.literal.into_any())?;
        let mut diagnostic =
            lint.diagnostic("boolean literal comparison is unnecessary", literal_span);
        if let Some(suggestion) = comparison.suggestion(module, expression_id, lint)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One comparison between a boolean literal and another expression.
#[derive(Debug, Clone, Copy)]
struct BooleanLiteralComparison {
    /// The boolean literal expression.
    literal: dir::LocalNodeId<dir::Expression>,
    /// The compared boolean expression.
    value: dir::LocalNodeId<dir::Expression>,
    /// Whether the replacement negates the compared value.
    is_negated: bool,
}

impl BooleanLiteralComparison {
    /// Select one boolean literal comparison.
    fn select(
        view: dir::View<'_>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) -> Option<Self> {
        let (literal, value, literal_value) =
            if let Some(literal_value) = view.get(right).as_boolean() {
                (right, left, literal_value)
            } else {
                let literal_value = view.get(left).as_boolean()?;

                (left, right, literal_value)
            };
        let is_negated = literal_value == operator.is_negative_equality();

        Some(Self {
            literal,
            value,
            is_negated,
        })
    }

    /// Build an automatic replacement that preserves the compared expression.
    fn suggestion(
        self,
        module: &DirModule<'_>,
        comparison: dir::LocalNodeId<dir::Expression>,
        lint: &Lint,
    ) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
        let comparison_span = module.source_extent(comparison.into_any())?;
        let value_span = module.source_extent(self.value.into_any())?;

        // do not discard comments outside the retained expression
        if module.has_unretained_comment(comparison_span, &[value_span])? {
            return Ok(None);
        }

        // preserve the exact authored expression text
        let replacement = if !self.is_negated {
            module.source(value_span)?.to_string()
        } else {
            let value = module.expression_source(self.value, dir::OperatorPrecedence::Prefix)?;

            format!("!{value}")
        };

        // replace the complete comparison
        let mut file_patch = FilePatch::new(comparison_span.file);
        file_patch.replace(comparison_span, replacement);
        let patches = PatchSet::from_files(vec![file_patch]);
        let preserves_type = self.is_negated
            || module
                .dir
                .strip_form(module.node_type_id(comparison.into_any())?)?
                == module
                    .dir
                    .strip_form(module.node_type_id(self.value.into_any())?)?;
        let suggestion = if preserves_type {
            lint.fix("remove the boolean literal comparison", patches)?
        } else {
            lint.suggestion("remove the boolean literal comparison", patches)?
        };

        Ok(Some(suggestion))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace positive equality to true with the compared value.
    #[test]
    fn test_replaces_equal_true() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean): boolean {
    return value == true;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-boolean-literal-compare]: boolean literal comparison is unnecessary
 ──▶ main.ds:2:21
  │
1 │ function active(value: boolean): boolean {
2 │     return value == true;
  │                     ^^^^
3 │ }
  │

 = fix: remove the boolean literal comparison
--- a/main.ds
+++ b/main.ds

    1│ function active(value: boolean): boolean {
-   2│     return value == true;
+   2│     return value;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function active(value: boolean): boolean {
    return value;
}
"#,
        );
    }

    /// Replace reversed positive equality to true with the compared value.
    #[test]
    fn test_replaces_reversed_equal_true() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean): boolean {
    return true == value;
}
"#,
        );

        session.assert_fixes(
            r#"
function active(value: boolean): boolean {
    return value;
}
"#,
        );
    }

    /// Replace inequality to true with the negated value.
    #[test]
    fn test_replaces_not_equal_true() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(value: boolean): boolean {
    return value !== true;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(value: boolean): boolean {
    return !value;
}
"#,
        );
    }

    /// Replace reversed inequality to true with the negated value.
    #[test]
    fn test_replaces_reversed_not_equal_true() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(value: boolean): boolean {
    return true !== value;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(value: boolean): boolean {
    return !value;
}
"#,
        );
    }

    /// Replace equality to false with the negated value.
    #[test]
    fn test_replaces_equal_false() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(value: boolean): boolean {
    return value === false;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(value: boolean): boolean {
    return !value;
}
"#,
        );
    }

    /// Replace reversed equality to false with the negated value.
    #[test]
    fn test_replaces_reversed_equal_false() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(value: boolean): boolean {
    return false === value;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(value: boolean): boolean {
    return !value;
}
"#,
        );
    }

    /// Replace inequality to false with the compared value.
    #[test]
    fn test_replaces_not_equal_false() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean): boolean {
    return value != false;
}
"#,
        );

        session.assert_fixes(
            r#"
function active(value: boolean): boolean {
    return value;
}
"#,
        );
    }

    /// Replace reversed inequality to false with the compared value.
    #[test]
    fn test_replaces_reversed_not_equal_false() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean): boolean {
    return false != value;
}
"#,
        );

        session.assert_fixes(
            r#"
function active(value: boolean): boolean {
    return value;
}
"#,
        );
    }

    /// Keep nullable boolean comparisons whose result cannot be replaced by the operand.
    #[test]
    fn test_accepts_nullable_boolean_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean | undefined): boolean {
    return value === true;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep mixed boolean comparisons whose result cannot be replaced by the operand.
    #[test]
    fn test_accepts_mixed_boolean_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean | string): boolean {
    return value === true;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a comparison whose generic operand is constrained to booleans.
    #[test]
    fn test_replaces_constrained_boolean_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active<T: boolean>(value: T): boolean {
    return value === true;
}
"#,
        );

        session.assert_suggestions(
            r#"
function active<T: boolean>(value: T): boolean {
    return value;
}
"#,
        );
    }

    /// Replace a comparison under a boolean where-clause predicate.
    #[test]
    fn test_replaces_where_constrained_boolean_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active<T>(value: T): boolean where T: boolean {
    return value === true;
}
"#,
        );

        session.assert_suggestions(
            r#"
function active<T>(value: T): boolean where T: boolean {
    return value;
}
"#,
        );
    }

    /// Intersect declared and where-clause bounds before classifying the parameter.
    #[test]
    fn test_replaces_intersected_boolean_parameter_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active<T: boolean | string>(value: T): boolean where T: boolean {
    return value === true;
}
"#,
        );

        session.assert_suggestions(
            r#"
function active<T: boolean | string>(value: T): boolean where T: boolean {
    return value;
}
"#,
        );
    }

    /// Use a method predicate that constrains an outer generic parameter.
    #[test]
    fn test_replaces_outer_parameter_comparison_under_method_predicate() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
class Flag<T> {
    active(value: T): boolean where T: boolean {
        return value === true;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
class Flag<T> {
    active(value: T): boolean where T: boolean {
        return value;
    }
}
"#,
        );
    }

    /// Follow an induced method template to its outer boolean parameter.
    #[test]
    fn test_replaces_outer_parameter_comparison_under_induced_template() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
class Flag<T> {
    active(value: &T): boolean where T: boolean {
        return *value === true;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
class Flag<T> {
    active(value: &T): boolean where T: boolean {
        return *value;
    }
}
"#,
        );
    }

    /// Keep comparisons whose generic operand can also hold strings.
    #[test]
    fn test_accepts_mixed_generic_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active<T: boolean | string>(value: T): boolean {
    return value === true;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a comparison through a reduced boolean alias.
    #[test]
    fn test_replaces_boolean_alias_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
type Flag = boolean;

function active(value: Flag): boolean {
    return value === true;
}
"#,
        );

        session.assert_fixes(
            r#"
type Flag = boolean;

function active(value: Flag): boolean {
    return value;
}
"#,
        );
    }

    /// Fix a comparison over the complete boolean literal union.
    #[test]
    fn test_replaces_boolean_literal_union_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: true | false): boolean {
    return value === true;
}
"#,
        );

        session.assert_fixes(
            r#"
function active(value: true | false): boolean {
    return value;
}
"#,
        );
    }

    /// Replace a statically fixed literal comparison.
    #[test]
    fn test_replaces_constant_literal_comparison() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
const active = true === false;
"#,
        );

        session.assert_fixes(
            r#"
const active = !true;
"#,
        );
    }

    /// Keep equality calls selected through a user-defined implementation.
    #[test]
    fn test_accepts_overloaded_equality() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
import { PartialEqual } from "destack:ops";

struct Flag {
    value: boolean;
}

extension of Flag implements PartialEqual<boolean> {
    equal(&readonly this, other: &readonly boolean): boolean {
        this.value === other
    }
}

declare const flag: Flag;
const active = flag == true;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Parenthesize a lower-precedence value exactly once when negating it.
    #[test]
    fn test_parenthesizes_negated_binary_value() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(left: boolean, right: boolean): boolean {
    return (left && right) === false;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(left: boolean, right: boolean): boolean {
    return !(left && right);
}
"#,
        );
    }

    /// Negate a postfix value without adding parentheses.
    #[test]
    fn test_negates_call_value() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(value: () => boolean): boolean {
    return value() === false;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(value: () => boolean): boolean {
    return !value();
}
"#,
        );
    }

    /// Preserve comments owned by the retained value.
    #[test]
    fn test_preserves_value_comment() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function inactive(value: boolean): boolean {
    return (value /* keep */) === false;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(value: boolean): boolean {
    return !(value /* keep */);
}
"#,
        );
    }

    /// Report without a fix when replacement would discard a comment.
    #[test]
    fn test_preserves_comparison_comment() {
        let session = TestSession::dir(
            &NO_BOOLEAN_LITERAL_COMPARE,
            r#"
function active(value: boolean): boolean {
    return value /* comparison */ === true;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-boolean-literal-compare]: boolean literal comparison is unnecessary
 ──▶ main.ds:2:39
  │
1 │ function active(value: boolean): boolean {
2 │     return value /* comparison */ === true;
  │                                       ^^^^
3 │ }
  │
"#,
        );
    }
}
