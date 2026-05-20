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
        DirRows::checked(),
        r#"
let mutableValues = [1, 2];
/// @type.node source="[1, 2]" type=int32[]
/// @type.symbol symbol=mutableValues type=int32[]

const constValues = [1, 2];
/// @type.node source="[1, 2]" type=int32[]
/// @type.symbol symbol=constValues type=int32[]
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
        DirRows::checked(),
        r#"
const values = [1, 2] as const;
/// @type.node source="[1, 2] as const" type=readonly [1, 2]
/// @type.symbol symbol=values type=readonly [1, 2]

const first = values[0];
/// @resolution.name source=values target=values
/// @resolution.member source="values[0]" receiver=readonly [1, 2] kind=direct target=0
/// @type.symbol symbol=first type=1
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
        DirRows::checked(),
        r#"
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode type="dev" | "prod"

type Shape = { mode: Mode };
/// @resolution.name source=Mode target=Mode
/// @type.symbol symbol=Shape type={ mode: Mode }

let config = { mode: "dev" } satisfies Shape;
/// @resolution.name source=Shape target=Shape
/// @type.node source="{ mode: \"dev\" } satisfies Shape" type={ mode: "dev" }
/// @type.symbol symbol=config type={ mode: "dev" }

const mode = config.mode;
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } kind=direct target=config.mode
/// @type.symbol symbol=mode type="dev"
"#,
    );
}
