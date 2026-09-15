use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow comparisons with identical deterministic operands.
    pub NO_SELF_COMPARE {
        id: "no-self-compare",
        summary: "Disallow comparisons with identical deterministic operands",
        explanation: r#"
Repeating the same deterministic operand on both sides usually indicates that one of the intended values was duplicated.
Instead, you SHOULD compare the two intended values or call `.isNaN()` when testing floating-point NaN.
"#,
        example: {
            reported: r#"
function changed(value: int32): boolean {
    return value !== value;
}
"#,
            accepted: r#"
function changed(left: int32, right: int32): boolean {
    return left !== right;
}
"#,
        },
        provenance: [Eslint("no-self-compare")],
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report comparisons whose operands repeat the same value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined comparisons
    for expression_id in module.operator_expressions() {
        let expression_id = expression_id?;
        let Some((operator, _)) = module.builtin_binary(expression_id)? else {
            continue;
        };
        if !operator.is_comparison() {
            continue;
        }

        // require the compiler's builtin comparison selection
        let Some(resolution) = module.operator_decision(expression_id.into_any())? else {
            continue;
        };
        let Some([left_operand, right_operand]) = resolution.builtin_operands() else {
            continue;
        };

        // require both operands to repeat one value
        if !module.is_same_operand(left_operand, right_operand)? {
            continue;
        }

        // retain the canonical NaN predicate definition
        let is_nan = dir::LanguageItem::Float.member("isNaN");
        if module.is_within_language_member(expression_id.into_any(), is_nan)? {
            continue;
        }

        // defer exact NaN comparisons to use-isnan
        if module.is_nan(left_operand.source.local_id)? {
            continue;
        }

        // report the complete comparison
        let span = module.span(expression_id.into_any())?;
        let mut diagnostic = lint.diagnostic("comparison has identical operands", span);

        // explain self-comparison when a float operand may be NaN
        if operator.is_equality() {
            let float = dir::ScalarFamily::Domain(dir::ScalarDomain::Float);
            let has_float_family = left_operand.has_scalar_family(float);
            let can_be_nan = has_float_family
                && module
                    .scalar_constant(left_operand.source.local_id)?
                    .is_none_or(|constant| constant.is_nan());
            if can_be_nan {
                let help = if operator.is_negative_equality() {
                    "use a NaN predicate to test for NaN"
                } else {
                    "use a negated NaN predicate to test for non-NaN"
                };
                diagnostic = diagnostic.help(help);
            }
        }

        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Ignore parentheses when comparing stable value paths.
    #[test]
    fn test_reports_parenthesized_binding_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function changed(value: int32): boolean {
    return (value) !== (value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:2:12
  │
1 │ function changed(value: int32): boolean {
2 │     return (value) !== (value);
  │            ^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report the same direct field read on one stable receiver.
    #[test]
    fn test_reports_field_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
struct Point {
    x: int32;
}
function unchanged(point: Point): boolean {
    return point.x === point.x;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:5:12
  │
3 │ }
4 │ function unchanged(point: Point): boolean {
5 │     return point.x === point.x;
  │            ^^^^^^^^^^^^^^^^^^^
6 │ }
  │
"#,
        );
    }

    /// Report identical builtin operations over duplicable operands.
    #[test]
    fn test_reports_builtin_operation_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function unchanged(value: int32): boolean {
    return value + 1 === value + 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:2:12
  │
1 │ function unchanged(value: int32): boolean {
2 │     return value + 1 === value + 1;
  │            ^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report identical builtin unary operations over duplicable operands.
    #[test]
    fn test_reports_builtin_unary_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function unchanged(value: int32): boolean {
    return -value === -value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:2:12
  │
1 │ function unchanged(value: int32): boolean {
2 │     return -value === -value;
  │            ^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report a relational comparison with identical operands.
    #[test]
    fn test_reports_relational_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function ordered(value: int32): boolean {
    return value < value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:2:12
  │
1 │ function ordered(value: int32): boolean {
2 │     return value < value;
  │            ^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Recommend a NaN predicate for the traditional float self-inequality idiom.
    #[test]
    fn test_reports_float_nan_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function isNaN(value: float64): boolean {
    return value !== value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:2:12
  │
1 │ function isNaN(value: float64): boolean {
2 │     return value !== value;
  │            ^^^^^^^^^^^^^^^
3 │ }
  │

 = help: use a NaN predicate to test for NaN
"#,
        );
    }

    /// Recommend a negated NaN predicate for float self-equality.
    #[test]
    fn test_reports_float_non_nan_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function isPresent(value: float64): boolean {
    return value === value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:2:12
  │
1 │ function isPresent(value: float64): boolean {
2 │     return value === value;
  │            ^^^^^^^^^^^^^^^
3 │ }
  │

 = help: use a negated NaN predicate to test for non-NaN
"#,
        );
    }

    /// Leave exact NaN comparisons to use-isnan.
    #[test]
    fn test_accepts_exact_nan_operands() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
const same = NaN === NaN;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep comparisons between distinct bindings.
    #[test]
    fn test_accepts_distinct_bindings() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
function changed(left: int32, right: int32): boolean {
    return left !== right;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep equal field names selected from distinct receivers.
    #[test]
    fn test_accepts_fields_on_distinct_receivers() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
struct Point {
    x: int32;
}

function aligned(left: Point, right: Point): boolean {
    return left.x === right.x;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated calls because each evaluation can produce another value.
    #[test]
    fn test_accepts_repeated_calls() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
declare function next(): int32;

const unchanged = next() === next();
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated overloaded operations because each evaluation invokes user code.
    #[test]
    fn test_accepts_repeated_overloaded_operations() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
import { Multiply } from "destack:ops";

newtype Force = float64;

extension of Force implements Multiply<float64> {
    type Output = float64;

    multiply(other: float64): float64 {
        return other;
    }
}

declare const force: Force;
const unchanged = force * 2.0 === force * 2.0;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep self-comparisons that invoke user-defined equality.
    #[test]
    fn test_accepts_overloaded_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
import { PartialEqual } from "destack:ops";

struct Badge {
    id: int32;
}

extension of Badge implements PartialEqual<Badge> {
    equal(&readonly this, other: &readonly Badge): boolean {
        return this.id == other.id;
    }
}

declare const badge: Badge;
const same = badge == badge;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated accessor reads because each evaluation invokes the getter.
    #[test]
    fn test_accepts_repeated_getter_reads() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
class Counter {
    get value(): int32 {
        return 1;
    }
}

declare const counter: Counter;
const unchanged = counter.value === counter.value;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated dynamic subscripts because key lookup is not a stable field path.
    #[test]
    fn test_accepts_repeated_dynamic_subscripts() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
declare const values: int32[];
declare const index: isize;

const unchanged = values[index] === values[index];
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated array allocations because they create distinct identities.
    #[test]
    fn test_accepts_repeated_array_allocations() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
const unchanged = [1] === [1];
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a literal compared with itself.
    #[test]
    fn test_reports_literal_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
const unchanged = 1 === 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:1:19
  │
1 │ const unchanged = 1 === 1;
  │                   ^^^^^^^
  │
"#,
        );
    }

    /// Report a plain template string through its canonical scalar value.
    #[test]
    fn test_reports_template_string_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
const unchanged = `value` === `value`;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-compare]: comparison has identical operands
 ──▶ main.ds:1:19
  │
1 │ const unchanged = `value` === `value`;
  │                   ^^^^^^^^^^^^^^^^^^^
  │
"#,
        );
    }

    /// Keep interpolated templates because evaluating their values can invoke user code.
    #[test]
    fn test_accepts_interpolated_template_self_comparison() {
        let session = TestSession::dir(
            &NO_SELF_COMPARE,
            r#"
declare function next(): int32;
const unchanged = `${next()}` === `${next()}`;
"#,
        );

        session.assert_no_diagnostics();
    }
}
