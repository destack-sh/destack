use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_widens_array_literals_without_const_assertions() {
    let session = TestSession::single(
        r#"
let mutableValues = [1, 2];
const constValues = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let mutableValues = [1, 2];
/// @type.symbol symbol=mutableValues type=collections.array.Array<int32>
/// @type.node source=[1, 2] type=collections.array.Array<int32>
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32

const constValues = [1, 2];
/// @type.symbol symbol=constValues type=collections.array.Array<int32>
/// @type.node source=[1, 2] type=collections.array.Array<int32>
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
"#,
    );
}

#[test]
fn test_check_preserves_const_asserted_tuple_literals() {
    let session = TestSession::single(
        r#"
const values = [1, 2] as const;
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const values = [1, 2] as const;
/// @type.symbol symbol=values type=readonly [1, 2]
/// @type.node source="[1, 2] as const" type=readonly [1, 2]

const first = values[0];
/// @type.symbol symbol=first type=1
/// @type.node source=values type=readonly [1, 2]
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @type.node source=0 type=0
"#,
    );
}

#[test]
fn test_check_records_explicit_cast_coercion() {
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
/// @type.symbol symbol=value type=int32
/// @type.node source="1 as int32" type=int32
/// @type.node source=1 type=1
/// @coercion.node source="1 as int32" from=1 to=int32 origin=explicit
"#,
    );
}

#[test]
fn test_check_preserves_satisfies_literal_members() {
    let session = TestSession::single(
        r#"
type Mode = "dev" | "prod";
type Shape = { mode: Mode };

let config = { mode: "dev" } satisfies Shape;
const mode = config.mode;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode type="dev" | "prod"

type Shape = { mode: Mode };
/// @type.symbol symbol=Shape type={ mode: "dev" | "prod" }
/// @type.symbol symbol=Shape.mode type="dev" | "prod"

let config = { mode: "dev" } satisfies Shape;
/// @type.symbol symbol=config type={ mode: "dev" }
/// @type.node source="{ mode: \"dev\" } satisfies Shape" type={ mode: "dev" }
/// @type.node source="{ mode: \"dev\" }" type={ mode: "dev" }
/// @type.node source="\"dev\"" type="dev"

const mode = config.mode;
/// @type.symbol symbol=mode type="dev"
/// @type.node source=config type={ mode: "dev" }
/// @type.node source=config.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } kind=field key=mode
"#,
    );
}
