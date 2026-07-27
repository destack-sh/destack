use crate::tests::TestMatcher;

/// Bind member names through scalar side positions.
#[test]
fn test_match_member_name() {
    TestMatcher::new(
        "$OBJECT.$MEMBER",
        r#"
user.name
user.email
"#,
    )
    .assert(
        r#"
user.name
^^^^^^^^^ match OBJECT.node="user" MEMBER.name="name"
user.email
^^^^^^^^^^ match OBJECT.node="user" MEMBER.name="email"
"#,
    );
}
