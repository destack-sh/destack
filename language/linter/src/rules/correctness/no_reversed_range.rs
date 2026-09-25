use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow constant ranges whose start exceeds their end.
    pub NO_REVERSED_RANGE {
        id: "no-reversed-range",
        summary: "Disallow constant ranges whose start exceeds their end",
        explanation: r#"
An ascending range with a constant start greater than its end cannot yield a value.
Instead, you MUST order the endpoints from lower to higher.
"#,
        example: {
            reported: r#"
function values(): Range<int32> {
    return 10..0;
}
"#,
            accepted: r#"
function values(): Range<int32> {
    return 0..10;
}
"#,
        },
        provenance: [Clippy("reversed_empty_ranges")],
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report constant ascending ranges with reversed endpoints.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect range expressions with two exact scalar endpoints
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::RangeExpression {
            start: Some(start),
            end: Some(end),
            ..
        } = node
        else {
            continue;
        };
        let Some(start_value) = module.scalar_constant(*start)? else {
            continue;
        };
        let Some(end_value) = module.scalar_constant(*end)? else {
            continue;
        };
        if start_value.interval_ordering(&end_value) != Some(std::cmp::Ordering::Greater) {
            continue;
        }

        // report the complete empty range
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("range start exceeds its end", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report reversed integer and character ranges.
    #[test]
    fn test_reports_reversed_constant_ranges() {
        let session = TestSession::dir(
            &NO_REVERSED_RANGE,
            r#"
const Start: int32 = 10;
const integers = Start..0;
const bigints = 10n..0n;
const characters = 'z'..='a';
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-reversed-range]: range start exceeds its end
 ──▶ main.tspp:2:18
  │
1 │ const Start: int32 = 10;
2 │ const integers = Start..0;
  │                  ^^^^^^^^
3 │ const bigints = 10n..0n;
4 │ const characters = 'z'..='a';
  │

warning[no-reversed-range]: range start exceeds its end
 ──▶ main.tspp:3:17
  │
1 │ const Start: int32 = 10;
2 │ const integers = Start..0;
3 │ const bigints = 10n..0n;
  │                 ^^^^^^^
4 │ const characters = 'z'..='a';
  │

warning[no-reversed-range]: range start exceeds its end
 ──▶ main.tspp:4:20
  │
2 │ const integers = Start..0;
3 │ const bigints = 10n..0n;
4 │ const characters = 'z'..='a';
  │                    ^^^^^^^^^
  │
"#,
        );
    }

    /// Accept ordered, empty, unbounded, and dynamic ranges.
    #[test]
    fn test_accepts_nonreversed_ranges() {
        let session = TestSession::dir(
            &NO_REVERSED_RANGE,
            r#"
declare const end: int32;
const ordered = 0..10;
const empty = 0..0;
const dynamic = 10..end;
const unbounded = 10..;
"#,
        );

        session.assert_no_diagnostics();
    }
}
