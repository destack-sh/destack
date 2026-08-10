use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer isEmpty over comparisons with zero length.
    pub PREFER_IS_EMPTY {
        id: "prefer-is-empty",
        summary: "Prefer isEmpty over comparisons with zero length",
        explanation: r#"
Comparing a canonical collection or string measurement with zero performs the same empty-state query as `isEmpty` or its negation.
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
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };

        // normalize the canonical measurement to the left operand
        let left = left.source.local_id;
        let right = right.source.local_id;
        let Some(swapped_operator) = operator.swapped() else {
            continue;
        };
        let mut comparison = None;
        for (member, zero, operator) in [(left, right, operator), (right, left, swapped_operator)] {
            let Some((receiver, language_member)) = select_collection_measurement(module, member)?
            else {
                continue;
            };
            if !matches!(
                module.scalar_constant(zero)?,
                Some(dir::ScalarLiteral::Integer(0))
            ) {
                continue;
            }
            let Some(is_negated) = is_empty_negated(operator) else {
                continue;
            };

            comparison = Some((receiver, language_member, is_negated));
            break;
        }
        let Some((receiver, language_member, is_negated)) = comparison else {
            continue;
        };

        // retain the canonical property's defining comparison
        let is_this = matches!(view.get(receiver), dir::Expression::This { .. });
        let is_implementation = module.is_within_language_member(
            expression.into_any(),
            language_member.owner.member("isEmpty"),
        )?;
        if is_this && is_implementation {
            continue;
        }

        // replace the complete comparison with the collection property
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("length or size is compared with zero", span);
        if let Some(suggestion) = suggestion(module, lint, span, receiver, is_negated)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the receiver and identity of one canonical collection measurement.
fn select_collection_measurement(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, dir::LanguageMember)>, ProviderError> {
    let dir::Expression::Member {
        left: receiver,
        is_optional: false,
        ..
    } = module.view().get(expression)
    else {
        return Ok(None);
    };
    let Some(member) = module.language_member(expression)? else {
        return Ok(None);
    };
    if !is_length_or_size(member) {
        return Ok(None);
    }

    Ok(Some((*receiver, member)))
}

/// Return whether an equivalent `isEmpty` query requires negation.
fn is_empty_negated(operator: dir::BinaryOperator) -> Option<bool> {
    match operator {
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => Some(false),
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => Some(true),
        dir::BinaryOperator::LessThanOrEqual => Some(false),
        dir::BinaryOperator::GreaterThan => Some(true),
        _ => None,
    }
}

/// Return whether one canonical member measures a value with `isEmpty`.
fn is_length_or_size(member: dir::LanguageMember) -> bool {
    let owner = member.owner;

    match owner {
        dir::LanguageItem::Array
        | dir::LanguageItem::FixedArray
        | dir::LanguageItem::ReadonlyArray
        | dir::LanguageItem::Slice
        | dir::LanguageItem::String => {
            member == owner.member("length") || member == owner.member("size")
        }
        dir::LanguageItem::Map | dir::LanguageItem::Set => member == owner.member("size"),
        _ => false,
    }
}

/// Build the equivalent collection emptiness query.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    receiver: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let receiver_span = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_span])? {
        return Ok(None);
    }

    // preserve the receiver and express the requested empty state
    let prefix = if is_negated { "!" } else { "" };
    let receiver = module.operand_source(receiver, dir::OperatorPrecedence::Postfix)?;
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
const zero: usize = 0;
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
const zero: usize = 0;
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
function emptyMap(values: Map<string, int32>): boolean {
    return values.size === 0;
}
function emptyString(value: string): boolean {
    return value.length === 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function emptyMap(values: Map<string, int32>): boolean {
    return values.isEmpty;
}
function emptyString(value: string): boolean {
    return value.isEmpty;
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
    comptime Rank: int,
    comptime AxisCount: int,
    comptime OutRank: int = Rank - AxisCount,
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
    for (let index: usize = 0; index < values.length; index += 1) {}
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
 ──▶ main.ds:2:12
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
