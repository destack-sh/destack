use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer else in compact terminal value decisions.
    pub PREFER_ELSE {
        id: "prefer-else",
        summary: "Prefer else in compact terminal value decisions",
        explanation: r#"
An adjacent sequence of terminal value branches forms one decision.
You SHOULD write that decision as an `if`/`else if`/`else` chain.
"#,
        example: {
            reported: r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    }
    return "nonnegative";
}
"#,
            accepted: r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    } else {
        return "nonnegative";
    }
}
"#,
        },
        provenance: [],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report compact terminal value branches written outside one decision chain.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect the terminal value decision in each block
    for (block_id, block) in view.iter_nodes::<dir::Block>() {
        // select one implicit value or explicit value return as the final alternative
        let (alternative, preceding, is_implicit) = if let Some(alternative) = block.tail_expression
        {
            let Some(body) = view
                .get_parent_for(block_id)
                .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
            else {
                continue;
            };
            if module.enclosing_callable_body(block_id.into_any()) != Some(body) {
                continue;
            }

            (alternative, block.leading_expressions.as_slice(), true)
        } else {
            let Some((alternative, preceding)) = block.leading_expressions.split_last() else {
                continue;
            };
            if module.sole_return_value(*alternative).is_none() {
                continue;
            }

            (*alternative, preceding, false)
        };

        // select the adjacent compact branches before the final alternative
        let start = preceding
            .iter()
            .rposition(|expression| !is_compact_value_if(module, *expression))
            .map_or(0, |index| index + 1);
        if start == preceding.len()
            || (is_implicit && module.is_diverging(alternative.into_any())?)
        {
            continue;
        }

        // report each branch in source order
        for expression in &preceding[start..] {
            let span = module.main_span(expression.into_any())?;
            output.report(
                lint.diagnostic("terminal value alternative is separate from its if", span),
            );
        }
    }

    Ok(output)
}

/// Return whether one if branch consists only of a value return.
fn is_compact_value_if(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let dir::Expression::If {
        form: dir::IfForm::If,
        then_expression,
        else_expression: None,
        ..
    } = module.view().get(expression)
    else {
        return false;
    };

    module.sole_return_value(*then_expression).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a guard that calls a diverging function.
    #[test]
    fn test_accepts_call_diverging_guard() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function classifyNonzero(value: int32): never;

function classify(value: int32): string {
    if (value !== 0) {
        classifyNonzero(value);
    }
    return "zero";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report every branch in one adjacent terminal decision chain.
    #[test]
    fn test_reports_terminal_chain() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
function classify(value: int32): string {
    if (value < 0) {
        return "negative";
    }
    if (value > 0) {
        return "positive";
    }
    return "zero";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-else]: terminal value alternative is separate from its if
 ──▶ main.tspp:2:5
  │
1 │ function classify(value: int32): string {
2 │     if (value < 0) {
  │     ^^
3 │         return "negative";
4 │     }
  │

warning[prefer-else]: terminal value alternative is separate from its if
 ──▶ main.tspp:5:5
  │
3 │         return "negative";
4 │     }
5 │     if (value > 0) {
  │     ^^
6 │         return "positive";
7 │     }
  │
"#,
        );
    }

    /// Report a compact return before one implicit value alternative.
    #[test]
    fn test_reports_implicit_value_alternative() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
function checked(result: int32, overflow: boolean): int32 | undefined {
    if (overflow) {
        return undefined;
    }
    result
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-else]: terminal value alternative is separate from its if
 ──▶ main.tspp:2:5
  │
1 │ function checked(result: int32, overflow: boolean): int32 | undefined {
2 │     if (overflow) {
  │     ^^
3 │         return undefined;
4 │     }
  │
"#,
        );
    }

    /// Accept a value return followed by computed fallback work.
    #[test]
    fn test_accepts_computed_fallback() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function adjust(value: int32): int32;

function checked(value: int32, isSpecial: boolean): int32 {
    if (isSpecial) {
        return value;
    }
    const adjusted = adjust(value);

    adjusted
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a terminal branch that performs work before returning a value.
    #[test]
    fn test_accepts_worked_branch() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function observe(value: int32): void;

function checked(value: int32, isSpecial: boolean): int32 {
    if (isSpecial) {
        observe(value);

        return value;
    }
    return 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a return guard before a nested block value.
    #[test]
    fn test_accepts_nested_block_value() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
function checked(value: int32, isSpecial: boolean): int32 {
    const adjusted = do {
        if (isSpecial) {
            return value;
        }
        value + 1
    };

    adjusted
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a value return before a diverging fallback.
    #[test]
    fn test_accepts_diverging_fallback() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function fail(): never;

function checked(isValid: boolean): int32 {
    if (isValid) {
        return 1;
    }
    fail()
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a valueless return guard before a continuing function path.
    #[test]
    fn test_accepts_valueless_return_guard() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function work(): void;

function run(isInvalid: boolean): void {
    if (isInvalid) {
        return;
    }
    work();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a continue guard before the remaining iteration path.
    #[test]
    fn test_accepts_continue_guard() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
declare function work(value: int32): void;

function visit(values: int32[]): void {
    for (const value of values) {
        if (value < 0) {
            continue;
        }
        work(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an implicit undefined result in a default interface method.
    #[test]
    fn test_accepts_default_interface_expression() {
        let session = TestSession::dir(
            &PREFER_ELSE,
            r#"
newtype interface Source {
    source(): int32 | undefined {
        undefined
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
