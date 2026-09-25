use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match tree attributes and children selected from a tree expression.
#[test]
fn test_match_tree_nodes() {
    TestMatcher::context(
        "<Panel title={$TITLE} />",
        dir::NodeType::TreeAttribute,
        r#"
<Panel title={heading} />
<Panel count={total} />
"#,
    )
    .assert(
        r#"
<Panel title={heading} />
       ^^^^^^^^^^^^^^^ match TITLE.node="heading"
<Panel count={total} />
"#,
    );

    TestMatcher::context(
        "<Panel>{$CHILD}</Panel>",
        dir::NodeType::TreeChild,
        r#"
<Panel>{content}</Panel>
<Panel>plain text</Panel>
"#,
    )
    .assert(
        r#"
<Panel>{content}</Panel>
       ^^^^^^^^^ match CHILD.node="content"
<Panel>plain text</Panel>
"#,
    );
}
