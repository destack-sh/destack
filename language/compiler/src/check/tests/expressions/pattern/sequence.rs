use crate::tests::{DirRows, TestSession};

#[test]
fn test_sequence_pattern_binds_fixed_array_elements() {
    let session = TestSession::single(
        r#"
declare const values: [int32; 2];

let [first, second] = values;

first satisfies int32;
second satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: [int32; 2];

let [first, second] = values;

first satisfies int32;
second satisfies int32;

=== checked ===
declare const values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>

let [first, second] = values;
/// @resolution.pattern source=[first, second] kind=sequence element=int32 arity=2 fields=(first, second)
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=values type=FixedArray<int32, 2>
/// @resolution.name source=values target=values

first satisfies int32;
/// @type.node source="first satisfies int32" type=int32
/// @type.node source=first type=int32
/// @resolution.name source=first target=first

second satisfies int32;
/// @type.node source="second satisfies int32" type=int32
/// @type.node source=second type=int32
/// @resolution.name source=second target=second
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
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC425 message="type '{ x: int32; y: int32 }' cannot be destructured as a sequence pattern"
/// @diagnostic.label line=4 column=5 span="[x, y]" line_source="let [x, y] = point;"
"#,
    );
}
