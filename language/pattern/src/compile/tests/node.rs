use crate::tests::TestPattern;

/// Compile single and repeated node metavariables against authoritative DIR.
#[test]
fn test_compile_node_metavariables() {
    TestPattern::new("$VALUE + $VALUE").compile().assert(
        r#"
$VALUE + $VALUE
/// @pattern.root node=Expression source="$VALUE + $VALUE"
/// @pattern.metavariable name=VALUE kind=node node=Expression
/// @pattern.use name=VALUE kind=node node=Expression
/// @pattern.use name=VALUE kind=node node=Expression
"#,
    );
}

/// Retain lowercase and escaped dollar identifiers as concrete syntax.
#[test]
fn test_compile_literal_dollar_identifiers() {
    TestPattern::new("$local + \\u0024VALUE").compile().assert(
        r#"
$local + \u0024VALUE
/// @pattern.root node=Expression source="$local + \\u0024VALUE"
"#,
    );
}

/// Retain anonymous uses without declaring named metavariables.
#[test]
fn test_compile_anonymous_metavariables() {
    TestPattern::new("$_ + $_").compile().assert(
        r#"
$_ + $_
/// @pattern.root node=Expression source="$_ + $_"
/// @pattern.use name=_ kind=node node=Expression
/// @pattern.use name=_ kind=node node=Expression
"#,
    );
}

/// Ignore marker-shaped text inside literals and comments.
#[test]
fn test_compile_marker_text() {
    TestPattern::new("/* $COMMENT */ \"$LITERAL\"")
        .compile()
        .assert(
            r#"
/* $COMMENT */ "$LITERAL"
/// @pattern.root node=Expression source="\"$LITERAL\""
"#,
        );
}
