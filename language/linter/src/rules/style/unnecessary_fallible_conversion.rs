use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Simplify fallible integer conversions that cannot fail.
    pub UNNECESSARY_FALLIBLE_CONVERSION {
        id: "unnecessary-fallible-conversion",
        summary: "Simplify fallible integer conversions that cannot fail",
        explanation: r#"
A fallible integer conversion adds an undefined case when the source type always fits the destination.
Instead, you SHOULD remove identity conversions and use `as` for lossless widening conversions.
"#,
        example: {
            reported: r#"
function widen(value: int32): int64 | undefined {
    return value.tryInto<int64>();
}
"#,
            accepted: r#"
function widen(value: int32): int64 | undefined {
    return value as int64;
}
"#,
        },
        provenance: [Clippy("unnecessary_fallible_conversions")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical integer `tryInto` calls whose source range fits the destination.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical integer tryInto calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || !call.arguments.is_empty()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Integer.member("tryInto"))
        {
            continue;
        }

        // compare the concrete source and selected destination ranges
        let Some(dir::PrimitiveType::Integer(source)) =
            module.primitive_type(call.receiver.into_any())?
        else {
            continue;
        };
        let Some(bindings) = module.call_generic_bindings(expression)? else {
            continue;
        };
        let Some(destination) = bindings.last() else {
            return Err(ProviderError::internal(
                "Integer.tryInto call has no destination type argument",
            ));
        };
        let destination = module.dir.strip_form(destination.argument)?;
        let dir::Type::Primitive(dir::PrimitiveType::Integer(destination)) =
            module.dir.get_type(destination)?
        else {
            continue;
        };
        if source != destination && !source.widens_to(destination) {
            continue;
        }

        // simplify the conversion that cannot fail
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("integer conversion cannot fail", span);
        if let Some(suggestion) =
            suggestion(module, lint, expression, call.receiver, source, destination)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one infallible integer conversion replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
    source: dir::IntegerType,
    destination: dir::IntegerType,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(span, &[value_span])? {
        return Ok(None);
    }

    // remove an identity conversion or retain one lossless cast
    let (replacement, message) = if source == destination {
        let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;

        (value.into_owned(), "remove the identity conversion")
    } else {
        let value = module.expression_source(value, dir::OperatorPrecedence::Comparison)?;

        (
            format!(
                "{value} as {}",
                dir::PrimitiveType::Integer(destination).as_str()
            ),
            "use the lossless cast",
        )
    };
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.fix(message, patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Replace an unsigned conversion into a wider signed integer.
    #[test]
    fn test_replaces_unsigned_to_signed_widening() {
        let session = TestSession::dir(
            &UNNECESSARY_FALLIBLE_CONVERSION,
            r#"
function widen(value: uint32): int64 | undefined {
    return value.tryInto<int64>();
}
"#,
        );

        session.assert_fixes(
            r#"
function widen(value: uint32): int64 | undefined {
    return value as int64;
}
"#,
        );
    }

    /// Accept narrowing and signed-to-unsigned conversions.
    #[test]
    fn test_accepts_fallible_conversions() {
        let session = TestSession::dir(
            &UNNECESSARY_FALLIBLE_CONVERSION,
            r#"
function narrow(value: int64): int32 | undefined {
    return value.tryInto<int32>();
}
function unsigned(value: int32): uint32 | undefined {
    return value.tryInto<uint32>();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove an identity conversion without retaining a cast.
    #[test]
    fn test_replaces_identical_conversion() {
        let session = TestSession::dir(
            &UNNECESSARY_FALLIBLE_CONVERSION,
            r#"
function convert(value: int32): int32 | undefined {
    return value.tryInto<int32>();
}
"#,
        );

        session.assert_fixes(
            r#"
function convert(value: int32): int32 | undefined {
    return value;
}
"#,
        );
    }
}
