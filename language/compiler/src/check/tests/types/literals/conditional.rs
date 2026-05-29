use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_conditional_preserves_literal_union() {
    let session = TestSession::single(
        r#"
const value = true ? 1 : 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = true ? 1 : 2;
/// @type.symbol symbol=value type=1 | 2
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=boolean
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_let_conditional_widens_literal_union() {
    let session = TestSession::single(
        r#"
let value = true ? 1 : 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = true ? 1 : 2;
/// @type.symbol symbol=value type=int32
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=boolean
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}
