use crate::tests::{DirRows, TestSession};

#[test]
fn test_explicit_cast_coerces_literal_to_target_type() {
    let session = TestSession::single(
        r#"
const value = 1 as int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
const value = 1 as int32;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="1 as int32" type=int32
/// @type.node source=1 type=1
/// @coercion.node source="1 as int32" from=1 to=int32 origin=explicit
"#,
    );
}
