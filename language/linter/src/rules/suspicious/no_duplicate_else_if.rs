use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow else-if conditions made unreachable by earlier conditions.
    pub NO_DUPLICATE_ELSE_IF {
        id: "no-duplicate-else-if",
        summary: "Disallow else-if conditions made unreachable by earlier conditions",
        explanation: r#"
An else-if condition that implies an earlier condition can never select its body.
Instead, you SHOULD remove the unreachable branch or correct the conditions so each branch can be selected.
"#,
        example: {
            reported: r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else if (value > 0 && value < 10) {
        return "small";
    }
    return "other";
}
"#,
            accepted: r#"
function classify(value: int32): string {
    if (value > 0 && value < 10) {
        return "small";
    } else if (value > 0) {
        return "positive";
    }
    return "other";
}
"#,
        },
        provenance: [Eslint("no-dupe-else-if")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report else-if conditions made unreachable by earlier conditions.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect only heads of authored if chains
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        if module.is_else_if(expression) {
            continue;
        }
        let Some(chain) = module.if_chain(expression) else {
            continue;
        };

        // compare each later expression condition with all earlier branches
        let mut previous = Vec::new();
        for branch in chain.branches {
            let Some(condition) = branch.condition.as_expression() else {
                continue;
            };
            if is_branch_unreachable(module, &previous, condition)? {
                let span = module.source_extent(condition.into_any())?;
                let diagnostic = lint.diagnostic(
                    "else-if condition cannot be reached after earlier branches",
                    span,
                );
                output.report(diagnostic);
            }
            previous.push(condition);
        }
    }

    Ok(output)
}

/// Return whether earlier branches make one later condition unreachable.
fn is_branch_unreachable(
    module: &DirModule<'_>,
    earlier: &[dir::LocalNodeId<dir::Expression>],
    candidate: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let mut conditions = vec![candidate];
    let conjunction = module.short_circuit_operands(candidate, dir::BinaryOperator::And)?;
    if conjunction.len() > 1 {
        conditions.extend(conjunction);
    }

    // one rejected conjunct makes the complete candidate unreachable
    for condition in conditions {
        let alternatives = module.short_circuit_operands(condition, dir::BinaryOperator::Or)?;

        // require every alternative to imply at least one earlier branch
        let mut is_rejected = true;
        for alternative in alternatives {
            let mut is_implied = false;
            for earlier in earlier.iter().copied() {
                if module.boolean_implies(alternative, earlier)? {
                    is_implied = true;
                    break;
                }
            }
            if !is_implied {
                is_rejected = false;
                break;
            }
        }
        if is_rejected {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an exact duplicate else-if condition.
    #[test]
    fn test_reports_duplicate_condition() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else if (value > 0) {
        return "also positive";
    }
    return "other";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:4:16
  │
2 │     if (value > 0) {
3 │         return "positive";
4 │     } else if (value > 0) {
  │                ^^^^^^^^^
5 │         return "also positive";
6 │     }
  │
"#,
        );
    }

    /// Report a duplicate conjunction and a conjunction containing it.
    #[test]
    fn test_reports_duplicate_conjunctions() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(value: int32, isReady: boolean): string {
    if (value > 0 && isReady) {
        return "first";
    } else if (value > 0 && isReady) {
        return "duplicate";
    } else if ((value > 0 && isReady) && value < 10) {
        return "covered";
    }
    return "other";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:4:16
  │
2 │     if (value > 0 && isReady) {
3 │         return "first";
4 │     } else if (value > 0 && isReady) {
  │                ^^^^^^^^^^^^^^^^^^^^
5 │         return "duplicate";
6 │     } else if ((value > 0 && isReady) && value < 10) {
  │

warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:6:16
  │
4 │     } else if (value > 0 && isReady) {
5 │         return "duplicate";
6 │     } else if ((value > 0 && isReady) && value < 10) {
  │                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │         return "covered";
8 │     }
  │
"#,
        );
    }

    /// Report a conjunction implied by an earlier condition.
    #[test]
    fn test_reports_implied_conjunction() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(value: int32): string {
    if (value > 0) {
        return "positive";
    } else if (value > 0 && value < 10) {
        return "small";
    }
    return "other";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:4:16
  │
2 │     if (value > 0) {
3 │         return "positive";
4 │     } else if (value > 0 && value < 10) {
  │                ^^^^^^^^^^^^^^^^^^^^^^^
5 │         return "small";
6 │     }
  │
"#,
        );
    }

    /// Report a condition implied by one earlier disjunction alternative.
    #[test]
    fn test_reports_implied_alternative() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(value: int32): string {
    if (value < 0 || value > 10) {
        return "outside";
    } else if (value > 10) {
        return "large";
    }
    return "inside";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:4:16
  │
2 │     if (value < 0 || value > 10) {
3 │         return "outside";
4 │     } else if (value > 10) {
  │                ^^^^^^^^^^
5 │         return "large";
6 │     }
  │
"#,
        );
    }

    /// Report a disjunction whose alternatives imply earlier branches.
    #[test]
    fn test_reports_alternatives_implying_prior_branches() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(isText: boolean, isNumber: boolean): string {
    if (isText) {
        return "text";
    } else if (isNumber) {
        return "number";
    } else if (isText || isNumber) {
        return "scalar";
    }
    return "other";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:6:16
  │
4 │     } else if (isNumber) {
5 │         return "number";
6 │     } else if (isText || isNumber) {
  │                ^^^^^^^^^^^^^^^^^^
7 │         return "scalar";
8 │     }
  │
"#,
        );
    }

    /// Report a conjunct whose alternatives are rejected by earlier branches.
    #[test]
    fn test_reports_collectively_rejected_conjunct() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(a: boolean, b: boolean, c: boolean, d: boolean, e: boolean): string {
    if (a) {
        return "a";
    } else if (b && c) {
        return "bc";
    } else if (d && ((c && e && b) || a)) {
        return "unreachable";
    }
    return "other";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-else-if]: else-if condition cannot be reached after earlier branches
 ──▶ main.tspp:6:16
  │
4 │     } else if (b && c) {
5 │         return "bc";
6 │     } else if (d && ((c && e && b) || a)) {
  │                ^^^^^^^^^^^^^^^^^^^^^^^^^
7 │         return "unreachable";
8 │     }
  │
"#,
        );
    }

    /// Accept conditions ordered from specific to general.
    #[test]
    fn test_accepts_specific_before_general() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
function classify(value: int32): string {
    if (value > 0 && value < 10) {
        return "small";
    } else if (value > 0) {
        return "positive";
    }
    return "other";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept repeated effectful calls because their results can differ.
    #[test]
    fn test_accepts_repeated_calls() {
        let session = TestSession::dir(
            &NO_DUPLICATE_ELSE_IF,
            r#"
declare function ready(): boolean;

function wait(): string {
    if (ready()) {
        return "first";
    } else if (ready()) {
        return "second";
    }
    return "waiting";
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
