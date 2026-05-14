use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow negation of the left operand of relational operators.
    ///
    /// Expressions like `!a in b` are often a mistake, as they parse as `(!a) in b`
    /// instead of the intended `!(a in b)`. The same applies to `is` and
    /// `instanceof`.
    #[lint(
        id = "no-unsafe-negation",
        code = "LC034",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoUnsafeNegation,
    "Disallow negation of left operand in relational operators"
}

impl LintRule for NoUnsafeNegation {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnsafeNegation::meta()
    }

    /// Check module source nodes for unsafe negation on relational operators.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let (left, right_span, operator_name) = match ctx.dir.get(node_id) {
                dir::Expression::Binary {
                    left,
                    operator,
                    right,
                } if is_unsafe_negation_operator(operator) => {
                    let right_span = ctx.dir.get_span(*right);

                    (*left, right_span, binary_operator_name(operator))
                }
                dir::Expression::Is { value, target_type } => {
                    let right_span = ctx.dir.get_span(*target_type);

                    (*value, right_span, "is")
                }
                dir::Expression::InstanceOf { value, target } => {
                    let right_span = ctx.dir.get_span(*target);

                    (*value, right_span, "instanceof")
                }
                _ => continue,
            };

            // check if left side is a logical not and get the inner expression
            if let Some(inner_id) = get_negated_inner(ctx, left) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // make fix: convert `!a in b` to `!(a in b)`
                let expression_span = ctx.dir.get_span(node_id);
                let inner_span = ctx.dir.get_span(inner_id);
                let inner_text = ctx.get_span_text(inner_span);
                let right_text = ctx.get_span_text(right_span);
                let replacement = format!("!({inner_text} {operator_name} {right_text})");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Wrap in parentheses").with_edits(edits);

                ctx.report(
                    LintReport::new(
                        NO_UNSAFE_NEGATION.id,
                        NO_UNSAFE_NEGATION.code,
                        NO_UNSAFE_NEGATION.category,
                        severity,
                        format!("negation of left operand of `{operator_name}`"),
                        expression_span)
                    .label(format!(
                        "this parses as `(!a) {operator_name} b`, use `!(a {operator_name} b)` instead"
                    ))
                    .fix(fix),
                );
            }
        }
    }
}

/// Return true when the binary operator should reject negated left operands.
fn is_unsafe_negation_operator(operator: &dir::BinaryOperator) -> bool {
    matches!(operator, dir::BinaryOperator::In)
}

/// Return one display name for a binary operator.
fn binary_operator_name(operator: &dir::BinaryOperator) -> &'static str {
    match operator {
        dir::BinaryOperator::In => "in",
        dir::BinaryOperator::LessThan => "<",
        dir::BinaryOperator::LessThanOrEqual => "<=",
        dir::BinaryOperator::GreaterThan => ">",
        dir::BinaryOperator::GreaterThanOrEqual => ">=",
        _ => "operator",
    }
}

/// Get the inner expression if this is a logical not operation.
/// Returns the inner expression ID, or None if not a negation.
fn get_negated_inner(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let expression = ctx.dir.get(expression_id);
    match expression {
        dir::Expression::Unary { operator, right } if *operator == dir::UnaryOperator::Not => {
            Some(*right)
        }
        // explicit parenthesized negation: `(!a) in b`
        // this keeps intent explicit and should not be flagged
        dir::Expression::Parenthesized { .. } => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_negation_in_in() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_detects_negation_in_in.ds",
            r#"
let obj = { a: 1 };
let key = "a";
!key in obj;
"#,
        );
        test.result(result).assert_lint("no-unsafe-negation");
    }

    #[test]
    fn test_detects_negation_in_instanceof() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_detects_negation_in_instanceof.ds",
            r#"
class Foo {}
let x = new Foo();
!x instanceof Foo;
"#,
        );
        test.result(result).assert_lint("no-unsafe-negation");
    }

    #[test]
    fn test_detects_negation_in_is() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_detects_negation_in_is.ds",
            r#"
!x is Foo;
"#,
        );
        test.result(result).assert_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_negation_outside_parens() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_allows_negation_outside_parens.ds",
            r#"
let obj = { a: 1 };
let key = "a";
!(key in obj);
"#,
        );
        // ! is outside the parens, so left operand of `in` is `key`, not `!key`
        test.result(result).assert_no_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_non_negated_in() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_allows_non_negated_in.ds",
            r#"
let obj = { a: 1 };
let key = "a";
key in obj;
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_non_negated_instanceof() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_allows_non_negated_instanceof.ds",
            r#"
class Foo {}
let x = new Foo();
x instanceof Foo;
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_parenthesized_negation_left_operand() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_allows_parenthesized_negation_left_operand.ds",
            r#"
let obj = { a: 1 };
let key = "a";
(!key) in obj;
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-negation");
    }

    #[test]
    fn test_fix_in_operator() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_fix_in_operator.ds",
            r#"
let obj = { a: 1 }
let result = !key in obj
"#,
        );
        test.result(result)
            .assert_lint("no-unsafe-negation")
            .assert_safe_fixed(
                r#"
let obj = { a: 1 };
let result = !(key in obj);
"#,
            );
    }

    #[test]
    fn test_fix_instanceof_operator() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_fix_instanceof_operator.ds",
            r#"
class Foo {}
let x = new Foo()
let result = !x instanceof Foo
"#,
        );
        test.result(result)
            .assert_lint("no-unsafe-negation")
            .assert_safe_fixed(
                r#"
class Foo {}
let x = new Foo();
let result = !(x instanceof Foo);
"#,
            );
    }

    #[test]
    fn test_fix_is_operator() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_fix_is_operator.ds",
            r#"
let result = !x is Foo
"#,
        );
        test.result(result)
            .assert_lint("no-unsafe-negation")
            .assert_safe_fixed(
                r#"
let result = !(x is Foo);
"#,
            );
    }

    #[test]
    fn test_allows_negation_in_less_than_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeNegation);
        let result = test.lint(
            "no_unsafe_negation/test_allows_negation_in_less_than_by_default.ds",
            r#"
let threshold = 2;
!value < threshold;
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-negation");
    }
}
