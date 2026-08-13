use crate::rules::declare_lint_stub;

declare_lint_stub! {
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
        category: Style,
        level: Warning,
        fixable: Suggestion,
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
        TestSession::assert_example(&PREFER_LOOP_CONDITION);
    }

    /// Move a leading break condition into a while header.
    #[ignore]
    #[test]
    fn test_replaces_leading_break() {
        let session = TestSession::dir(
            &PREFER_LOOP_CONDITION,
            PREFER_LOOP_CONDITION.example.reported(),
        );

        session.assert_suggestions(PREFER_LOOP_CONDITION.example.accepted());
    }

    /// Remove an existing negation when forming the continuation condition.
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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

    /// Accept a leading break that exits an outer loop.
    #[ignore]
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
    #[ignore]
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
  ──▶ main.ds:6:5
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
