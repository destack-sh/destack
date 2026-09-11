use crate::tests::TestMatcher;

/// Match qualified callees by their exact authored DIR structure.
#[test]
fn test_match_qualified_callee_structure() {
    TestMatcher::new(
        "myPackage.net.fetch($VALUE)",
        r#"
myPackage.net.fetch(first)
fetch(second)
alias.fetch(third)
"#,
    )
    .assert(
        r#"
myPackage.net.fetch(first)
^^^^^^^^^^^^^^^^^^^^^^^^^^ match VALUE.node="first"
fetch(second)
alias.fetch(third)
"#,
    );
}

/// Match equivalent borrow qualifier orders and reject different guarantees.
#[test]
fn test_match_borrow_qualifiers() {
    TestMatcher::new(
        "&readonly exclusive $VALUE",
        r#"
&readonly exclusive first;
&exclusive readonly second;
&readonly third;
&exclusive fourth;
"#,
    )
    .assert(
        r#"
&readonly exclusive first;
^^^^^^^^^^^^^^^^^^^^^^^^^ match VALUE.node="first"
&exclusive readonly second;
^^^^^^^^^^^^^^^^^^^^^^^^^^ match VALUE.node="second"
&readonly third;
&exclusive fourth;
"#,
    );
}
