use destack_dir as dir;

use crate::tests::TestMatcher;

/// Combine structural matching with an ancestor relation.
#[test]
fn test_match_inside() {
    TestMatcher::new(
        "$OBJECT.$MEMBER",
        r#"
outside.name
consume(user.name)
"#,
    )
    .inside(dir::NodeType::Expression)
    .assert(
        r#"
outside.name
consume(user.name)
        ^^^^^^^^^ match OBJECT.node="user" MEMBER.name="name"
"#,
    );
}

/// Combine structural matching with a descendant relation.
#[test]
fn test_match_has() {
    TestMatcher::new(
        "$VALUE",
        r#"
1
fetch(url)
"#,
    )
    .has(dir::NodeType::Expression)
    .assert(
        r#"
1
fetch(url)
^^^^^^^^^^ match VALUE.node="fetch(url)"
"#,
    );
}

/// Match candidates with later and earlier siblings.
#[test]
fn test_match_sibling_relations() {
    let source = r#"
target(first)
between()
target(second)
"#;

    TestMatcher::new("target($VALUE)", source)
        .precedes(dir::NodeType::Expression)
        .assert(
            r#"
target(first)
^^^^^^^^^^^^^ match VALUE.node="first"
between()
target(second)
"#,
        );

    TestMatcher::new("target($VALUE)", source)
        .follows(dir::NodeType::Expression)
        .assert(
            r#"
target(first)
between()
target(second)
^^^^^^^^^^^^^^ match VALUE.node="second"
"#,
        );
}

/// Match one exact one-based sibling position.
#[test]
fn test_match_nth_child() {
    TestMatcher::new(
        "target($VALUE)",
        r#"
target(first)
target(second)
target(third)
"#,
    )
    .nth_child(2)
    .assert(
        r#"
target(first)
target(second)
^^^^^^^^^^^^^^ match VALUE.node="second"
target(third)
"#,
    );
}
