use crate::tests::TestMatcher;

/// Match the complete operand sequence of one while binding condition.
#[test]
fn test_match_while_binding_condition() {
    TestMatcher::new(
        "while (let value! = $SOURCE && value.isReady()) {}",
        r#"
while (let value! = first && value.isReady()) {}
while (let value = first && value.isReady()) {}
while (let value! = second && value.isReady()) {}
while (ready) {}
"#,
    )
    .assert(
        r#"
while (let value! = first && value.isReady()) {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match SOURCE.node="first"
while (let value = first && value.isReady()) {}
while (let value! = second && value.isReady()) {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match SOURCE.node="second"
while (ready) {}
"#,
    );
}
