use crate::tests::TestRewriter;

/// Preserve zero or more captured arguments while changing a call target.
#[test]
fn test_rewrite_call_arguments() {
    TestRewriter::new(
        "fetch($URL, $$$ARGUMENTS)",
        "client.fetch($URL, $$$ARGUMENTS)",
        r#"
fetch(url)
fetch(url, init)
"#,
    )
    .assert(
        r#"
client.fetch(url);
client.fetch(url, init);
"#,
    );
}

/// Preserve decorators attached to one opaque node capture.
#[test]
fn test_rewrite_decorated_node() {
    TestRewriter::new(
        "function example(): void { $BODY }",
        "function renamed(): void { $BODY }",
        r#"
function example(): void {
    @trace
    first()
}
"#,
    )
    .assert(
        r#"
function renamed(): void {
    @trace
    first();
}
"#,
    );
}

/// Replace authored decorators around one opaque node capture.
#[test]
fn test_rewrite_metavariable_decorator() {
    TestRewriter::new(
        "@trace\n$NODE",
        "@logged\n$NODE",
        r#"
@trace
first()
"#,
    )
    .assert(
        r#"
@logged
first();
"#,
    );
}

/// Insert one decorator while preserving an explicitly captured decorator list.
#[test]
fn test_rewrite_metavariable_decorators() {
    TestRewriter::new(
        "@trace\n@$$$DECORATORS\n$NODE",
        "@logged\n@trace\n@$$$DECORATORS\n$NODE",
        r#"
@trace
@memo
first()
"#,
    )
    .assert(
        r#"
@logged
@trace
@memo
first();
"#,
    );
}
