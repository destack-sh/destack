use crate::tests::TestRewriter;

/// Reorder repeated callable parameters while preserving their source.
#[test]
fn test_rewrite_parameter_sequence() {
    TestRewriter::new(
        "function collect($FIRST, $$$REST): void {}",
        "function collect($$$REST, $FIRST): void {}",
        r#"
function collect(first: string): void {}
function collect(first: string, second: number, third: boolean): void {}
"#,
    )
    .assert(
        r#"
function collect(first: string): void {}
function collect(second: number, third: boolean, first: string): void {}
"#,
    );
}

/// Reorder a union while preserving separators inside its repeated capture.
#[test]
fn test_rewrite_type_sequence() {
    TestRewriter::new(
        "type Values = $FIRST | $$$REST",
        "type Values = $$$REST | $FIRST",
        "type Values = First | Second | Third\n",
    )
    .assert("type Values = Second | Third | First;\n");
}
