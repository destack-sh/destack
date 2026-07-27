use crate::tests::TestMatcher;

/// Match repeated generic parameters and callable parameters.
#[test]
fn test_match_declaration_sequences() {
    TestMatcher::new(
        "function collect<$$$GENERICS>($FIRST, $$$REST): void {}",
        r#"
function collect(value: string): void {}
function collect<T>(value: T, count: int32): void {}
"#,
    )
    .assert(
        r#"
function collect(value: string): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match GENERICS.nodes=[] FIRST.node="value: string" REST.nodes=[]
function collect<T>(value: T, count: int32): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match GENERICS.nodes=["T"] FIRST.node="value: T" REST.nodes=["count: int32"]
"#,
    );
}

/// Match repeated property and union-type sequences.
#[test]
fn test_match_value_and_type_sequences() {
    TestMatcher::new(
        "({ first: $FIRST, $$$REST })",
        r#"
({ first: 1 })
({ first: 1, second: 2, third: 3 })
"#,
    )
    .assert(
        r#"
({ first: 1 })
 ^^^^^^^^^^^^ match FIRST.node="1" REST.nodes=[]
({ first: 1, second: 2, third: 3 })
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match FIRST.node="1" REST.nodes=["second: 2", "third: 3"]
"#,
    );

    TestMatcher::new(
        "type Values = $FIRST | $$$REST",
        r#"
type Values = First | Second
type Values = First | Second | Third
"#,
    )
    .assert(
        r#"
type Values = First | Second
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match FIRST.node="First" REST.nodes=["Second"]
type Values = First | Second | Third
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match FIRST.node="First" REST.nodes=["Second", "Third"]
"#,
    );
}
