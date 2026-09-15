use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer saturating arithmetic over equivalent manual bounds logic.
    pub MANUAL_SATURATING_ARITHMETIC {
        id: "manual-saturating-arithmetic",
        summary: "Prefer saturating arithmetic over equivalent manual bounds logic",
        explanation: r#"
Falling back from overflow-checked unsigned arithmetic to the matching integer bound manually implements saturation.
Instead, you SHOULD call the corresponding saturating arithmetic method.
"#,
        example: {
            reported: r#"
function add(left: uint32, right: uint32): uint32 {
    return left.checkedAdd(right) ?? uint32.maximum();
}
"#,
            accepted: r#"
function add(left: uint32, right: uint32): uint32 {
    return left.saturatingAdd(right);
}
"#,
        },
        provenance: [Clippy("manual_saturating_arithmetic")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One overflow-checked unsigned operation followed by its saturating bound.
struct ManualSaturation {
    /// The complete coalescing expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The arithmetic receiver.
    receiver: dir::LocalNodeId<dir::Expression>,
    /// The other arithmetic operand.
    argument: dir::LocalNodeId<dir::Expression>,
    /// The saturating method.
    method: &'static str,
}

/// Report overflow-checked arithmetic with a matching bound fallback.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin nullish coalescing expressions
    for expression in module.operator_expressions() {
        let expression = expression?;
        let dir::Expression::Binary {
            left: checked,
            operator: dir::BinaryOperator::Coalesce,
            right: bound,
        } = module.view().get(expression)
        else {
            continue;
        };
        let Some(manual) = manual_saturation(module, expression, *checked, *bound)? else {
            continue;
        };

        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("arithmetic falls back to its saturating bound", span);
        if let Some(suggestion) = suggestion(module, lint, &manual)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one exact overflow-checked operation and matching bound fallback.
fn manual_saturation(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    checked: dir::LocalNodeId<dir::Expression>,
    bound: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<ManualSaturation>, ProviderError> {
    let Some(operation_call) = module.member_call(checked) else {
        return Ok(None);
    };
    let Some(bound_call) = module.member_call(bound) else {
        return Ok(None);
    };
    if operation_call.is_optional()
        || !operation_call.generic_arguments.is_empty()
        || bound_call.is_optional()
        || !bound_call.generic_arguments.is_empty()
        || !bound_call.arguments.is_empty()
    {
        return Ok(None);
    }
    let [argument] = operation_call.arguments else {
        return Ok(None);
    };
    let Some(argument) = module.view().get(*argument).value() else {
        return Ok(None);
    };

    // require one unsigned builtin integer type throughout
    let Some(dir::PrimitiveType::Integer(integer)) =
        module.primitive_type(operation_call.receiver.into_any())?
    else {
        return Ok(None);
    };
    if integer.is_signed()
        || module.primitive_type(bound.into_any())? != Some(dir::PrimitiveType::Integer(integer))
    {
        return Ok(None);
    }

    // pair each overflow-checked operation with the bound that has exact saturating behavior
    let operation = module.language_member(checked)?;
    let bound = module.language_member(bound)?;
    let method = match (operation, bound) {
        (Some(operation), Some(bound))
            if operation == dir::LanguageItem::Integer.member("checkedAdd")
                && bound == dir::LanguageItem::Integer.member("maximum") =>
        {
            "saturatingAdd"
        }
        (Some(operation), Some(bound))
            if operation == dir::LanguageItem::Integer.member("checkedSubtract")
                && bound == dir::LanguageItem::Integer.member("minimum") =>
        {
            "saturatingSubtract"
        }
        (Some(operation), Some(bound))
            if operation == dir::LanguageItem::Integer.member("checkedMultiply")
                && bound == dir::LanguageItem::Integer.member("maximum") =>
        {
            "saturatingMultiply"
        }
        (Some(operation), Some(bound))
            if operation == dir::LanguageItem::Integer.member("checkedPower")
                && bound == dir::LanguageItem::Integer.member("maximum") =>
        {
            "saturatingPower"
        }
        _ => return Ok(None),
    };

    Ok(Some(ManualSaturation {
        expression,
        receiver: operation_call.receiver,
        argument,
        method,
    }))
}

/// Replace overflow-checked arithmetic and its fallback with one saturating call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    manual: &ManualSaturation,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(manual.expression.into_any())?;
    let receiver_span = module.source_extent(manual.receiver.into_any())?;
    let argument_span = module.source_extent(manual.argument.into_any())?;
    if module.has_unretained_comment(span, &[receiver_span, argument_span])? {
        return Ok(None);
    }

    let receiver = module.expression_source(manual.receiver, dir::OperatorPrecedence::Postfix)?;
    let argument = module.expression_source(manual.argument, dir::OperatorPrecedence::Lowest)?;
    let method = manual.method;
    let patch = Patch::replace(span, format!("{receiver}.{method}({argument})"));
    let suggestion = lint.suggestion("use saturating arithmetic", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace overflow-checked addition with saturatingAdd.
    #[test]
    fn test_replaces_checked_addition() {
        TestSession::assert_example(&MANUAL_SATURATING_ARITHMETIC);
    }

    /// Replace overflow-checked subtraction with saturatingSubtract.
    #[test]
    fn test_replaces_checked_subtraction() {
        let session = TestSession::dir(
            &MANUAL_SATURATING_ARITHMETIC,
            r#"
function subtract(left: uint16, right: uint16): uint16 {
    return left.checkedSubtract(right) ?? uint16.minimum();
}
"#,
        );

        session.assert_suggestions(
            r#"
function subtract(left: uint16, right: uint16): uint16 {
    return left.saturatingSubtract(right);
}
"#,
        );
    }

    /// Replace overflow-checked multiplication with saturatingMultiply.
    #[test]
    fn test_replaces_checked_multiplication() {
        let session = TestSession::dir(
            &MANUAL_SATURATING_ARITHMETIC,
            r#"
function multiply(left: uint64, right: uint64): uint64 {
    return left.checkedMultiply(right) ?? uint64.maximum();
}
"#,
        );

        session.assert_suggestions(
            r#"
function multiply(left: uint64, right: uint64): uint64 {
    return left.saturatingMultiply(right);
}
"#,
        );
    }

    /// Replace overflow-checked exponentiation with saturatingPower.
    #[test]
    fn test_replaces_checked_power() {
        let session = TestSession::dir(
            &MANUAL_SATURATING_ARITHMETIC,
            r#"
function power(value: uint32, exponent: uint32): uint32 {
    return value.checkedPower(exponent) ?? uint32.maximum();
}
"#,
        );

        session.assert_suggestions(
            r#"
function power(value: uint32, exponent: uint32): uint32 {
    return value.saturatingPower(exponent);
}
"#,
        );
    }

    /// Accept a fallback that does not match the saturation direction.
    #[test]
    fn test_accepts_other_bound() {
        let session = TestSession::dir(
            &MANUAL_SATURATING_ARITHMETIC,
            r#"
function add(left: uint32, right: uint32): uint32 {
    return left.checkedAdd(right) ?? uint32.minimum();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept signed arithmetic because overflow direction affects its bound.
    #[test]
    fn test_accepts_signed_arithmetic() {
        let session = TestSession::dir(
            &MANUAL_SATURATING_ARITHMETIC,
            r#"
function add(left: int32, right: int32): int32 {
    return left.checkedAdd(right) ?? int32.maximum();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an overflow-checked operation without a fallback.
    #[test]
    fn test_accepts_checked_arithmetic() {
        let session = TestSession::dir(
            &MANUAL_SATURATING_ARITHMETIC,
            r#"
function add(left: uint32, right: uint32): uint32 | undefined {
    return left.checkedAdd(right);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
