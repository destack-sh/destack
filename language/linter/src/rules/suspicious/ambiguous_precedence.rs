use tspp_dir as dir;
use tspp_dir::OperatorPrecedence::{
    Addition, BitwiseAnd, BitwiseOr, BitwiseXor, Comparison, Equality, Exponentiation, LogicalAnd,
    LogicalOr, Multiplication, Shift,
};
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require grouping where operator precedence is easy to misread.
    pub AMBIGUOUS_PRECEDENCE {
        id: "ambiguous-precedence",
        summary: "Require grouping where operator precedence is easy to misread",
        explanation: r#"
Mixed arithmetic, shift, bitwise, comparison, and logical operators can make evaluation order difficult to see.
Instead, you SHOULD parenthesize each nested operation whose precedence is not conventional arithmetic.
"#,
        example: {
            reported: r#"
function mask(offset: int64, width: int64): int64 {
    return 1 << offset + width;
}
"#,
            accepted: r#"
function mask(offset: int64, width: int64): int64 {
    return 1 << (offset + width);
}
"#,
        },
        provenance: [Clippy("precedence")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report ungrouped binary operator combinations with non-obvious precedence.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect each binary parent and its immediate binary operands
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = node
        else {
            continue;
        };

        for operand in [*left, *right] {
            let dir::Expression::Binary {
                operator: nested, ..
            } = view.get(operand)
            else {
                continue;
            };
            if !is_ambiguous(*operator, *nested)
                || module.source_parentheses(operand.into_any()).is_some()
            {
                continue;
            }

            // make the parsed grouping explicit
            let span = module.source_extent(operand.into_any())?;
            let suggestion = suggest_grouping(module, lint, span)?;
            let diagnostic = lint
                .diagnostic("mixed operators rely on implicit precedence", span)
                .suggestion(suggestion);
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return whether one parent and nested binary operator require explicit grouping.
fn is_ambiguous(parent: dir::BinaryOperator, nested: dir::BinaryOperator) -> bool {
    let precedences = (parent.precedence(), nested.precedence());

    matches!(
        precedences,
        (Shift, Addition | Multiplication | Exponentiation)
            | (
                BitwiseAnd | BitwiseXor | BitwiseOr,
                Equality | Comparison | Shift | Addition | Multiplication | Exponentiation
            )
            | (BitwiseXor | BitwiseOr, BitwiseAnd)
            | (BitwiseOr, BitwiseXor)
            | (LogicalOr, LogicalAnd)
    )
}

/// Parenthesize one exact authored operand.
fn suggest_grouping(
    module: &DirModule<'_>,
    lint: &Lint,
    span: tspp_source::Span,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let source = module.source(span)?;
    let patch = Patch::replace(span, format!("({source})"));
    let suggestion = lint.fix("make the evaluation order explicit", patch)?;

    Ok(suggestion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept conventional arithmetic precedence.
    #[test]
    fn test_accepts_mixed_arithmetic() {
        let session = TestSession::dir(
            &AMBIGUOUS_PRECEDENCE,
            r#"
function area(width: int32, height: int32, padding: int32): int32 {
    return width * height + padding;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept explicitly grouped logical operators.
    #[test]
    fn test_accepts_grouped_logical_expression() {
        let session = TestSession::dir(
            &AMBIGUOUS_PRECEDENCE,
            r#"
function ready(primary: boolean, backup: boolean, enabled: boolean): boolean {
    return (primary || backup) && enabled;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Group mixed logical operators according to their parsed order.
    #[test]
    fn test_groups_logical_expression() {
        let session = TestSession::dir(
            &AMBIGUOUS_PRECEDENCE,
            r#"
function ready(primary: boolean, backup: boolean, enabled: boolean): boolean {
    return primary || backup && enabled;
}
"#,
        );

        session.assert_fixes(
            r#"
function ready(primary: boolean, backup: boolean, enabled: boolean): boolean {
    return primary || (backup && enabled);
}
"#,
        );
    }

    /// Group a comparison nested beneath a bitwise operator.
    #[test]
    fn test_groups_bitwise_comparison() {
        let session = TestSession::dir(
            &AMBIGUOUS_PRECEDENCE,
            r#"
import { And, PartialEqual } from "tspp:ops";

struct Logic {
    value: boolean;
}

extension of Logic implements And<boolean> {
    type Output = boolean;

    and(other: boolean): boolean {
        other
    }
}

extension of Logic implements PartialEqual<Logic> {
    equal(&readonly this, other: &readonly Logic): boolean {
        this.value == other.value
    }
}

function combine(left: Logic, right: Logic, expected: Logic): boolean {
    return left & right == expected;
}
"#,
        );

        session.assert_fixes(
            r#"
import { And, PartialEqual } from "tspp:ops";

struct Logic {
    value: boolean;
}

extension of Logic implements And<boolean> {
    type Output = boolean;

    and(other: boolean): boolean {
        other
    }
}

extension of Logic implements PartialEqual<Logic> {
    equal(&readonly this, other: &readonly Logic): boolean {
        this.value == other.value
    }
}

function combine(left: Logic, right: Logic, expected: Logic): boolean {
    return left & (right == expected);
}
"#,
        );
    }

    /// Group a shift nested beneath a bitwise operator.
    #[test]
    fn test_groups_bitwise_shift() {
        let session = TestSession::dir(
            &AMBIGUOUS_PRECEDENCE,
            r#"
function mask(value: int64, offset: int64): int64 {
    return value & 1 << offset;
}
"#,
        );

        session.assert_fixes(
            r#"
function mask(value: int64, offset: int64): int64 {
    return value & (1 << offset);
}
"#,
        );
    }

    /// Group one stronger bitwise operation beneath another.
    #[test]
    fn test_groups_mixed_bitwise_expression() {
        let session = TestSession::dir(
            &AMBIGUOUS_PRECEDENCE,
            r#"
function merge(first: int32, second: int32, third: int32): int32 {
    return first | second & third;
}
"#,
        );

        session.assert_fixes(
            r#"
function merge(first: int32, second: int32, third: int32): int32 {
    return first | (second & third);
}
"#,
        );
    }
}
