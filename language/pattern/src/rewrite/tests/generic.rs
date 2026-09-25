use tspp_dir as dir;

use crate::tests::TestRewriter;

/// Rewrite a generic parameter selected from a declaration context.
#[test]
fn test_rewrite_generic_parameter() {
    TestRewriter::context(
        "function f<$NAME: $CONSTRAINT = $DEFAULT>() {}",
        "function f<$NAME: Wrapped<$CONSTRAINT> = $DEFAULT>() {}",
        dir::NodeType::GenericParameter,
        r#"
function first<Value: Serializable = DefaultValue>() {}
function second<Item>() {}
"#,
    )
    .assert(
        r#"
function first<Value: Wrapped<Serializable> = DefaultValue>() {}
function second<Item>() {}
"#,
    );
}
