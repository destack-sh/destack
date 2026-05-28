use crate::tests::{DirRows, TestSession};

#[test]
fn test_local_reference_selects_declared_binding() {
    let session = TestSession::single(
        r#"
const value = 1;
const copy = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = 1;
/// @type.symbol symbol=value type=1
/// @type.node source=1 type=1

const copy = value;
/// @type.symbol symbol=copy type=1
/// @type.node source=value type=1
/// @resolution.name source=value target=value
"#,
    );
}
