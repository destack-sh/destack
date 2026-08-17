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

/// Match the complete operand sequence of one match binding guard.
#[test]
fn test_match_binding_guard() {
    TestMatcher::new(
        "match (value) { text if (let parsed! = $SOURCE && parsed > 0) => parsed }",
        r#"
match (value) { text if (let parsed! = parse(text) && parsed > 0) => parsed }
match (value) { text if (let parsed = parse(text) && parsed > 0) => parsed }
match (value) { text if (let parsed! = convert(text) && parsed > 0) => parsed }
match (value) { text if (ready) => text }
"#,
    )
    .assert(
        r#"
match (value) { text if (let parsed! = parse(text) && parsed > 0) => parsed }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match SOURCE.node="parse(text)"
match (value) { text if (let parsed = parse(text) && parsed > 0) => parsed }
match (value) { text if (let parsed! = convert(text) && parsed > 0) => parsed }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match SOURCE.node="convert(text)"
match (value) { text if (ready) => text }
"#,
    );
}
