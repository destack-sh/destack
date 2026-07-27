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
