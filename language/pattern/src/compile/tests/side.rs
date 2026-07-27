use crate::tests::TestPattern;

/// Compile member names as scalar side uses rather than synthetic nodes.
#[test]
fn test_compile_member_name() {
    TestPattern::new("$OBJECT.$MEMBER").compile().assert(
        r#"
$OBJECT.$MEMBER
/// @pattern.root node=Expression source="$OBJECT.$MEMBER"
/// @pattern.metavariable name=OBJECT kind=node node=Expression
/// @pattern.use name=OBJECT kind=node node=Expression
/// @pattern.metavariable name=MEMBER kind=name
/// @pattern.use name=MEMBER kind=name node=Expression span=Main
"#,
    );
}
