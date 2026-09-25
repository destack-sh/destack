use tspp_dir as dir;

use crate::tests::TestPattern;

/// Compile a repeated metavariable in a destructuring field list.
#[test]
fn test_compile_pattern_fields() {
    TestPattern::context(
        "match (value) { Point { $$$FIELDS } => body }",
        dir::NodeType::MatchArm,
    )
    .compile()
    .assert(
        r#"
match (value) { Point { $$$FIELDS } => body }
/// @pattern.root node=MatchArm source="Point { $$$FIELDS } => body"
/// @pattern.metavariable name=FIELDS kind=nodes node=PatternField
/// @pattern.use name=FIELDS kind=nodes node=PatternField
"#,
    );
}
