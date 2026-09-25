use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Extract identical branch prefixes or suffixes.
    pub BRANCHES_SHARING_CODE {
        id: "branches-sharing-code",
        summary: "Extract identical branch prefixes or suffixes",
        explanation: r#"
The same checked statements occur at the corresponding prefix or suffix of every branch.
Instead, you SHOULD extract the repetition while preserving condition evaluation, branch scope, and drop order.
"#,
        example: {
            reported: r#"
declare function record(value: string): void;
declare function flush(): void;
function finish(condition: boolean): void {
    if (condition) {
        record("left");
        flush();
    } else {
        record("right");
        flush();
    }
}
"#,
            accepted: r#"
declare function record(value: string): void;
declare function flush(): void;
function finish(condition: boolean): void {
    if (condition) {
        record("left");
    } else {
        record("right");
    }
    flush();
}
"#,
        },
        provenance: [Clippy("branches_sharing_code")],
        category: Style,
        level: Warning,
        fixable: None,
        indexes: [Code],
        check: DirModule(check),
    }
}

/// Report alpha-equivalent statements at matching branch edges.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect complete if chains with explicit branch blocks
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        if module.is_else_if(expression) {
            continue;
        }
        let Some(chain) = module.if_chain(expression) else {
            continue;
        };
        let Some(else_body) = chain.else_body else {
            continue;
        };
        let Some(mut branches) = chain
            .branches
            .iter()
            .map(|branch| module.block_expressions(branch.body))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let Some(else_body) = module.block_expressions(else_body) else {
            continue;
        };
        branches.push(else_body);
        let Some(shortest_len) = branches.iter().map(Vec::len).min() else {
            return Err(ProviderError::internal("complete if chain has no branches"));
        };

        // report the longest common prefix and then the nonoverlapping suffix
        let sequences = branches.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let prefix_len = module
            .dir
            .alpha_common_prefix_len(&sequences, shortest_len)?;
        if prefix_len > 0 {
            report_shared_range(
                module,
                lint,
                &branches,
                0..prefix_len,
                false,
                shortest_len,
                "branch prefix repeats across alternatives",
                &mut output,
            )?;
        }
        let suffix_len = module
            .dir
            .alpha_common_suffix_len(&sequences, shortest_len - prefix_len)?;
        if suffix_len > 0 {
            let suffix_start = shortest_len - suffix_len;
            report_shared_range(
                module,
                lint,
                &branches,
                suffix_start..shortest_len,
                true,
                shortest_len,
                "branch suffix repeats across alternatives",
                &mut output,
            )?;
        }
    }

    // inspect match values that directly finish an enclosing block
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { arms, .. } = node else {
            continue;
        };
        let Some(parent) = view.get_parent_for(expression) else {
            continue;
        };
        let Ok(parent) = parent.try_into_typed::<dir::Block>() else {
            continue;
        };
        if view.get(parent).value_expression() != Some(expression) || arms.len() < 2 {
            continue;
        }

        // collect one value tail from every explicit arm block
        let mut tails = Vec::with_capacity(arms.len());
        let mut has_preceding_work = false;
        let mut is_complete = true;
        for arm in arms {
            let dir::MatchArm::Block { body, .. } = view.get(*arm) else {
                is_complete = false;
                break;
            };
            let block = view.get(*body);
            let Some(tail) = block.value_expression() else {
                is_complete = false;
                break;
            };
            has_preceding_work |= !block.leading_expressions.is_empty();
            tails.push(tail);
        }
        if !is_complete || !has_preceding_work {
            continue;
        }

        // require corresponding tails under each arm's pattern bindings
        let first_tail = tails[0];
        let first_tail = [first_tail.into_global_any(module.id)];
        let mut spans = vec![module.source_extent(first_tail[0].local_id)?];
        for tail in &tails[1..] {
            let tail = [tail.into_global_any(module.id)];
            let Some(mut comparison) = module.dir.alpha_comparison(&first_tail, &tail)? else {
                spans.clear();
                break;
            };
            if !comparison.compare_nodes(&first_tail, &tail)? {
                spans.clear();
                break;
            }
            spans.push(module.source_extent(tail[0].local_id)?);
        }
        if spans.len() != tails.len() {
            continue;
        }

        // use the final arm as the primary repeated value
        let Some(repeated) = spans.pop() else {
            return Err(ProviderError::internal("shared match arm set is empty"));
        };
        let mut diagnostic = lint
            .diagnostic("match arm suffix repeats across alternatives", repeated)
            .primary("repeated value");
        for earlier in spans {
            diagnostic = diagnostic.label(earlier, "shared value");
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Report one corresponding statement range across every branch.
fn report_shared_range(
    module: &DirModule<'_>,
    lint: &Lint,
    branches: &[Vec<dir::GlobalNodeIdAny>],
    range: std::ops::Range<usize>,
    is_suffix: bool,
    shortest_len: usize,
    message: &'static str,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let mut spans = Vec::with_capacity(branches.len());

    // map the shortest-branch indices onto each branch's matching edge
    for branch in branches {
        let offset = if is_suffix {
            branch.len() - shortest_len
        } else {
            0
        };
        spans.push(expression_range(
            module,
            &branch[range.start + offset..range.end + offset],
        )?);
    }

    // use the final branch as the primary repeated range
    let Some(repeated) = spans.pop() else {
        return Err(ProviderError::internal("shared branch set is empty"));
    };

    // label every preceding occurrence in source order
    let mut diagnostic = lint
        .diagnostic(message, repeated)
        .primary("repeated statements");
    for earlier in spans {
        diagnostic = diagnostic.label(earlier, "shared statements");
    }
    output.report(diagnostic);

    Ok(())
}

/// Return the merged source extent of one nonempty expression sequence.
fn expression_range(
    module: &DirModule<'_>,
    expressions: &[dir::GlobalNodeIdAny],
) -> Result<Span, ProviderError> {
    let (Some(first), Some(last)) = (expressions.first(), expressions.last()) else {
        return Err(ProviderError::internal("shared branch range is empty"));
    };
    let first = module.source_extent(first.local_id)?;
    let last = module.source_extent(last.local_id)?;

    Ok(first.merge(last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an effectful suffix shared by both alternatives.
    #[test]
    fn test_reports_shared_suffix() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: string): void;
declare function flush(): void;
function finish(condition: boolean): void {
    if (condition) {
        record("left");
        record("again");
        flush();
    } else {
        record("right");
        flush();
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[branches-sharing-code]: branch suffix repeats across alternatives
  ──▶ main.tspp:10:9
   │
 5 │         record("left");
 6 │         record("again");
 7 │         flush();
   │         ------- shared statements
 8 │     } else {
 9 │         record("right");
10 │         flush();
   │         ^^^^^^^ repeated statements
11 │     }
12 │ }
   │
"#,
        );
    }

    /// Report a shared edge only when every else-if alternative contains it.
    #[test]
    fn test_reports_else_if_suffix() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: string): void;
declare function flush(): void;
function finish(value: int32): void {
    if (value == 0) {
        record("zero");
        flush();
    } else if (value == 1) {
        record("one");
        flush();
    } else {
        record("other");
        flush();
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[branches-sharing-code]: branch suffix repeats across alternatives
  ──▶ main.tspp:12:9
   │
 4 │     if (value == 0) {
 5 │         record("zero");
 6 │         flush();
   │         ------- shared statements
 7 │     } else if (value == 1) {
 8 │         record("one");
 9 │         flush();
   │         ------- shared statements
10 │     } else {
11 │         record("other");
12 │         flush();
   │         ^^^^^^^ repeated statements
13 │     }
14 │ }
   │
"#,
        );
    }

    /// Report a shared suffix that declares and then reads corresponding locals.
    #[test]
    fn test_reports_suffix_bindings() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: int32): void;
function finish(condition: boolean): void {
    if (condition) {
        record(0);
        const left = 1;
        record(left);
    } else {
        record(2);
        const right = 1;
        record(right);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[branches-sharing-code]: branch suffix repeats across alternatives
  ──▶ main.tspp:9:9
   │
 3 │     if (condition) {
 4 │         record(0);
 5 │         const left = 1;
   │         --------------- shared statements
 6 │         record(left);
   │         ------------
 7 │     } else {
 8 │         record(2);
 9 │         const right = 1;
   │         ^^^^^^^^^^^^^^^^ repeated statements
10 │         record(right);
   │         ^^^^^^^^^^^^^
11 │     }
12 │ }
   │
"#,
        );
    }

    /// Report a value tail shared by every match arm after distinct work.
    #[test]
    fn test_reports_shared_match_tail() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: int32): void;
function finish(value: boolean): int32 {
    match (value) {
        true => {
            record(1);
            0
        }
        false => {
            record(2);
            0
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[branches-sharing-code]: match arm suffix repeats across alternatives
  ──▶ main.tspp:10:13
   │
 4 │         true => {
 5 │             record(1);
 6 │             0
   │             - shared value
 7 │         }
 8 │         false => {
 9 │             record(2);
10 │             0
   │             ^ repeated value
11 │         }
12 │     }
   │
"#,
        );
    }

    /// Accept a common match result that cannot move beyond the surrounding expression.
    #[test]
    fn test_accepts_nonfinal_match_tail() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: int32): void;
function finish(value: boolean): int32 {
    const result = match (value) {
        true => {
            record(1);
            0
        }
        false => {
            record(2);
            0
        }
    };
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept match tails that read their respective arm bindings.
    #[test]
    fn test_accepts_match_binding_tails() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: int32): void;
function finish(value: (int32, boolean)): int32 {
    return match (value) {
        (left, true) => {
            record(1);
            left
        }
        (right, false) => {
            record(2);
            right
        }
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report only nonoverlapping common edges when both branches are equal.
    #[test]
    fn test_reports_equal_branches_once() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: string): void;
function finish(condition: boolean): void {
    if (condition) {
        record("same");
    } else {
        record("same");
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[branches-sharing-code]: branch prefix repeats across alternatives
 ──▶ main.tspp:6:9
  │
2 │ function finish(condition: boolean): void {
3 │     if (condition) {
4 │         record("same");
  │         -------------- shared statements
5 │     } else {
6 │         record("same");
  │         ^^^^^^^^^^^^^^ repeated statements
7 │     }
8 │ }
  │
"#,
        );
    }

    /// Preserve binding correspondence across a shared statement sequence.
    #[test]
    fn test_reports_renamed_prefix_bindings() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: int32): void;
function finish(condition: boolean): void {
    if (condition) {
        const left = 1;
        record(left);
    } else {
        const right = 1;
        record(right);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[branches-sharing-code]: branch prefix repeats across alternatives
  ──▶ main.tspp:7:9
   │
 2 │ function finish(condition: boolean): void {
 3 │     if (condition) {
 4 │         const left = 1;
   │         --------------- shared statements
 5 │         record(left);
   │         ------------
 6 │     } else {
 7 │         const right = 1;
   │         ^^^^^^^^^^^^^^^^ repeated statements
 8 │         record(right);
   │         ^^^^^^^^^^^^^
 9 │     }
10 │ }
   │
"#,
        );
    }

    /// Accept similar branch edges that read distinct free bindings.
    #[test]
    fn test_accepts_different_bindings() {
        let session = TestSession::dir(
            &BRANCHES_SHARING_CODE,
            r#"
declare function record(value: int32): void;
function finish(condition: boolean, left: int32, right: int32): void {
    if (condition) {
        record(left);
    } else {
        record(right);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
