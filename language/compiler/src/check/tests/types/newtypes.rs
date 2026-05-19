use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_newtype_union_layout() {
    assert_check_snapshot(
        r#"
struct Rectangle {}
struct Circle {}

newtype Shape = Rectangle | Circle;
"#,
        r#"
struct Rectangle {}
/// @type.symbol key=Rectangle value=Rectangle

struct Circle {}
/// @type.symbol key=Circle value=Circle

newtype Shape = Rectangle | Circle;
/// @type.symbol key=Shape value=Shape
/// @layout.type type=Shape layout=layout0 shape=variant

/// @layout.entry layout=layout0 shape=variant
/// @layout.summary layouts=1 types=1
/// @type.summary types=4 nodes=0 symbols=3
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=0 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
"#,
    );
}
