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
/// @resolution.pattern source={ x, y } kind=object fields={ x, y }
/// @type.symbol symbol=x#2 source=x type=int32
/// @type.symbol symbol=y#2 source=y type=string
/// @type.node source=point type={ x: int32; y: string }
/// @resolution.name source=point target=point

x satisfies int32;
/// @type.node source="x satisfies int32" type=int32
/// @type.node source=x type=int32
/// @resolution.name source=x target=x#2

y satisfies string;
/// @type.node source="y satisfies string" type=string
/// @type.node source=y type=string
/// @resolution.name source=y target=y#2
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
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=pattern-source-not-object-shaped message="type '1' cannot be destructured as an object pattern"
/// @diagnostic.label line=2 column=5 span="{ value }" line_source="let { value } = 1;"
"#,
    );
}
