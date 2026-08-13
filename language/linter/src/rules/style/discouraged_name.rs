use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Discourage generic names that hide a declaration's role.
    pub DISCOURAGED_NAME {
        id: "discouraged-name",
        summary: "Discourage generic names that hide a declaration's role",
        explanation: r#"
Names such as `helper`, `util`, `wrapper`, `info`, and `support` obscure the concrete noun or verb
owned by a declaration. Use the standardized denylist only for declarations where the word carries
no domain meaning; generated and foreign names are exempt.
"#,
        example: {
            reported: r#"
function helper(order: &readonly Order): Money {
    return order.items.sum(Item.price);
}
"#,
            accepted: r#"
function calculateTotal(order: &readonly Order): Money {
    return order.items.sum(Item.price);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
