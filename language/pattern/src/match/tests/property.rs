use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match nominal members selected from a declaration context.
#[test]
fn test_match_member() {
    TestMatcher::context(
        "struct Placeholder { $NAME: $TYPE }",
        dir::NodeType::Member,
        r#"
struct User {
    name: string;
    age: int32
}
"#,
    )
    .assert(
        r#"
struct User {
    name: string;
    ^^^^^^^^^^^^ match NAME.name="name" TYPE.node="string"
    age: int32
    ^^^^^^^^^^ match NAME.name="age" TYPE.node="int32"
}
"#,
    );
}

/// Match type members selected from an object type context.
#[test]
fn test_match_type_member() {
    TestMatcher::context(
        "type Placeholder = { $NAME: $TYPE }",
        dir::NodeType::TypeMember,
        r#"
type User = {
    name: string;
    age: int32
}
"#,
    )
    .assert(
        r#"
type User = {
    name: string;
    ^^^^^^^^^^^^ match NAME.name="name" TYPE.node="string"
    age: int32
    ^^^^^^^^^^ match NAME.name="age" TYPE.node="int32"
}
"#,
    );
}
