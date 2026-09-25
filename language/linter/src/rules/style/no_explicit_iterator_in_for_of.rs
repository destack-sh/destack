use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow explicit iterator calls in for-of loops.
    pub NO_EXPLICIT_ITERATOR_IN_FOR_OF {
        id: "no-explicit-iterator-in-for-of",
        summary: "Disallow explicit iterator calls in for-of loops",
        explanation: r#"
A for-of loop obtains an iterable's iterator automatically.
Instead, you SHOULD iterate over the iterable directly.
"#,
        example: {
            reported: r#"
function sum(values: int32[]): int32 {
    let total: int32 = 0;
    for (const value of values.iterator()) {
        total += value;
    }
    return total;
}
"#,
            accepted: r#"
function sum(values: int32[]): int32 {
    let total: int32 = 0;
    for (const value of values) {
        total += value;
    }
    return total;
}
"#,
        },
        provenance: [Clippy("explicit_iter_loop")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report for-of loops over explicit canonical iterator calls.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect synchronous for-of loops
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(loop_) = module.for_of(expression) else {
            continue;
        };
        if loop_.asynchrony != dir::Asynchrony::Sync {
            continue;
        }
        let Some(receiver) = module.iterator_receiver(loop_.iterator)? else {
            continue;
        };

        // remove the redundant iterator call
        let span = module.source_extent(loop_.iterator.into_any())?;
        let mut diagnostic = lint.diagnostic("for-of calls iterator explicitly", span);
        if let Some(suggestion) = suggestion(module, lint, loop_.iterator, receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one explicit iterator call with its iterable receiver.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    iterator: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(iterator.into_any())?;
    let receiver = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[receiver])? {
        return Ok(None);
    }

    // replace the complete call to preserve receiver grouping
    let source = module.source(receiver)?;
    let patch = Patch::replace(extent, source);
    let suggestion = lint.fix("iterate over the iterable directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove an explicit Array iterator call from for-of.
    #[test]
    fn test_removes_array_iterator_call() {
        let session = TestSession::dir(
            &NO_EXPLICIT_ITERATOR_IN_FOR_OF,
            r#"
function sum(values: int32[]): int32 {
    let total: int32 = 0;
    for (const value of values.iterator()) {
        total += value;
    }
    return total;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-explicit-iterator-in-for-of]: for-of calls iterator explicitly
 ──▶ main.tspp:3:25
  │
1 │ function sum(values: int32[]): int32 {
2 │     let total: int32 = 0;
3 │     for (const value of values.iterator()) {
  │                         ^^^^^^^^^^^^^^^^^
4 │         total += value;
5 │     }
  │

 = fix: iterate over the iterable directly
--- a/main.tspp
+++ b/main.tspp

    2│     let total: int32 = 0;
-   3│     for (const value of values.iterator()) {
+   3│     for (const value of values) {
    4│         total += value;
"#,
        );
        session.assert_fixes(
            r#"
function sum(values: int32[]): int32 {
    let total: int32 = 0;
    for (const value of values) {
        total += value;
    }
    return total;
}
"#,
        );
    }

    /// Remove an explicit Set iterator call from for-of.
    #[test]
    fn test_removes_set_iterator_call() {
        let session = TestSession::dir(
            &NO_EXPLICIT_ITERATOR_IN_FOR_OF,
            r#"
function visit(values: Set<int32>): void {
    for (const value of values.iterator()) {
        value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function visit(values: Set<int32>): void {
    for (const value of values) {
        value;
    }
}
"#,
        );
    }

    /// Preserve a comment inside the removed call.
    #[test]
    fn test_reports_commented_iterator_without_fix() {
        let session = TestSession::dir(
            &NO_EXPLICIT_ITERATOR_IN_FOR_OF,
            r#"
function visit(values: int32[]): void {
    for (const value of values.iterator(/* retain */)) {
        // intentionally empty
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-explicit-iterator-in-for-of]: for-of calls iterator explicitly
 ──▶ main.tspp:2:25
  │
1 │ function visit(values: int32[]): void {
2 │     for (const value of values.iterator(/* retain */)) {
  │                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         // intentionally empty
4 │     }
  │
"#,
        );
    }

    /// Accept iterator calls outside for-of.
    #[test]
    fn test_accepts_iterator_value() {
        let session = TestSession::dir(
            &NO_EXPLICIT_ITERATOR_IN_FOR_OF,
            r#"
import { Iterator } from "tspp:iter";

function iterator(values: int32[]): Iterator<int32> {
    return values.iterator();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined iterator method.
    #[test]
    fn test_accepts_user_iterator_method() {
        let session = TestSession::dir(
            &NO_EXPLICIT_ITERATOR_IN_FOR_OF,
            r#"
class Values {
    iterator(): int32[] {
        return [];
    }
}

function visit(values: Values): void {
    for (const value of values.iterator()) {
        // intentionally empty
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
