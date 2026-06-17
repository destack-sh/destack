use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_binds_fields() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32; y: string };

let { x, y } = point;

x satisfies int32;
y satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: string };

let { x, y } = point;

x satisfies int32;
y satisfies string;

=== checked ===
declare const point: { x: int32; y: string };
/// @type.symbol symbol=point source=point type={ x: int32; y: string }

let { x, y } = point;
/// @type.symbol symbol=x source=x type=int32
/// @type.symbol symbol=y source=y type=string
/// @resolution.pattern source="{ x, y }" kind=object fields=[x, y]
/// @type.node source=point type={ x: int32; y: string }
/// @resolution.name source=point target=point

x satisfies int32;
/// @type.node source="x satisfies int32" type=int32
/// @type.node source=x type=int32
/// @resolution.name source=x target=x

y satisfies string;
/// @type.node source="y satisfies string" type=string
/// @type.node source=y type=string
/// @resolution.name source=y target=y
"#,
    );
}

#[test]
fn test_object_pattern_rejects_primitive_value() {
    let session = TestSession::single(
        r#"
let { value } = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let { value } = 1;

=== checked ===
let { value } = 1;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source="{ value }" kind=object fields=[value]
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC423 message="type '1' cannot be destructured as an object pattern"
/// @diagnostic.label line=2 column=5 source="{ value }"
"#,
    );
}
