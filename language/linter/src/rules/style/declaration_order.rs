use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require declarations in the canonical source order.
    pub DECLARATION_ORDER {
        id: "declaration-order",
        summary: "Require declarations in the canonical source order",
        explanation: r#"
A consistent declaration order makes modules and types predictable to scan. Keep imports first,
constants before declarations, principal types before their extensions, and tests last; preserve
authored order where it affects overload selection or representation.
"#,
        example: {
            reported: r#"
function run(): void {}

const RETRY_LIMIT = 3;
"#,
            accepted: r#"
const RETRY_LIMIT = 3;

function run(): void {}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
