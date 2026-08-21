use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow repeated construction of the same regular expression.
    pub REPEATED_REGEX_CONSTRUCTION {
        id: "repeated-regex-construction",
        summary: "Disallow repeated construction of the same regular expression",
        explanation: r#"
Evaluating the same constant regular-expression construction inside a loop creates a new matcher for every iteration.
Instead, you SHOULD construct a stateless regular expression before the loop and reuse it.

Global and sticky regular expressions retain matching position, so reconstruction can intentionally reset their state.
"#,
        example: {
            reported: r#"
function containsWord(lines: string[]): boolean {
    for (const line of lines) {
        if (/\w+/.test(line)) {
            return true;
        }
    }
    return false;
}
"#,
            accepted: r#"
function containsWord(lines: string[]): boolean {
    const word = /\w+/;
    for (const line of lines) {
        if (word.test(line)) {
            return true;
        }
    }
    return false;
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report constant stateless regular-expression construction inside loops.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every regular-expression construction owned by an iteration
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        if module.enclosing_iteration(expression.into_any()).is_none() {
            continue;
        }

        // require a constant regex without stateful flags
        if !is_constant_stateless_regex(module, expression)? {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic(
            "constant regular expression is reconstructed in a loop",
            span,
        ));
    }

    Ok(output)
}

/// Return whether one expression constructs a constant regex without global or sticky state.
fn is_constant_stateless_regex(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // recognize authored regular-expression literals directly
    if let dir::Expression::Literal(dir::Literal::RegexString { flags, .. }) = view.get(expression)
    {
        let flags = flags
            .map(|flags| module.dir.strings.get(flags))
            .unwrap_or_default();

        return Ok(!flags.contains(['g', 'y']));
    }

    // recognize canonical construction from a constant string and optional flags
    let dir::Expression::New { arguments, .. } = view.get(expression) else {
        return Ok(false);
    };
    if module.language_item(expression)? != Some(dir::LanguageItem::RegExp) {
        return Ok(false);
    }
    let (pattern, flags) = match arguments.as_slice() {
        [pattern] => (pattern, None),
        [pattern, flags] => (pattern, Some(flags)),
        _ => return Ok(false),
    };
    let Some(pattern) = view.get(*pattern).value() else {
        return Ok(false);
    };
    if !matches!(
        module.scalar_constant(pattern)?,
        Some(dir::Literal::String(_))
    ) {
        return Ok(false);
    }
    let Some(flags) = flags else {
        return Ok(true);
    };
    let Some(flags) = view.get(*flags).value() else {
        return Ok(false);
    };
    let Some(dir::Literal::String(flags)) = module.scalar_constant(flags)? else {
        return Ok(false);
    };
    let flags = module.dir.strings.get(flags);

    Ok(!flags.contains(['g', 'y']))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report direct constant RegExp construction inside a loop.
    #[test]
    fn test_reports_constant_constructor() {
        let session = TestSession::dir(
            &REPEATED_REGEX_CONSTRUCTION,
            r#"
function containsWord(lines: string[]): boolean {
    for (const line of lines) {
        if (new RegExp("[a-z]+").test(line)) {
            return true;
        }
    }
    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[repeated-regex-construction]: constant regular expression is reconstructed in a loop
 ──▶ main.ds:3:13
  │
1 │ function containsWord(lines: string[]): boolean {
2 │     for (const line of lines) {
3 │         if (new RegExp("[a-z]+").test(line)) {
  │             ^^^^^^^^^^^^^^^^^^^^
4 │             return true;
5 │         }
  │
"#,
        );
    }

    /// Report a constant regular expression stored inside a loop.
    #[test]
    fn test_reports_stored_literal() {
        let session = TestSession::dir(
            &REPEATED_REGEX_CONSTRUCTION,
            r#"
function compile(lines: string[]): void {
    for (const _ of lines) {
        const pattern = /[a-z]+/;
        pattern.test("word");
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[repeated-regex-construction]: constant regular expression is reconstructed in a loop
 ──▶ main.ds:3:25
  │
1 │ function compile(lines: string[]): void {
2 │     for (const _ of lines) {
3 │         const pattern = /[a-z]+/;
  │                         ^^^^^^^^
4 │         pattern.test("word");
5 │     }
  │
"#,
        );
    }

    /// Accept stateful regular expressions whose reconstruction resets lastIndex.
    #[test]
    fn test_accepts_stateful_regex() {
        let session = TestSession::dir(
            &REPEATED_REGEX_CONSTRUCTION,
            r#"
function find(lines: string[]): void {
    for (const line of lines) {
        /word/g.exec(line);
        new RegExp("word", "y").test(line);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a constant regex created outside the loop.
    #[test]
    fn test_accepts_reused_regex() {
        let session = TestSession::dir(
            &REPEATED_REGEX_CONSTRUCTION,
            r#"
function containsWord(lines: string[]): boolean {
    const word = /\w+/;
    for (const line of lines) {
        if (word.test(line)) {
            return true;
        }
    }
    return false;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept construction owned by a nested callable.
    #[test]
    fn test_accepts_nested_callable_construction() {
        let session = TestSession::dir(
            &REPEATED_REGEX_CONSTRUCTION,
            r#"
function find(lines: string[]): void {
    for (const line of lines) {
        const matches = (): boolean => /word/.test(line);
        matches();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
