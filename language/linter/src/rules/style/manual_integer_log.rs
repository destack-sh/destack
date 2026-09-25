use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer integerLog2 over equivalent base-two integer logarithms.
    pub MANUAL_INTEGER_LOG {
        id: "manual-integer-log",
        summary: "Prefer integerLog2 over equivalent base-two integer logarithms",
        explanation: r#"
Subtracting the leading-zero count from an integer width manually computes its base-two logarithm.
Instead, you SHOULD call `integerLog2`, which states the operation directly and preserves exact integer behavior.
"#,
        example: {
            reported: r#"
function magnitude(value: uint32): uint32 {
    return 31 - value.countLeadingZeros();
}
"#,
            accepted: r#"
function magnitude(value: uint32): uint32 {
    return value.integerLog2();
}
"#,
        },
        provenance: [Clippy("manual_ilog2")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One manual computation of a base-two integer logarithm.
struct ManualLog2 {
    /// The complete expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The integer receiver.
    receiver: dir::LocalNodeId<dir::Expression>,
}

/// Report exact manual base-two integer logarithms.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin arithmetic and canonical Integer calls
    for (expression, _) in module.view().iter_nodes::<dir::Expression>() {
        let Some(manual) = manual_log2(module, expression)? else {
            continue;
        };

        let span = module.source_extent(manual.expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("base-two integer logarithm is computed manually", span);
        if let Some(suggestion) = suggestion(module, lint, &manual)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one exact base-two integer logarithm computation.
fn manual_log2(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<ManualLog2>, ProviderError> {
    // recognize value.integerLog(2)
    if let Some(call) = module.member_call(expression)
        && module.language_member(expression)?
            == Some(dir::LanguageItem::Integer.member("integerLog"))
        && !call.is_optional()
        && call.generic_arguments.is_empty()
        && let [argument] = call.arguments
        && let Some(base) = module.view().get(*argument).value()
        && module.integral_constant(base)? == Some(2)
    {
        return Ok(Some(ManualLog2 {
            expression,
            receiver: call.receiver,
        }));
    }

    // recognize one-less-than-width minus countLeadingZeros
    let Some((dir::BinaryOperator::Subtract, [width_expression, zeros])) =
        module.integral_binary(expression)?
    else {
        return Ok(None);
    };
    let Some(call) = module.member_call(zeros) else {
        return Ok(None);
    };
    if module.language_member(zeros)?
        != Some(dir::LanguageItem::Integer.member("countLeadingZeros"))
        || call.is_optional()
        || !call.generic_arguments.is_empty()
        || !call.arguments.is_empty()
    {
        return Ok(None);
    }
    let Some(dir::PrimitiveType::Integer(integer)) =
        module.primitive_type(call.receiver.into_any())?
    else {
        return Ok(None);
    };
    let Some(width) = integer.width() else {
        return Ok(None);
    };
    if integer.is_signed()
        || module.integral_constant(width_expression)? != Some(i64::from(width) - 1)
    {
        return Ok(None);
    }

    Ok(Some(ManualLog2 {
        expression,
        receiver: call.receiver,
    }))
}

/// Replace one manual computation with `integerLog2`.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    manual: &ManualLog2,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(manual.expression.into_any())?;
    let receiver_span = module.source_extent(manual.receiver.into_any())?;
    if module.has_unretained_comment(span, &[receiver_span])? {
        return Ok(None);
    }

    let receiver = module.expression_source(manual.receiver, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(span, format!("{receiver}.integerLog2()"));
    let suggestion = lint.suggestion("use integerLog2", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace width arithmetic with integerLog2.
    #[test]
    fn test_replaces_leading_zero_arithmetic() {
        TestSession::assert_example(&MANUAL_INTEGER_LOG);
    }

    /// Replace integerLog with the constant-base method.
    #[test]
    fn test_replaces_constant_base() {
        let session = TestSession::dir(
            &MANUAL_INTEGER_LOG,
            r#"
function magnitude(value: uint64): uint32 {
    return value.integerLog(2);
}
"#,
        );

        session.assert_suggestions(
            r#"
function magnitude(value: uint64): uint32 {
    return value.integerLog2();
}
"#,
        );
    }

    /// Accept a runtime logarithm base.
    #[test]
    fn test_accepts_runtime_base() {
        let session = TestSession::dir(
            &MANUAL_INTEGER_LOG,
            r#"
function magnitude(value: uint32, base: uint32): uint32 {
    return value.integerLog(base);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept signed leading-zero arithmetic because its domain differs.
    #[test]
    fn test_accepts_signed_arithmetic() {
        let session = TestSession::dir(
            &MANUAL_INTEGER_LOG,
            r#"
function magnitude(value: int32): uint32 {
    return 31 - value.countLeadingZeros();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a mismatched width constant.
    #[test]
    fn test_accepts_other_width() {
        let session = TestSession::dir(
            &MANUAL_INTEGER_LOG,
            r#"
function adjusted(value: uint32): uint32 {
    return 30 - value.countLeadingZeros();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
