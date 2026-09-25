use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `String.replaceAll` when every occurrence is replaced.
    pub PREFER_STRING_REPLACE_ALL {
        id: "prefer-string-replace-all",
        summary: "Prefer `String.replaceAll` when every occurrence is replaced",
        explanation: r#"
A global replacement or split-and-join chain expresses replacement of every occurrence indirectly.
Instead, you SHOULD use `String.replaceAll` to state that operation directly.

Splitting on an empty string has different behavior and is left unchanged.
"#,
        example: {
            reported: r#"
function redact(text: string): string {
    return text.replace(/secret/g, "***");
}
"#,
            accepted: r#"
function redact(text: string): string {
    return text.replaceAll(/secret/g, "***");
}
"#,
        },
        provenance: [Unicorn("prefer-string-replace-all")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical string operations that replace every occurrence indirectly.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical string calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let member = module.language_member(expression)?;

        // replace global regular-expression calls directly
        if member == Some(dir::LanguageItem::String.member("replace")) {
            report_global_replace(module, lint, expression, &mut output)?;
        }
        // replace split-and-join chains
        else if member == Some(dir::LanguageItem::Array.member("join")) {
            report_split_join(module, lint, expression, &mut output)?;
        }
    }

    Ok(output)
}

/// Report one `replace` call with a global regular-expression literal.
fn report_global_replace(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();
    let Some(call) = module.member_call(expression) else {
        return Ok(());
    };
    if call.is_optional() {
        return Ok(());
    }

    // require a global regular-expression literal as the search value
    let [pattern, _] = call.arguments else {
        return Ok(());
    };
    let Some(pattern) = view.get(*pattern).value() else {
        return Ok(());
    };
    let dir::Expression::Literal(dir::Literal::RegexString { flags, .. }) = view.get(pattern)
    else {
        return Ok(());
    };
    let is_global = flags.is_some_and(|flags| module.dir.strings.get(flags).contains('g'));
    if !is_global {
        return Ok(());
    }

    // replace only the selected method name
    let span = module.source_extent(expression.into_any())?;
    let mut diagnostic = lint.diagnostic("global regular expression uses String.replace", span);
    let member = module.main_span(call.callee.into_any())?;
    let patch = Patch::replace(member, "replaceAll");
    let fix = lint.fix("use String.replaceAll", patch)?;
    diagnostic = diagnostic.suggestion(fix);
    output.report(diagnostic);

    Ok(())
}

/// Report one `split(separator).join(replacement)` chain.
fn report_split_join(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();
    let Some(join) = module.member_call(expression) else {
        return Ok(());
    };
    if join.is_optional() {
        return Ok(());
    }

    // require one replacement and a canonical one-argument string split
    let [replacement] = join.arguments else {
        return Ok(());
    };
    let Some(replacement) = view.get(*replacement).value() else {
        return Ok(());
    };
    let Some(split) = module.member_call(join.receiver) else {
        return Ok(());
    };
    if split.is_optional()
        || module.language_member(join.receiver)? != Some(dir::LanguageItem::String.member("split"))
    {
        return Ok(());
    }
    let [separator] = split.arguments else {
        return Ok(());
    };
    let Some(separator) = view.get(*separator).value() else {
        return Ok(());
    };

    // exclude an empty or runtime-varying separator
    let Some(dir::Literal::String(value)) = module.scalar_constant(separator)? else {
        return Ok(());
    };
    if module.dir.strings.get(value).is_empty() {
        return Ok(());
    }

    // replace the complete chain while retaining its three evaluated expressions
    let span = module.source_extent(expression.into_any())?;
    let mut diagnostic = lint.diagnostic("split-and-join chain replaces every occurrence", span);
    if let Some(fix) = split_join_fix(
        module,
        lint,
        expression,
        split.receiver,
        separator,
        replacement,
    )? {
        diagnostic = diagnostic.suggestion(fix);
    }
    output.report(diagnostic);

    Ok(())
}

/// Build one direct `String.replaceAll` call.
fn split_join_fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
    separator: dir::LocalNodeId<dir::Expression>,
    replacement: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_span = module.source_extent(receiver.into_any())?;
    let separator_span = module.source_extent(separator.into_any())?;
    let replacement_span = module.source_extent(replacement.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_span, separator_span, replacement_span])? {
        return Ok(None);
    }

    // retain exact sources and group the postfix receiver when required
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let separator = module.source(separator_span)?;
    let replacement_source =
        module.expression_source(replacement, dir::OperatorPrecedence::Lowest)?;
    let replacement = match module.scalar_constant(replacement)? {
        Some(dir::Literal::String(value)) if !module.dir.strings.get(value).contains('$') => {
            replacement_source.into_owned()
        }
        _ if module.is_speculatable_expression(replacement)? => {
            format!("() => {replacement_source}")
        }
        _ => return Ok(None),
    };
    let replacement = format!("{receiver}.replaceAll({separator}, {replacement})");
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("use String.replaceAll", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a global regular-expression replacement directly.
    #[test]
    fn test_replaces_global_replacement() {
        let session = TestSession::dir(
            &PREFER_STRING_REPLACE_ALL,
            r#"
function redact(text: string): string {
    return text.replace(/secret/g, "***");
}
"#,
        );

        session.assert_fixes(
            r#"
function redact(text: string): string {
    return text.replaceAll(/secret/g, "***");
}
"#,
        );
    }

    /// Replace a split-and-join chain over a nonempty string separator.
    #[test]
    fn test_replaces_split_join() {
        let session = TestSession::dir(
            &PREFER_STRING_REPLACE_ALL,
            r#"
function rename(text: string): string {
    return text.split("old").join("new");
}
"#,
        );

        session.assert_fixes(
            r#"
function rename(text: string): string {
    return text.replaceAll("old", "new");
}
"#,
        );
    }

    /// Preserve dollar-prefixed replacement text with a replacement function.
    #[test]
    fn test_preserves_replacement_syntax() {
        let session = TestSession::dir(
            &PREFER_STRING_REPLACE_ALL,
            r#"
function rename(text: string): string {
    return text.split("old").join("$&");
}
"#,
        );

        session.assert_fixes(
            r#"
function rename(text: string): string {
    return text.replaceAll("old", () => "$&");
}
"#,
        );
    }

    /// Accept a non-global regular-expression replacement.
    #[test]
    fn test_accepts_single_replacement() {
        let session = TestSession::dir(
            &PREFER_STRING_REPLACE_ALL,
            r#"
function redact(text: string): string {
    return text.replace(/secret/, "***");
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept split-and-join over an empty separator.
    #[test]
    fn test_accepts_empty_split() {
        let session = TestSession::dir(
            &PREFER_STRING_REPLACE_ALL,
            r#"
function separate(text: string): string {
    return text.split("").join("-");
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
