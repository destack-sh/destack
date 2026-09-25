use tspp_dir as dir;

use crate::tests::TestPattern;

/// Select a match arm from a complete parse context.
#[test]
fn test_compile_match_arm_context() {
    TestPattern::context(
        "match (value) { Err($ERROR) => $BODY }",
        dir::NodeType::MatchArm,
    )
    .compile()
    .assert(
        r#"
match (value) { Err($ERROR) => $BODY }
/// @pattern.root node=MatchArm source="Err($ERROR) => $BODY"
/// @pattern.metavariable name=ERROR kind=node node=Pattern
/// @pattern.use name=ERROR kind=node node=Pattern
/// @pattern.metavariable name=BODY kind=node node=Expression
/// @pattern.use name=BODY kind=node node=Expression
"#,
    );
}

/// Select the outer root of a recursively nested node family.
#[test]
fn test_compile_recursive_context_root() {
    TestPattern::context(
        "({ value: $TARGET, $$$REST } = source)",
        dir::NodeType::AssignPattern,
    )
    .compile()
    .assert(
        r#"
({ value: $TARGET, $$$REST } = source)
/// @pattern.root node=AssignPattern source="{ value: $TARGET, $$$REST }"
/// @pattern.metavariable name=TARGET kind=node node=Expression
/// @pattern.use name=TARGET kind=node node=Expression
/// @pattern.metavariable name=REST kind=nodes node=AssignPatternField
/// @pattern.use name=REST kind=nodes node=AssignPatternField
"#,
    );
}
