use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow mutating range bounds during iteration over that range.
    pub NO_MUTATED_RANGE_BOUND {
        id: "no-mutated-range-bound",
        summary: "Disallow mutating range bounds during iteration over that range",
        explanation: r#"
A range captures its bounds before iteration, so mutating their source storage cannot change the active traversal.
Instead, you SHOULD leave captured bounds unchanged or use a conditional loop when each iteration must observe an updated bound.
"#,
        example: {
            reported: r#"
function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        end -= 1;
        value;
    }
}
"#,
            accepted: r#"
function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        value;
    }
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Validate the canonical lint example.
    #[ignore]
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&NO_MUTATED_RANGE_BOUND);
    }

    /// Report mutation of a captured range end.
    #[ignore]
    #[test]
    fn test_reports_mutated_end() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            NO_MUTATED_RANGE_BOUND.example.reported(),
        );

        session.assert_diagnostics(
            r#"
warning[no-mutated-range-bound]: range bound is mutated after the range captures it
 ──▶ main.ds:4:9
  │
2 │     let end = limit;
3 │     for (const value of 0..end) {
4 │         end -= 1;
  │         ^^^
5 │         value;
6 │     }
  │
"#,
        );
    }

    /// Report mutation through an exclusive borrow of a range bound.
    #[ignore]
    #[test]
    fn test_reports_borrowed_bound() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
declare function reset(value: &exclusive int32): void;

function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        reset(&exclusive end);
        value;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-mutated-range-bound]: range bound is mutated after the range captures it
 ──▶ main.ds:6:15
  │
4 │     let end = limit;
5 │     for (const value of 0..end) {
6 │         reset(&exclusive end);
  │               ^^^^^^^^^^^^^^
7 │         value;
8 │     }
  │
"#,
        );
    }

    /// Accept mutation of storage not used by the iterated range.
    #[ignore]
    #[test]
    fn test_accepts_unrelated_mutation() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function visit(limit: int32): void {
    let count = 0;
    for (const value of 0..limit) {
        count += 1;
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a range whose bounds are constants.
    #[ignore]
    #[test]
    fn test_accepts_constant_bounds() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
for (const value of 0..10) {
    value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
