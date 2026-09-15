use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow match arms with identical bodies.
    pub NO_DUPLICATE_MATCH_ARMS {
        id: "no-duplicate-match-arms",
        summary: "Disallow match arms with identical bodies",
        explanation: r#"
Identical match arm bodies often indicate an editing mistake and repeat one result.
Instead, you SHOULD correct an unintended body or combine the patterns when their guards match.
"#,
        example: {
            reported: r#"
function describe(value: int32): string {
    return match (value) {
        0 => "small"
        1 => "small"
        _ => "large"
    };
}
"#,
            accepted: r#"
function describe(value: int32): string {
    return match (value) {
        0 | 1 => "small"
        _ => "large"
    };
}
"#,
        },
        provenance: [Clippy("match_same_arms")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        indexes: [Code],
        check: DirModule(check),
    }
}

/// Report later match arms whose bodies repeat an earlier arm.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect the ordered arms of each match expression
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { arms, .. } = expression else {
            continue;
        };

        // compare each arm with preceding alternatives
        for (index, arm) in arms.iter().copied().enumerate() {
            let node = view.get(arm);
            let body = node.body();
            for earlier in arms[..index].iter().rev().copied() {
                let earlier_node = view.get(earlier);
                let earlier_body = earlier_node.body();
                let left = [earlier.into_global_any(module.id)];
                let right = [arm.into_global_any(module.id)];
                let Some(mut comparison) = module.dir.alpha_comparison(&left, &right)? else {
                    continue;
                };
                if !comparison.compare_conditions(earlier_node.guard(), node.guard())? {
                    continue;
                }
                let left = [earlier_body.into_global(module.id)];
                let right = [body.into_global(module.id)];
                if !comparison.compare_nodes(&left, &right)? {
                    continue;
                }

                // report the repeated body once against its nearest earlier match
                let span = module.source_extent(body)?;
                let earlier_span = module.source_extent(earlier_body)?;
                let diagnostic = lint
                    .diagnostic("match arm repeats an earlier body", span)
                    .primary("repeated body")
                    .label(earlier_span, "earlier body");
                output.report(diagnostic);
                break;
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report blocks that differ only in their arm binding names.
    #[test]
    fn test_reports_renamed_bindings() {
        let session = TestSession::dir(
            &NO_DUPLICATE_MATCH_ARMS,
            r#"
function describe(value: (int32, int32)): string {
    return match (value) {
        (left, 0) if (left > 0) => {
            const incremented = left + 1;
            return `value: ${incremented}`;
        }
        (right, 1) if (right > 0) => {
            const added = right + 1;
            return `value: ${added}`;
        }
        _ => "missing"
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-match-arms]: match arm repeats an earlier body
  ──▶ main.ds:7:38
   │
 1 │ function describe(value: (int32, int32)): string {
 2 │     return match (value) {
 3 │         (left, 0) if (left > 0) => {
   │                                    - earlier body
 4 │             const incremented = left + 1;
   │             -----------------------------
 5 │             return `value: ${incremented}`;
   │             -------------------------------
 6 │         }
   │         -
 7 │         (right, 1) if (right > 0) => {
   │                                      ^ repeated body
 8 │             const added = right + 1;
   │             ^^^^^^^^^^^^^^^^^^^^^^^^
 9 │             return `value: ${added}`;
   │             ^^^^^^^^^^^^^^^^^^^^^^^^^
10 │         }
   │         ^
11 │         _ => "missing"
12 │     };
   │
"#,
        );
    }

    /// Accept equal source text that resolves to different free bindings.
    #[test]
    fn test_accepts_different_resolutions() {
        let session = TestSession::dir(
            &NO_DUPLICATE_MATCH_ARMS,
            r#"
function choose(value: boolean, left: int32, right: int32): int32 {
    return match (value) {
        true => left + 1
        false => right + 1
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept equal bodies selected under different guards.
    #[test]
    fn test_accepts_different_guards() {
        let session = TestSession::dir(
            &NO_DUPLICATE_MATCH_ARMS,
            r#"
function classify(value: int32): int32 {
    return match (value) {
        current if (current > 0) => 1
        current if (current < 0) => 1
        _ => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report identical effectful bodies because comparison does not move or duplicate them.
    #[test]
    fn test_reports_effectful_bodies() {
        let session = TestSession::dir(
            &NO_DUPLICATE_MATCH_ARMS,
            r#"
declare function record(value: int32): int32;
function choose(value: boolean): int32 {
    return match (value) {
        true => record(1)
        false => record(1)
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-match-arms]: match arm repeats an earlier body
 ──▶ main.ds:5:18
  │
2 │ function choose(value: boolean): int32 {
3 │     return match (value) {
4 │         true => record(1)
  │                 --------- earlier body
5 │         false => record(1)
  │                  ^^^^^^^^^ repeated body
6 │     };
7 │ }
  │
"#,
        );
    }

    /// Accept bodies whose selected operations differ despite similar structure.
    #[test]
    fn test_accepts_different_operations() {
        let session = TestSession::dir(
            &NO_DUPLICATE_MATCH_ARMS,
            r#"
function choose(value: boolean, text: string, values: int32[]): isize {
    return match (value) {
        true => text.length
        false => values.length
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
