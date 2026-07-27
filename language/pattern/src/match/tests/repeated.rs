use crate::tests::TestMatcher;

/// Require repeated node metavariables to be structurally equal.
#[test]
fn test_match_repeated_node() {
    TestMatcher::new(
        "$VALUE + $VALUE",
        r#"
left + left
left + right
"#,
    )
    .assert(
        r#"
left + left
^^^^^^^^^^^ match VALUE.node="left"
left + right
"#,
    );
}

/// Compare complete supported member and call subtrees for repeated bindings.
#[test]
fn test_match_repeated_compound_nodes() {
    TestMatcher::new(
        "$VALUE + $VALUE",
        r#"
user.name + user.name
fetch(value) + fetch(value)
fetch(left) + fetch(right)
"#,
    )
    .assert(
        r#"
user.name + user.name
^^^^^^^^^^^^^^^^^^^^^ match VALUE.node="user.name"
fetch(value) + fetch(value)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ match VALUE.node="fetch(value)"
fetch(left) + fetch(right)
"#,
    );
}
