use crate::tests::{DirRows, TestSession};

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

#[test]
fn test_tuple_pattern_rejects_object_value() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32; y: int32 };

let (x, y) = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: int32 };

let (x, y) = point;

=== checked ===
declare const point: { x: int32; y: int32 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }

let (x, y) = point;
/// @type.symbol symbol=x source=x type=<error>
/// @type.symbol symbol=y source=y type=<error>
/// @resolution.pattern source="(x, y)" kind=tuple fields=[0: x, 1: y]
/// @resolution.pattern source=x kind=binding target=x
/// @resolution.pattern source=y kind=binding target=y
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC424 message="type '{ x: int32; y: int32 }' cannot be destructured as a tuple pattern"
/// @diagnostic.label line=4 column=5 source="(x, y)"
"#,
    );
}

#[test]
fn test_sequence_pattern_rejects_object_value() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32; y: int32 };

let [x, y] = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: int32 };

let [x, y] = point;

=== checked ===
declare const point: { x: int32; y: int32 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }

let [x, y] = point;
/// @type.symbol symbol=x source=x type=<error>
/// @type.symbol symbol=y source=y type=<error>
/// @resolution.pattern source="[x, y]" kind=sequence fields=[0: x, 1: y]
/// @resolution.pattern source=x kind=binding target=x
/// @resolution.pattern source=y kind=binding target=y
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC425 message="type '{ x: int32; y: int32 }' cannot be destructured as a sequence pattern"
/// @diagnostic.label line=4 column=5 source="[x, y]"
"#,
    );
}
