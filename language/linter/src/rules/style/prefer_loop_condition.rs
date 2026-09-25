use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, NodeSpanRegion, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer loop conditions over leading conditional breaks.
    pub PREFER_LOOP_CONDITION {
        id: "prefer-loop-condition",
        summary: "Prefer loop conditions over leading conditional breaks",
        explanation: r#"
A leading conditional `break` makes readers inspect the loop body to discover its primary continuation condition.
Instead, you SHOULD place the complemented condition in a `while` header when the test occurs before every iteration body.
"#,
        example: {
            reported: r#"
declare function isDone(): boolean;
declare function work(): void;

loop {
    if (isDone()) {
        break;
    }
    work();
}
"#,
            accepted: r#"
declare function isDone(): boolean;
declare function work(): void;

while (!isDone()) {
    work();
}
"#,
        },
        provenance: [Unicorn("prefer-while-loop-condition")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report unconditional loops beginning with their exit condition.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect unconditional loops with a leading conditional
    for (iteration, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Loop { body, .. } = node else {
            continue;
        };
        let Some(conditional) = view.get(*body).first_expression() else {
            continue;
        };
        let dir::Expression::If {
            condition,
            then_expression,
            else_expression: None,
            ..
        } = view.get(conditional)
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        if !matches!(view.get(*then_expression), dir::Expression::Block(_)) {
            continue;
        }
        if module.plain_break_target(*then_expression)? != Some(iteration)
            || module.has_valued_break(iteration)?
        {
            continue;
        }

        // replace the loop header and remove the leading exit branch
        let span = module.source_extent(conditional.into_any())?;
        let mut diagnostic = lint.diagnostic("loop begins with its continuation condition", span);
        if let Some(suggestion) =
            suggestion(module, lint, iteration, *body, conditional, condition)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build a while loop from one leading conditional break.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    iteration: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Block>,
    conditional: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let conditional_extent = module.source_extent(conditional.into_any())?;
    let condition_extent = module.source_extent(condition.into_any())?;
    let body_span = module.span(body.into_any())?;
    let removed = Span::new(body_span.file, body_span.start, conditional_extent.end);
    if module.has_unretained_comment(removed, &[condition_extent])? {
        return Ok(None);
    }

    // complement the exit condition without retaining a redundant negation
    let condition = match module.builtin_unary(condition)? {
        Some((dir::UnaryOperator::Not, operand)) => {
            module.expression_source(operand.source.local_id, dir::OperatorPrecedence::Lowest)?
        }
        _ => {
            let condition = module.expression_source(condition, dir::OperatorPrecedence::Prefix)?;

            format!("!{condition}").into()
        }
    };

    // edit the loop keyword and remove the leading conditional statement
    let mut patch = FilePatch::new(conditional_extent.file);
    patch.replace(
        module.source_region(iteration.into_any(), NodeSpanRegion::Keyword)?,
        format!("while ({condition})"),
    );
    let conditional = module.statement_span(conditional)?;
    patch.delete(module.line_removal_span(conditional)?);
    patch.sort();
    let suggestion = lint.suggestion("move the condition into the loop header", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove an existing negation when forming the continuation condition.
    #[test]
    fn test_replaces_negated_break_condition() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            r#"
declare function isReady(): boolean;
declare function work(): void;

loop {
    if (!isReady()) {
        break;
    }
    work();
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function isReady(): boolean;
declare function work(): void;

while (isReady()) {
    work();
}
"#,
        );
    }

    /// Preserve a label whose name contains the loop keyword.
    #[test]
    fn test_preserves_loop_keyword_in_label() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            r#"
declare function isDone(): boolean;
declare function work(): void;

looping: loop {
    if (isDone()) {
        break looping;
    }
    work();
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function isDone(): boolean;
declare function work(): void;

looping: while (!isDone()) {
    work();
}
"#,
        );
    }

    /// Accept an exit condition evaluated after other loop work.
    #[test]
    fn test_accepts_late_break() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            r#"
declare function isDone(): boolean;
declare function work(): void;

loop {
    work();
    if (isDone()) {
        break;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a value-producing loop that cannot become a while loop.
    #[test]
    fn test_accepts_loop_with_valued_break() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            r#"
declare function isDone(): boolean;

loop {
    if (isDone()) {
        break 0;
    }
    break 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a leading break that exits an outer loop.
    #[test]
    fn test_accepts_outer_break() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            r#"
declare function isDone(): boolean;
declare function work(): void;

outer: loop {
    loop {
        if (isDone()) {
            break outer;
        }
        work();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without moving comments that explain the exit.
    #[test]
    fn test_retains_break_comment() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            r#"
declare function isDone(): boolean;
declare function work(): void;

loop {
    // stop before starting another unit
    if (isDone()) {
        break;
    }
    work();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-loop-condition]: loop begins with its continuation condition
  ──▶ main.tspp:6:5
   │
 4 │ loop {
 5 │     // stop before starting another unit
 6 │     if (isDone()) {
   │     ^^^^^^^^^^^^^^^
 7 │         break;
   │         ^^^^^^
 8 │     }
   │     ^
 9 │     work();
10 │ }
   │
"#,
        );
    }
}
