use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer isEmpty over comparisons with zero length.
    pub PREFER_IS_EMPTY {
        id: "prefer-is-empty",
        summary: "Prefer isEmpty over comparisons with zero length",
        explanation: r#"
Zero comparisons over canonical length and size members duplicate the `isEmpty` query.
Instead, you SHOULD use `isEmpty`, negating it when the comparison asks whether elements exist.
"#,
        example: {
            reported: r#"
function empty(values: int32[]): boolean {
    return values.length === 0;
}
"#,
            accepted: r#"
function empty(values: int32[]): boolean {
    return values.isEmpty;
}
"#,
        },
        provenance: [Clippy("len_zero")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical collection lengths compared with zero.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin comparisons with an exact zero operand
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(test) = module.emptiness_test(expression)? else {
            continue;
        };

        // reserve direct filtered existence tests for prefer-array-search
        let is_filtered = module
            .member_call(test.receiver)
            .is_some_and(|call| !call.is_optional() && call.arguments.len() == 1)
            && module.language_member(test.receiver)?
                == Some(dir::LanguageItem::Array.member("filter"));
        if is_filtered {
            continue;
        }

        // retain the canonical property's defining comparison
        let is_this = matches!(view.get(test.receiver), dir::Expression::This);
        let is_implementation = module.is_within_language_member(
            expression.into_any(),
            test.measurement.owner.member("isEmpty"),
        )?;
        if is_this && is_implementation {
            continue;
        }

        // replace the complete comparison with the collection property
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("length or size is compared with zero", span);
        if let Some(suggestion) = suggestion(module, lint, span, test.receiver, !test.is_empty)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the equivalent collection emptiness query.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    receiver: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let receiver_span = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_span])? {
        return Ok(None);
    }

    // preserve the receiver and express the requested empty state
    let prefix = if is_negated { "!" } else { "" };
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let replacement = format!("{prefix}{receiver}.isEmpty");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use the collection emptiness property", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Preserve the meaning of supported zero comparisons in either operand order.
    #[test]
    fn test_replaces_zero_comparisons() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
const zero: isize = 0;
function unequal(values: int32[]): boolean {
    return values.length !== 0;
}
function reversed(values: int32[]): boolean {
    return zero < values.length;
}
function bounded(values: int32[]): boolean {
    return values.length <= 0;
}
"#,
        );

        session.assert_fixes(
            r#"
const zero: isize = 0;
function unequal(values: int32[]): boolean {
    return !values.isEmpty;
}
function reversed(values: int32[]): boolean {
    return !values.isEmpty;
}
function bounded(values: int32[]): boolean {
    return values.isEmpty;
}
"#,
        );
    }

    /// Replace canonical length and size members across collection types.
    #[test]
    fn test_replaces_collection_measurements() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
import { LinkedList } from "tspp:collections";

function emptyMap(values: Map<string, int32>): boolean {
    return values.size === 0;
}
function emptyString(value: string): boolean {
    return value.length === 0;
}
function emptyList(values: LinkedList<int32>): boolean {
    return values.length < 1;
}
"#,
        );

        session.assert_fixes(
            r#"
import { LinkedList } from "tspp:collections";

function emptyMap(values: Map<string, int32>): boolean {
    return values.isEmpty;
}
function emptyString(value: string): boolean {
    return value.isEmpty;
}
function emptyList(values: LinkedList<int32>): boolean {
    return values.isEmpty;
}
"#,
        );
    }

    /// Accept a user-defined length property without canonical `isEmpty` behavior.
    #[test]
    fn test_accepts_user_length_property() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
class Value {
    length: int32 = 0;
}
function empty(value: Value): boolean {
    return value.length === 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore compile-time binary expressions without runtime operator selection.
    #[test]
    fn test_accepts_compile_time_generic_default() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
function reduce<
    const Rank: int,
    const AxisCount: int,
    const OutRank: int = Rank - AxisCount,
>(): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore unrelated comparisons between local bindings and collection lengths.
    #[test]
    fn test_accepts_loop_condition() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
function visit(values: int32[]): void {
    for (let index: isize = 0; index < values.length; index += 1) {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Leave filtered existence tests to the direct Array search rule.
    #[test]
    fn test_accepts_filtered_existence() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
function hasPositive(values: int32[]): boolean {
    return values.filter((value) => value > 0).length > 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments in a length comparison by omitting the fix.
    #[test]
    fn test_reports_commented_length_comparison_without_fix() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
function empty(values: int32[]): boolean {
    return values.length /* retain */ === 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-is-empty]: length or size is compared with zero
 ──▶ main.tspp:2:12
  │
1 │ function empty(values: int32[]): boolean {
2 │     return values.length /* retain */ === 0;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Preserve grouping around a conditional collection receiver.
    #[test]
    fn test_preserves_grouped_collection_receiver() {
        let session = TestSession::dir(
            &PREFER_IS_EMPTY,
            r#"
function populated(isPrimary: boolean, first: int32[], second: int32[]): boolean {
    return (isPrimary ? first : second).length !== 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function populated(isPrimary: boolean, first: int32[], second: int32[]): boolean {
    return !(isPrimary ? first : second).isEmpty;
}
"#,
        );
    }
}
