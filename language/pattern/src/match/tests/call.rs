use crate::tests::TestMatcher;

/// Match empty and populated call argument sequences.
#[test]
fn test_match_call_arguments() {
    TestMatcher::new(
        "fetch($URL, $$$ARGUMENTS)",
        r#"
fetch(url)
fetch(url, init)
other(url)
"#,
    )
    .assert(
        r#"
fetch(url)
^^^^^^^^^^ match URL.node="url" ARGUMENTS.nodes=[]
fetch(url, init)
^^^^^^^^^^^^^^^^ match URL.node="url" ARGUMENTS.nodes=["init"]
other(url)
"#,
    );
}

/// Partition separated sequences from left to right using the shortest match.
#[test]
fn test_match_separated_sequences() {
    TestMatcher::new(
        "consume($$$BEFORE, target, $$$AFTER)",
        r#"
consume(a, target, b, target, c)
"#,
    )
    .assert(
        r#"
consume(a, target, b, target, c)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match BEFORE.nodes=["a"] AFTER.nodes=["b", "target", "c"]
"#,
    );
}

/// Match nested generic type arguments through call expressions.
#[test]
fn test_match_generic_call() {
    TestMatcher::new(
        "fetch<Result<$TYPE>>(value)",
        r#"
fetch<Result<User>>(value)
fetch<Option<User>>(value)
"#,
    )
    .assert(
        r#"
fetch<Result<User>>(value)
^^^^^^^^^^^^^^^^^^^^^^^^^^ match TYPE.node="User"
fetch<Option<User>>(value)
"#,
    );
}
