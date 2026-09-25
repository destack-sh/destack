use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Unicode code-point operations over UTF-16 code-unit operations.
    pub PREFER_CODE_POINT {
        id: "prefer-code-point",
        summary: "Prefer Unicode code-point operations over UTF-16 code-unit operations",
        explanation: r#"
UTF-16 code-unit operations split characters outside the Basic Multilingual Plane into surrogate pairs.
Instead, you SHOULD use `codePointAt` and `String.fromCodePoint` when processing Unicode characters.
"#,
        example: {
            reported: r#"
function inspectFirstCharacter(text: string): void {
    text.charCodeAt(0);
}
"#,
            accepted: r#"
function inspectFirstCharacter(text: string): void {
    text.codePointAt(0);
}
"#,
        },
        provenance: [Unicorn("prefer-code-point")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical string calls that operate on UTF-16 code units.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical string code-unit operations
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let replacement = match module.language_member(expression)? {
            Some(member) if member == dir::LanguageItem::String.member("charCodeAt") => {
                "codePointAt"
            }
            Some(member) if member == dir::LanguageItem::String.member("fromCharCode") => {
                "fromCodePoint"
            }
            _ => continue,
        };

        // replace only the selected member name
        let span = module.main_span(call.callee.into_any())?;
        let mut diagnostic = lint.diagnostic("string operation uses UTF-16 code units", span);
        let suggestion = suggestion(module, lint, call.callee, replacement)?;
        diagnostic = diagnostic.suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one code-point operation replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    member: dir::LocalNodeId<dir::Expression>,
    replacement: &str,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let span = module.main_span(member.into_any())?;
    let patch = Patch::replace(span, replacement);

    lint.suggestion("use the Unicode code-point operation", patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Suggest the code-point constructor for a Unicode scalar value.
    #[test]
    fn test_replaces_character_constructor() {
        let session = TestSession::dir(
            &PREFER_CODE_POINT,
            r#"
function unicorn(): string {
    return String.fromCharCode(0x41);
}
"#,
        );

        session.assert_suggestions(
            r#"
function unicorn(): string {
    return String.fromCodePoint(0x41);
}
"#,
        );
    }

    /// Accept user methods with the same names.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &PREFER_CODE_POINT,
            r#"
class Encoding {
    charCodeAt(index: isize): uint16 {
        return index.truncate<uint16>();
    }
}
function encode(value: Encoding): uint16 {
    return value.charCodeAt(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
