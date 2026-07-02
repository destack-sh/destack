use crate::tests::{DirRows, TestSession};

#[test]
fn test_dereference_projects_borrowed_value() {
    let session = TestSession::single(
        r#"
declare const shared: &readonly int32;
const value = *shared;
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
declare const shared: &readonly int32;
const value: int32 = *shared;

=== checked ===
declare const shared: &readonly int32;
/// @type.symbol symbol=shared source=shared type=&readonly int32

const value = *shared;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.call source=*shared parameters=() return=int32 kind=builtin builtin=unary.dereference
/// @resolution.name source=shared target=shared
"#);
}
