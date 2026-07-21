use crate::tests::{DirRows, TestSession};

#[test]
fn test_tuple_pattern_binds_elements() {
    let session = TestSession::single(
        r#"
declare const pair: (int32, string);

let (count, label) = pair;

count satisfies int32;
label satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: (int32, string);

let (count, label) = pair;

count satisfies int32;
label satisfies string;

=== checked ===
declare const pair: (int32, string);
/// @type.symbol symbol=pair source=pair type=(int32, string)
/// @resolution.pattern source=pair kind=binding target=pair

let (count, label) = pair;
/// @resolution.pattern source=(count, label) kind=tuple fields=(count, label)
/// @type.symbol symbol=count source=count type=int32
/// @resolution.pattern source=count kind=binding target=count
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source=pair type=(int32, string)
/// @resolution.name source=pair target=pair

count satisfies int32;
/// @type.node source="count satisfies int32" type=int32
/// @type.node source=count type=int32
/// @resolution.name source=count target=count

label satisfies string;
/// @type.node source="label satisfies string" type=string
/// @type.node source=label type=string
/// @resolution.name source=label target=label
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
/// @resolution.pattern source=point kind=binding target=point

let (x, y) = point;
/// @type.symbol symbol=x#2 source=x type=<error>
/// @type.symbol symbol=y#2 source=y type=<error>
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error id=pattern-source-not-tuple-shaped message="type '{ x: int32; y: int32 }' cannot be destructured as a tuple pattern"
/// @diagnostic.label line=4 column=5 span="(x, y)" line_source="let (x, y) = point;"
"#,
    );
}
