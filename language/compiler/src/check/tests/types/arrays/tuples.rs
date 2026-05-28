use crate::tests::{DirRows, TestSession};

#[test]
fn test_tuple_subscript_selects_literal_element() {
    let session = TestSession::single(
        r#"
const tuple = ["id", 42] as const;
const name = tuple[0];
const count = tuple[1];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
const tuple = ["id", 42] as const;
/// @type.symbol symbol=tuple type=readonly ["id", 42]

const name = tuple[0];
/// @resolution.name source=tuple target=tuple
/// @resolution.member source="tuple[0]" receiver=readonly ["id", 42] kind=symbol target=0
/// @type.symbol symbol=name type="id"

const count = tuple[1];
/// @resolution.name source=tuple target=tuple
/// @resolution.member source="tuple[1]" receiver=readonly ["id", 42] kind=symbol target=1
/// @type.symbol symbol=count type=42
"#,
    );
}
