use crate::tests::{DirRows, TestSession};

#[test]
fn test_union_newtype_uses_variant_layout() {
    let session = TestSession::single(
        r#"
struct Rectangle {}
struct Circle {}

newtype Shape = Rectangle | Circle;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_layout(),
        r#"
struct Rectangle {}
/// @type.symbol symbol=Rectangle type=Rectangle

struct Circle {}
/// @type.symbol symbol=Circle type=Circle

newtype Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape type=Shape
/// @layout.type type=Shape shape=variant
"#,
    );
}
