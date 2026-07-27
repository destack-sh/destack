use crate::tests::TestRewriter;

/// Rewrite every nonoverlapping match against the original source.
#[test]
fn test_rewrite_nonoverlapping_matches() {
    TestRewriter::new("old($VALUE)", "fresh($VALUE)", "old(first) + old(second)")
        .assert("fresh(first) + fresh(second);\n");
}

/// Do not search replacement output again during one rewrite.
#[test]
fn test_rewrite_once() {
    TestRewriter::new("read($VALUE)", "read(read($VALUE))", "read(value)")
        .assert("read(read(value));\n");
}
