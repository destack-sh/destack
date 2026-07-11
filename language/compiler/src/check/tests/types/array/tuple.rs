use crate::tests::{DirRows, TestSession};

#[test]
fn test_tuple_optional_absorption_reshapes_the_value() {
    let session = TestSession::single(
        r#"
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair;
const same: (int32, int32) = pair;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair as (int32, int32, int32?);
const same: (int32, int32) = pair;

=== checked ===
declare const pair: (int32, int32);
/// @type.symbol symbol=pair source=pair type=(int32, int32)

const triple: (int32, int32, int32?) = pair;
/// @type.symbol symbol=triple source=triple type=(int32, int32, int32?)
/// @resolution.name source=pair target=pair

const same: (int32, int32) = pair;
/// @type.symbol symbol=same source=same type=(int32, int32)
/// @resolution.name source=pair target=pair
"#,
    );
}

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
=== annotated ===
const tuple: readonly ["id", 42] = ["id", 42] as const;
const name: "id" = tuple[0];
const count: 42 = tuple[1];

=== checked ===
const tuple = ["id", 42] as const;
/// @type.symbol symbol=tuple source=tuple type=readonly ["id", 42] reduced=["id", 42]

const name = tuple[0];
/// @type.symbol symbol=name source=name type="id"
/// @resolution.name source=tuple target=tuple
/// @resolution.member source=tuple[0] receiver=["id", 42] kind=element index=0

const count = tuple[1];
/// @type.symbol symbol=count source=count type=42
/// @resolution.name source=tuple target=tuple
/// @resolution.member source=tuple[1] receiver=["id", 42] kind=element index=1
"#,
    );
}
