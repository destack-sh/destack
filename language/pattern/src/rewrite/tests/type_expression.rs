use tspp_dir as dir;

use crate::tests::TestRewriter;

/// Rewrite a contextual tuple element while preserving its captured label.
#[test]
fn test_rewrite_tuple_element() {
    TestRewriter::context(
        "type Placeholder = ($NAME: $TYPE)",
        "type Placeholder = ($NAME: readonly $TYPE)",
        dir::NodeType::TupleElement,
        r#"
type Entry = (value: string)
"#,
    )
    .assert(
        r#"
type Entry = (value: readonly string,);
"#,
    );
}
