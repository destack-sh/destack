use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow block expressions in conditions and scrutinees.
    pub NO_BLOCK_IN_CONDITION {
        id: "no-block-in-condition",
        summary: "Disallow block expressions in conditions and scrutinees",
        explanation: r#"
A block expression in a condition or scrutinee interleaves statement execution with control selection.
Instead, you SHOULD use a single expression directly or compute a multi-step value before the construct.
"#,
        example: {
            reported: r#"
function choose(isReady: boolean): string {
    return do { isReady } ? "ready" : "waiting";
}
"#,
            accepted: r#"
function choose(isReady: boolean): string {
    return isReady ? "ready" : "waiting";
}
"#,
        },
        provenance: [Clippy("blocks_in_conditions")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report blocks used directly as control-flow inputs.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect direct conditions and selected values
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        match node {
            dir::Expression::If {
                form, condition, ..
            } => report_condition_blocks(
                module,
                lint,
                condition,
                *form == dir::IfForm::Ternary,
                &mut output,
            )?,
            dir::Expression::While { condition, .. } => {
                report_condition_blocks(module, lint, condition, false, &mut output)?;
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => report_block_input(
                module,
                lint,
                *condition,
                dir::OperatorPrecedence::Lowest,
                &mut output,
            )?,
            dir::Expression::Match { value, .. } | dir::Expression::Switch { value, .. } => {
                report_block_input(
                    module,
                    lint,
                    *value,
                    dir::OperatorPrecedence::Lowest,
                    &mut output,
                )?;
            }
            _ => {}
        }
    }

    // inspect match guards
    for (_, arm) in view.iter_nodes::<dir::MatchArm>() {
        let Some(guard) = arm.guard() else {
            continue;
        };

        report_condition_blocks(module, lint, guard, false, &mut output)?;
    }

    Ok(output)
}

/// Report blocks used directly as operands of one condition.
fn report_condition_blocks(
    module: &DirModule<'_>,
    lint: &Lint,
    condition: &dir::Condition,
    is_ternary: bool,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let minimum_precedence = if condition.operands.len() > 1 {
        dir::OperatorPrecedence::LogicalAnd
    } else if is_ternary {
        dir::OperatorPrecedence::TypeRelation
    } else {
        dir::OperatorPrecedence::Lowest
    };

    for input in condition.expressions() {
        report_block_input(module, lint, input, minimum_precedence, output)?;
    }

    Ok(())
}

/// Report one block used directly as a control-flow input.
fn report_block_input(
    module: &DirModule<'_>,
    lint: &Lint,
    input: dir::LocalNodeId<dir::Expression>,
    minimum_precedence: dir::OperatorPrecedence,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // require a block expression in the selected position
    let dir::Expression::Block(block) = view.get(input) else {
        return Ok(());
    };

    // replace a trivial block with its only expression
    let span = module.source_extent(input.into_any())?;
    let mut diagnostic = lint.diagnostic("control-flow input is a block expression", span);
    if let Some(value) = view.get(*block).only_expression()
        && let Some(suggestion) = suggestion(module, lint, span, value, minimum_precedence)?
    {
        diagnostic = diagnostic.suggestion(suggestion);
    }
    output.report(diagnostic);

    Ok(())
}

/// Replace one trivial block with its retained expression.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
    minimum_precedence: dir::OperatorPrecedence,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let value_span = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value_span])? {
        return Ok(None);
    }

    // retain authored grouping and meet the surrounding operator precedence
    let replacement = module.expression_source(value, minimum_precedence)?;

    // replace the complete block expression
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use the expression directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a block outside a control-flow input.
    #[test]
    fn test_accepts_regular_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function value(): int32 {
    return do { 1 };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a trivial block used as a while-loop condition.
    #[test]
    fn test_replaces_while_condition_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function wait(isReady: boolean): void {
    while (do { isReady }) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:12
  │
1 │ function wait(isReady: boolean): void {
2 │     while (do { isReady }) {}
  │            ^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function wait(isReady: boolean): void {
-   2│     while (do { isReady }) {}
+   2│     while (isReady) {}
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function wait(isReady: boolean): void {
    while (isReady) {}
}
"#,
        );
    }

    /// Replace a trivial block after a while-loop condition binding.
    #[test]
    fn test_replaces_while_condition_binding_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function wait(value: { ready: boolean | undefined } | null): void {
    while (let { ready } = value && do { ready ?? false }) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:37
  │
1 │ function wait(value: { ready: boolean | undefined } | null): void {
2 │     while (let { ready } = value && do { ready ?? false }) {}
  │                                     ^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function wait(value: { ready: boolean | undefined } | null): void {
-   2│     while (let { ready } = value && do { ready ?? false }) {}
+   2│     while (let { ready } = value && (ready ?? false)) {}
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function wait(value: { ready: boolean | undefined } | null): void {
    while (let { ready } = value && (ready ?? false)) {}
}
"#,
        );
    }

    /// Report a multi-step condition block without an automatic replacement.
    #[test]
    fn test_reports_nontrivial_condition_block_without_fix() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function choose(isReady: boolean): string {
    return do {
        isReady;
        isReady
    }
        ? "ready"
        : "waiting";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:12
  │
1 │ function choose(isReady: boolean): string {
2 │     return do {
  │            ^^^^
3 │         isReady;
  │         ^^^^^^^^
4 │         isReady
  │         ^^^^^^^
5 │     }
  │     ^
6 │         ? "ready"
7 │         : "waiting";
  │
"#,
        );
    }

    /// Replace a trivial block used as a match value.
    #[test]
    fn test_replaces_match_value_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function choose(value: int32): int32 {
    return match (do { value }) {
        selected => selected
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:19
  │
1 │ function choose(value: int32): int32 {
2 │     return match (do { value }) {
  │                   ^^^^^^^^^^^^
3 │         selected => selected
4 │     };
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function choose(value: int32): int32 {
-   2│     return match (do { value }) {
+   2│     return match (value) {
    3│         selected => selected
"#,
        );
        session.assert_fixes(
            r#"
function choose(value: int32): int32 {
    return match (value) {
        selected => selected
    };
}
"#,
        );
    }

    /// Replace a trivial block used as a switch value.
    #[test]
    fn test_replaces_switch_value_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function choose(value: int32): void {
    switch (do { value }) {
        case 0:
            return;
        default:
            return;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:13
  │
1 │ function choose(value: int32): void {
2 │     switch (do { value }) {
  │             ^^^^^^^^^^^^
3 │         case 0:
4 │             return;
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function choose(value: int32): void {
-   2│     switch (do { value }) {
+   2│     switch (value) {
    3│         case 0:
"#,
        );
        session.assert_fixes(
            r#"
function choose(value: int32): void {
    switch (value) {
        case 0:
            return;
        default:
            return;
    }
}
"#,
        );
    }

    /// Preserve a nested ternary as one condition operand.
    #[test]
    fn test_groups_nested_ternary_condition() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function choose(isReady: boolean, hasValue: boolean): string {
    return do { isReady ? hasValue : false } ? "ready" : "waiting";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:12
  │
1 │ function choose(isReady: boolean, hasValue: boolean): string {
2 │     return do { isReady ? hasValue : false } ? "ready" : "waiting";
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function choose(isReady: boolean, hasValue: boolean): string {
-   2│     return do { isReady ? hasValue : false } ? "ready" : "waiting";
+   2│     return (isReady ? hasValue : false) ? "ready" : "waiting";
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function choose(isReady: boolean, hasValue: boolean): string {
    return (isReady ? hasValue : false) ? "ready" : "waiting";
}
"#,
        );
    }

    /// Preserve authored grouping inside a trivial condition block.
    #[test]
    fn test_preserves_grouped_condition() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function choose(isReady: boolean, hasValue: boolean): string {
    return do { (isReady && hasValue) } ? "ready" : "waiting";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:12
  │
1 │ function choose(isReady: boolean, hasValue: boolean): string {
2 │     return do { (isReady && hasValue) } ? "ready" : "waiting";
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function choose(isReady: boolean, hasValue: boolean): string {
-   2│     return do { (isReady && hasValue) } ? "ready" : "waiting";
+   2│     return (isReady && hasValue) ? "ready" : "waiting";
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function choose(isReady: boolean, hasValue: boolean): string {
    return (isReady && hasValue) ? "ready" : "waiting";
}
"#,
        );
    }

    /// Replace a trivial block used as a traditional for-loop condition.
    #[test]
    fn test_replaces_for_condition_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function count(): void {
    for (let index = 0; do { index < 3 }; index++) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:2:25
  │
1 │ function count(): void {
2 │     for (let index = 0; do { index < 3 }; index++) {}
  │                         ^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    1│ function count(): void {
-   2│     for (let index = 0; do { index < 3 }; index++) {}
+   2│     for (let index = 0; index < 3; index++) {}
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function count(): void {
    for (let index = 0; index < 3; index++) {}
}
"#,
        );
    }

    /// Replace a trivial block used as a match guard.
    #[test]
    fn test_replaces_match_guard_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function positive(value: int32): int32 {
    return match (value) {
        value if (do { value > 0 }) => value
        _ => 0
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:3:19
  │
1 │ function positive(value: int32): int32 {
2 │     return match (value) {
3 │         value if (do { value > 0 }) => value
  │                   ^^^^^^^^^^^^^^^^
4 │         _ => 0
5 │     };
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    2│     return match (value) {
-   3│         value if (do { value > 0 }) => value
+   3│         value if (value > 0) => value
    4│         _ => 0
"#,
        );
        session.assert_fixes(
            r#"
function positive(value: int32): int32 {
    return match (value) {
        value if (value > 0) => value
        _ => 0
    };
}
"#,
        );
    }

    /// Replace a trivial block after a match-guard binding.
    #[test]
    fn test_replaces_match_binding_guard_block() {
        let session = TestSession::dir(
            &NO_BLOCK_IN_CONDITION,
            r#"
function positive(value: (int32, boolean) | null): int32 {
    return match (value) {
        pair if (let (number, ready) = pair && do { ready }) => number
        _ => 0
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-block-in-condition]: control-flow input is a block expression
 ──▶ main.tspp:3:48
  │
1 │ function positive(value: (int32, boolean) | null): int32 {
2 │     return match (value) {
3 │         pair if (let (number, ready) = pair && do { ready }) => number
  │                                                ^^^^^^^^^^^^
4 │         _ => 0
5 │     };
  │

 = fix: use the expression directly
--- a/main.tspp
+++ b/main.tspp

    2│     return match (value) {
-   3│         pair if (let (number, ready) = pair && do { ready }) => number
+   3│         pair if (let (number, ready) = pair && ready) => number
    4│         _ => 0
"#,
        );
        session.assert_fixes(
            r#"
function positive(value: (int32, boolean) | null): int32 {
    return match (value) {
        pair if (let (number, ready) = pair && ready) => number
        _ => 0
    };
}
"#,
        );
    }
}
