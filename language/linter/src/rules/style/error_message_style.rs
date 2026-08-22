use destack_dir as dir;
use destack_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Enforce error message casing and punctuation.
    pub ERROR_MESSAGE_STYLE {
        id: "error-message-style",
        summary: "Enforce error message casing and punctuation",
        explanation: r#"
Capitalized or punctuated failure messages compose poorly with the context that reports them.
Instead, you SHOULD begin failure messages with lowercase prose and omit terminal punctuation.
"#,
        example: {
            reported: r#"
import { panic } from "destack:error";

function fail(): never {
    panic("Operation failed.");
}
"#,
            accepted: r#"
import { panic } from "destack:error";

function fail(): never {
    panic("operation failed");
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report noncanonical literal failure messages.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect message arguments of canonical failure operations
    for expression in module.call_expressions() {
        let expression = expression?;
        let message_index = match module.language_item(expression)? {
            Some(dir::LanguageItem::Abort | dir::LanguageItem::Panic | dir::LanguageItem::Todo) => {
                0
            }
            Some(dir::LanguageItem::Assert) => 1,
            _ => continue,
        };
        let dir::Expression::Call { arguments, .. } = view.get(expression) else {
            continue;
        };
        let Some(argument) = arguments.get(message_index) else {
            continue;
        };
        let Some(message) = view.get(*argument).value() else {
            continue;
        };
        let Some(dir::Literal::String(value)) = module.scalar_constant(message)? else {
            continue;
        };
        let value = module.dir.strings.get(value);
        let Some(replacement) = canonical_message(value) else {
            continue;
        };

        // report the complete literal and replace directly written content
        let span = module.source_extent(message.into_any())?;
        let mut diagnostic = lint
            .diagnostic("failure message has noncanonical style", span)
            .help("begin with lowercase prose and omit terminal punctuation");
        if let Some(source) = canonical_literal(module.source(span)?, value, &replacement) {
            let patch = Patch::replace(span, source);
            let suggestion = lint.suggestion("use canonical failure message style", patch)?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the canonical form of one failure message when it differs.
fn canonical_message(message: &str) -> Option<String> {
    let mut replacement = message.trim_end_matches(['.', '!', '?']).to_string();

    // lowercase an initial prose character while preserving initialisms
    let mut characters = replacement.chars();
    if let Some(first) = characters.next()
        && first.is_uppercase()
        && characters
            .next()
            .is_none_or(|second| !second.is_uppercase())
    {
        let lowercase = first.to_lowercase().collect::<String>();
        replacement.replace_range(0..first.len_utf8(), &lowercase);
    }

    (replacement != message).then_some(replacement)
}

/// Return a rewritten literal when its content contains no source escapes.
fn canonical_literal(source: &str, message: &str, replacement: &str) -> Option<String> {
    let mut characters = source.chars();
    let quote = characters.next()?;
    if !matches!(quote, '\'' | '"') || !source.ends_with(quote) {
        return None;
    }
    let content = source.get(quote.len_utf8()..source.len() - quote.len_utf8())?;
    if content != message {
        return None;
    }

    Some(format!("{quote}{replacement}{quote}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace capitalization and terminal punctuation together.
    #[test]
    fn test_replaces_panic_message() {
        TestSession::assert_example(&ERROR_MESSAGE_STYLE);
    }

    /// Replace an assertion message without changing its condition.
    #[test]
    fn test_replaces_assertion_message() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            r#"
import { assert } from "destack:assert";

function verify(ready: boolean): void {
    assert(ready, "Service must be ready!");
}
"#,
        );

        session.assert_suggestions(
            r#"
import { assert } from "destack:assert";

function verify(ready: boolean): void {
    assert(ready, "service must be ready");
}
"#,
        );
    }

    /// Replace abort and todo messages through their canonical failure operations.
    #[test]
    fn test_replaces_abort_and_todo_messages() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            r#"
import { abort, todo } from "destack:error";

function abortNow(): never {
    abort("Fatal failure.");
}

function finishLater(): never {
    todo("Implement operation.");
}
"#,
        );

        session.assert_suggestions(
            r#"
import { abort, todo } from "destack:error";

function abortNow(): never {
    abort("fatal failure");
}

function finishLater(): never {
    todo("implement operation");
}
"#,
        );
    }

    /// Accept a dynamically computed message.
    #[test]
    fn test_accepts_dynamic_message() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            r#"
import { panic } from "destack:error";

function fail(message: string): never {
    panic(message);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept canonical lowercase prose without terminal punctuation.
    #[test]
    fn test_accepts_canonical_message() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            ERROR_MESSAGE_STYLE.example.accepted.source(),
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an initialism while removing terminal punctuation.
    #[test]
    fn test_preserves_initialism() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            r#"
import { panic } from "destack:error";

function fail(): never {
    panic("HTTP request failed.");
}
"#,
        );

        session.assert_suggestions(
            r#"
import { panic } from "destack:error";

function fail(): never {
    panic("HTTP request failed");
}
"#,
        );
    }

    /// Lowercase an initial Unicode prose character.
    #[test]
    fn test_replaces_unicode_capitalization() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            r#"
import { panic } from "destack:error";

function fail(): never {
    panic("Übertragung fehlgeschlagen");
}
"#,
        );

        session.assert_suggestions(
            r#"
import { panic } from "destack:error";

function fail(): never {
    panic("übertragung fehlgeschlagen");
}
"#,
        );
    }

    /// Report escaped literal content without proposing a lossy edit.
    #[test]
    fn test_reports_escaped_message_without_suggestion() {
        let session = TestSession::dir(
            &ERROR_MESSAGE_STYLE,
            r#"
import { panic } from "destack:error";

function fail(): never {
    panic("Operation\u0020failed.");
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[error-message-style]: failure message has noncanonical style
 ──▶ main.ds:4:11
  │
2 │
3 │ function fail(): never {
4 │     panic("Operation\u0020failed.");
  │           ^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │

 = help: begin with lowercase prose and omit terminal punctuation
"#,
        );
    }
}
