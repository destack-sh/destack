use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_literal_infers_property_types() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = { a: 1, b: "two" };
/// @type.symbol symbol=value type={ a: int32; b: string }
/// @type.node source="{ a: 1, b: \"two\" }" type={ a: 1; b: "two" }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
"#,
    );
}

#[test]
fn test_object_literal_uses_shorthand_binding_types() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const age = 42;
const person = { name, age };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const name = "Ada";
/// @type.symbol symbol=name type="Ada"
/// @type.node source="\"Ada\"" type="Ada"

const age = 42;
/// @type.symbol symbol=age type=42
/// @type.node source=42 type=42

const person = { name, age };
/// @type.symbol symbol=person type={ name: string; age: int32 }
/// @type.node source="{ name, age }" type={ name: "Ada"; age: 42 }
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
/// @type.node source=age type=42
/// @resolution.name source=age target=age
"#,
    );
}

#[test]
fn test_empty_object_literal_has_empty_shape() {
    let session = TestSession::single(
        r#"
const value = {};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = {};
/// @type.symbol symbol=value type={  }
/// @type.node source={} type={  }
"#,
    );
}

#[test]
fn test_const_asserted_object_preserves_literal_properties() {
    let session = TestSession::single(
        r#"
const value = { a: 1, b: "two" } as const;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = { a: 1, b: "two" } as const;
/// @type.symbol symbol=value type={ readonly a: 1; readonly b: "two" }
/// @type.node source="{ a: 1, b: \"two\" } as const" type={ readonly a: 1; readonly b: "two" }
/// @type.node source="{ a: 1, b: \"two\" }" type={ readonly a: 1; readonly b: "two" }
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
"#,
    );
}

#[test]
fn test_contextual_object_literal_rejects_property_mismatch() {
    let session = TestSession::single(
        r#"
const value: { a: number; b: string } = { a: 1, b: 2 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: { a: number; b: string } = { a: 1, b: 2 };
/// @type.symbol symbol=value type={ a: float64; b: string }
/// @type.symbol symbol=a type=float64
/// @type.symbol symbol=b type=string
/// @type.node source="{ a: 1, b: 2 }" type={ a: 1; b: 2 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=41 source="const value: { a: number; b: string } = { a: 1, b: 2 };"
"#,
    );
}

#[test]
fn test_satisfies_preserves_object_literal_members() {
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
/// @resolution.name source=Mode target=Mode

let config = { mode: "dev" } satisfies Shape;
/// @type.symbol symbol=config type={ mode: "dev" }
/// @type.node source="{ mode: \"dev\" } satisfies Shape" type={ mode: "dev" }
/// @type.node source="{ mode: \"dev\" }" type={ mode: "dev" }
/// @type.node source="\"dev\"" type="dev"
/// @resolution.name source=Shape target=Shape

const mode = config.mode;
/// @type.symbol symbol=mode type="dev"
/// @type.node source=config type={ mode: "dev" }
/// @type.node source=config.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } kind=field key=mode
"#,
    );
}
