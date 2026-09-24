use crate::tests::{DirRows, TestSession};

#[test]
fn test_dereference_projects_borrowed_value() {
    let session = TestSession::single(
        r#"
declare const shared: &readonly int32;
const value = *shared;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const shared: &'static readonly int32;
const value: int32 = *shared;

=== dir ===
declare const shared: &readonly int32;
/// @type.symbol symbol=shared source=shared type=&'static readonly int32
/// @resolution.pattern source=shared kind=binding target=shared

const value = *shared;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.operator source=*shared type=int32 operator="*" kind=builtin operands=[shared as &'static readonly int32]
/// @resolution.name source=shared target=shared
/// @resolution.place source=shared placement="local" lifetime="static" access="immutable"
/// @resolution.access source=shared root=shared
"#,
    );
}
