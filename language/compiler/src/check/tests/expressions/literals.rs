use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_literal_types() {
    let session = TestSession::single(
        r#"
const literal = 42;
let mutable = 42;
const widened: int32 = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const literal = 42;
/// @type.symbol symbol=literal type=42
/// @type.node source=42 type=42

let mutable = 42;
/// @type.symbol symbol=mutable type=int32
/// @type.node source=42 type=int32

const widened: int32 = 42;
/// @type.symbol symbol=widened type=int32
/// @type.node source=42 type=int32
"#,
    );
}
