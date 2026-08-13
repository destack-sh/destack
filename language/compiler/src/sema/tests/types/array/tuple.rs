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
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair as (int32, int32, int32?);
const same: (int32, int32) = pair;

=== checked ===
declare const pair: (int32, int32);
/// @type.symbol symbol=pair source=pair type=(int32, int32)
/// @resolution.pattern source=pair kind=binding target=pair

const triple: (int32, int32, int32?) = pair;
/// @type.symbol symbol=triple source=triple type=(int32, int32, int32?)
/// @resolution.pattern source=triple kind=binding target=triple
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=pair root=pair
/// @coercion.node source=pair from=(int32, int32) adjustments=[{ kind: tuple, target: (int32, int32, int32?) }] origin=implicit

const same: (int32, int32) = pair;
/// @type.symbol symbol=same source=same type=(int32, int32)
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=pair root=pair
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
/// @type.symbol symbol=tuple source=tuple type=readonly ["id", 42]
/// @resolution.pattern source=tuple kind=binding target=tuple

const name = tuple[0];
/// @type.symbol symbol=name source=name type="id"
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="static" access="readonly"
/// @resolution.access source=tuple root=tuple
/// @resolution.access source=tuple[0] root=tuple keys=[0]
/// @resolution.subscript source=tuple[0] type="id" kind=member target="receiver=readonly [\"id\", 42], target=field(receiver=[\"id\", 42], target=0, type=\"id\"), type=\"id\""

const count = tuple[1];
/// @type.symbol symbol=count source=count type=42
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="static" access="readonly"
/// @resolution.access source=tuple root=tuple
/// @resolution.access source=tuple[1] root=tuple keys=[1]
/// @resolution.subscript source=tuple[1] type=42 kind=member target="receiver=readonly [\"id\", 42], target=field(receiver=[\"id\", 42], target=1, type=42), type=42"
"#,
    );
}
