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

/// Match borrow expressions by access.
#[test]
fn test_match_borrow_qualifiers() {
    TestMatcher::new(
        "&immutable $VALUE",
        r#"
&immutable first;
&second;
&readonly third;
&exclusive fourth;
"#,
    )
    .assert(
        r#"
&immutable first;
^^^^^^^^^^^^^^^^ match VALUE.node="first"
&second;
&readonly third;
&exclusive fourth;
"#,
    );
}
