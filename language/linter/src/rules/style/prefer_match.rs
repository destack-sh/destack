use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer match over switch statements and repeated equality chains.
    pub PREFER_MATCH {
        id: "prefer-match",
        summary: "Prefer match over switch statements and repeated equality chains",
        explanation: r#"
Repeated equality branches and switch cases encode selection as independent statements.
Instead, you SHOULD use `match` to bind the selected value and its cases in one expression.
"#,
        example: {
            reported: r#"
function describe(value: int32): string {
    if (value === 0) {
        return "zero";
    } else if (value === 1) {
        return "one";
    } else if (value === 2) {
        return "two";
    }

    return "many";
}
"#,
            accepted: r#"
function describe(value: int32): string {
    return match (value) {
        0 => "zero"
        1 => "one"
        2 => "two"
        _ => "many"
    };
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

/// Report switch statements and repeated equality chains.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored selection expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let message = match node {
            dir::Expression::Switch { .. } => Some("switch statement can use match"),
            dir::Expression::If {
                form: dir::IfForm::If,
                ..
            } if !module.is_else_if(expression) && is_repeated_equality(expression, module)? => {
                Some("repeated equality chain can use match")
            }
            _ => None,
        };
        let Some(message) = message else {
            continue;
        };

        let span = module.main_span(expression.into_any())?;
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

/// Return whether one if chain compares the same value with at least three constants.
fn is_repeated_equality(
    expression: dir::LocalNodeId<dir::Expression>,
    module: &DirModule<'_>,
) -> Result<bool, ProviderError> {
    let Some(chain) = module.if_chain(expression) else {
        return Ok(false);
    };
    if chain.branches.len() < 3 {
        return Ok(false);
    }
    let Some(first) = equality_subject(chain.branches[0].condition, module)? else {
        return Ok(false);
    };

    // require every branch to compare the same computation with one scalar constant
    for branch in &chain.branches[1..] {
        let Some(subject) = equality_subject(branch.condition, module)? else {
            return Ok(false);
        };
        if !module.is_same_computation(first, subject)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Return the nonconstant subject of one builtin equality condition.
fn equality_subject(
    condition: &dir::Condition,
    module: &DirModule<'_>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(expression) = condition.as_expression() else {
        return Ok(None);
    };
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    if operator != dir::BinaryOperator::EqualStrict {
        return Ok(None);
    }
    let left = left.source.local_id;
    let right = right.source.local_id;
    let left_constant = module.scalar_constant(left)?.is_some();
    let right_constant = module.scalar_constant(right)?.is_some();

    Ok(match (left_constant, right_constant) {
        (false, true) => Some(left),
        (true, false) => Some(right),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report repeated equality selection over one value.
    #[test]
    fn test_reports_equality_chain() {
        let session = TestSession::dir(
            &PREFER_MATCH,
            r#"
function describe(value: int32): string {
    if (value === 0) {
        return "zero";
    } else if (value === 1) {
        return "one";
    } else if (value === 2) {
        return "two";
    }

    return "many";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-match]: repeated equality chain can use match
 ──▶ main.tspp:2:5
  │
1 │ function describe(value: int32): string {
2 │     if (value === 0) {
  │     ^^
3 │         return "zero";
4 │     } else if (value === 1) {
  │
"#,
        );
    }

    /// Accept a short conditional chain.
    #[test]
    fn test_accepts_short_chain() {
        let session = TestSession::dir(
            &PREFER_MATCH,
            r#"
function sign(value: int32): string {
    if (value < 0) {
        return "negative";
    } else if (value > 0) {
        return "positive";
    }

    return "zero";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a chain whose equality protocol need not match pattern selection.
    #[test]
    fn test_accepts_value_equality_chain() {
        let session = TestSession::dir(
            &PREFER_MATCH,
            r#"
function describe(value: int32): string {
    if (value == 0) {
        return "zero";
    } else if (value == 1) {
        return "one";
    } else if (value == 2) {
        return "two";
    }

    return "many";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report legacy switch selection.
    #[test]
    fn test_reports_switch() {
        let session = TestSession::dir(
            &PREFER_MATCH,
            r#"
function classify(value: int32): string {
    switch (value) {
        case 0:
            return "zero";
        default:
            return "other";
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-match]: switch statement can use match
 ──▶ main.tspp:2:5
  │
1 │ function classify(value: int32): string {
2 │     switch (value) {
  │     ^^^^^^
3 │         case 0:
4 │             return "zero";
  │
"#,
        );
    }
}
