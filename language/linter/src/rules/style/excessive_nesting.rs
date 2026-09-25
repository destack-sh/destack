use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const MAX_NESTING_DEPTH: usize = 4;

declare_lint! {
    /// Disallow control flow nested beyond four levels.
    pub EXCESSIVE_NESTING {
        id: "excessive-nesting",
        summary: "Disallow control flow nested beyond four levels",
        explanation: r#"
Deeply nested control flow makes every operation depend on a long chain of surrounding branches.
Instead, you SHOULD use guards, early exits, or an extracted function once nesting exceeds four levels.
"#,
        example: {
            reported: r#"
function acceptsMail(
    isActive: boolean,
    hasEmail: boolean,
    isSubscribed: boolean,
    isVerified: boolean,
    allowsMail: boolean,
): boolean {
    if (isActive) {
        if (hasEmail) {
            if (isSubscribed) {
                if (isVerified) {
                    if (allowsMail) {
                        return true;
                    }
                }
            }
        }
    }
    return false;
}
"#,
            accepted: r#"
function acceptsMail(
    isActive: boolean,
    hasEmail: boolean,
    isSubscribed: boolean,
    isVerified: boolean,
    allowsMail: boolean,
): boolean {
    if (!isActive || !hasEmail || !isSubscribed || !isVerified || !allowsMail) {
        return false;
    }
    return true;
}
"#,
        },
        provenance: [Clippy("excessive_nesting")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report control expressions nested beyond the maximum depth.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored control expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !introduces_nesting(node) {
            continue;
        }
        let depth = nesting_depth(module, expression);
        if depth <= MAX_NESTING_DEPTH {
            continue;
        }

        let span = module.main_span(expression.into_any())?;
        output.report(lint.diagnostic(format!("control flow is nested {depth} levels deep"), span));
    }

    Ok(output)
}

/// Return the authored control-flow depth of one expression.
fn nesting_depth(module: &DirModule<'_>, expression: dir::LocalNodeId<dir::Expression>) -> usize {
    let view = module.view();
    let mut depth = 1;
    let mut child = expression.into_any();
    let mut parent = view.get_parent_any(child);

    // climb through the current evaluation context
    while let Some(node) = parent {
        // stop at callable and static initialization boundaries
        let is_static_or_const_block = node.try_into_typed::<dir::Member>().is_ok_and(|member| {
            matches!(
                view.get(member),
                dir::Member::StaticBlock { .. } | dir::Member::ConstBlock { .. }
            )
        });
        if module.callable_body(node).is_some() || is_static_or_const_block {
            break;
        }

        // count control ancestors except the links within an else-if chain
        if let Ok(expression) = node.try_into_typed::<dir::Expression>() {
            let value = view.get(expression);
            let is_else_if = matches!(
                value,
                dir::Expression::If {
                    form: dir::IfForm::If,
                    else_expression: Some(branch),
                    ..
                } if branch.into_any() == child
            );
            if introduces_nesting(value) && !is_else_if {
                depth += 1;
            }
        }

        // advance through the authored parent chain
        child = node;
        parent = view.get_parent_any(node);
    }

    depth
}

/// Return whether one expression introduces statement control-flow nesting.
fn introduces_nesting(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::If {
            form: dir::IfForm::If,
            ..
        } | dir::Expression::While { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::Try { .. }
            | dir::Expression::Match { .. }
            | dir::Expression::Switch { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Highlight the control keyword that exceeds the maximum depth.
    #[test]
    fn test_reports_fifth_nesting_level() {
        let session = TestSession::dir(
            &EXCESSIVE_NESTING,
            r#"
function acceptsMail(
    isActive: boolean,
    hasEmail: boolean,
    isSubscribed: boolean,
    isVerified: boolean,
    allowsMail: boolean,
): boolean {
    if (isActive) {
        if (hasEmail) {
            if (isSubscribed) {
                if (isVerified) {
                    if (allowsMail) {
                        return true;
                    }
                }
            }
        }
    }
    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[excessive-nesting]: control flow is nested 5 levels deep
  ──▶ main.tspp:12:21
   │
10 │             if (isSubscribed) {
11 │                 if (isVerified) {
12 │                     if (allowsMail) {
   │                     ^^
13 │                         return true;
14 │                     }
   │
"#,
        );
    }

    /// Accept four nested control-flow levels.
    #[test]
    fn test_accepts_maximum_depth() {
        let session = TestSession::dir(
            &EXCESSIVE_NESTING,
            r#"
function visit(
    isReady: boolean,
    hasWork: boolean,
    canVisit: boolean,
    shouldContinue: boolean,
): void {
    if (isReady) {
        while (hasWork) {
            if (canVisit) {
                while (shouldContinue) {
                    break;
                }
            }
            break;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep an else-if chain at one nesting level.
    #[test]
    fn test_accepts_else_if_chain() {
        let session = TestSession::dir(
            &EXCESSIVE_NESTING,
            r#"
function classify(value: int32): int32 {
    if (value < 0) {
        return -1;
    } else if (value == 0) {
        return 0;
    } else if (value == 1) {
        return 1;
    } else if (value == 2) {
        return 2;
    } else {
        return 3;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Reset nesting inside a nested callable.
    #[test]
    fn test_accepts_nested_callable_at_maximum_depth() {
        let session = TestSession::dir(
            &EXCESSIVE_NESTING,
            r#"
declare function visit(): void;

function traverse(
    hasRoot: boolean,
    hasBranch: boolean,
    hasNode: boolean,
    hasLeaf: boolean,
    shouldVisit: boolean,
): void {
    if (hasRoot) {
        while (hasBranch) {
            if (hasNode) {
                while (hasLeaf) {
                    const nested = () => {
                        if (shouldVisit) {
                            visit();
                        }
                    };
                    nested();
                    break;
                }
            }
            break;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Reset nesting inside a static initialization block.
    #[test]
    fn test_accepts_deep_nesting_inside_a_static_block() {
        let session = TestSession::dir(
            &EXCESSIVE_NESTING,
            r#"
const hasRoot: boolean = true;
const hasBranch: boolean = true;
const hasNode: boolean = true;
const hasLeaf: boolean = true;

class Registry {
    static {
        if (hasRoot) {
            while (hasBranch) {
                if (hasNode) {
                    while (hasLeaf) {
                        const value = 1;
                        break;
                    }
                }
                break;
            }
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
