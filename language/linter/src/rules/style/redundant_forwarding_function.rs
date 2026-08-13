use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Discourage functions that only forward their parameters unchanged.
    pub REDUNDANT_FORWARDING_FUNCTION {
        id: "redundant-forwarding-function",
        summary: "Discourage functions that only forward their parameters unchanged",
        explanation: r#"
A function whose complete body calls another callable with the same parameters adds a name without
adding behavior. Use the underlying callable directly; intentional API aliases, protocol
implementations, decorators, and foreign boundaries are exempt.
"#,
        example: {
            reported: r#"
function parseUser(source: string): Result<User, ParseError> {
    return User.parse(source);
}
"#,
            accepted: r#"
const parseUser = User.parse;
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
