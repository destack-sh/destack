use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow unwrap after control flow proves the Result variant.
    pub UNNECESSARY_UNWRAP {
        id: "unnecessary-unwrap",
        summary: "Disallow unwrap after control flow proves the Result variant",
        explanation: r#"
Unwrapping a Result after control flow has already narrowed it repeats a variant check that is known to succeed.
Instead, you SHOULD read the narrowed variant payload directly.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): int32 {
    if (result.kind === "Ok") {
        return result.unwrap();
    }
    return 0;
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): int32 {
    if (result.kind === "Ok") {
        return result.value;
    }
    return 0;
}
"#,
        },
        provenance: [Clippy("unnecessary_unwrap")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Result unwrap calls whose receiver is already a variant.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical unwrap calls without arguments
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() || !call.arguments.is_empty() || !call.generic_arguments.is_empty() {
            continue;
        }
        let (variant, field) = match module.language_member(expression)? {
            Some(member) if member == dir::LanguageItem::Result.member("unwrap") => {
                (dir::LanguageItem::Ok, "value")
            }
            Some(member) if member == dir::LanguageItem::Result.member("unwrapErr") => {
                (dir::LanguageItem::Err, "error")
            }
            _ => continue,
        };

        // rely exclusively on the narrowed receiver type
        let receiver = module.adjusted_type_id(call.receiver.into_any())?;
        if !module.dir.represents_item(receiver, variant)? {
            continue;
        }

        // replace the unwrap call with direct payload access
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Result variant is already proven", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, call.receiver, field)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one direct narrowed payload access.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    receiver: dir::LocalNodeId<dir::Expression>,
    field: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping around the narrowed Result
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{receiver}.{field}"));
    let suggestion = lint.fix("read the narrowed Result payload", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace unwrapErr after failed Result narrowing.
    #[test]
    fn test_replaces_failed_unwrap() {
        let session = TestSession::dir(
            &UNNECESSARY_UNWRAP,
            r#"
function error(result: Result<int32, string>): string {
    if (result.kind === "Err") {
        return result.unwrapErr();
    }
    return "";
}
"#,
        );

        session.assert_fixes(
            r#"
function error(result: Result<int32, string>): string {
    if (result.kind === "Err") {
        return result.error;
    }
    return "";
}
"#,
        );
    }

    /// Accept unwrap without prior narrowing.
    #[test]
    fn test_accepts_unproven_unwrap() {
        let session = TestSession::dir(
            &UNNECESSARY_UNWRAP,
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrap();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
