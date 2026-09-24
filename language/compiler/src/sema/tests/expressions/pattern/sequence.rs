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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: [int32; 2];

let [first, second] = values;

first satisfies int32;
second satisfies int32;

=== dir ===
declare const values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>
/// @resolution.pattern source=values kind=binding target=values

let [first, second] = values;
/// @resolution.pattern source=[first, second] kind=sequence element=int32 arity=2 fields=(first, second)
/// @generic.instantiation id="index#2<int32, 2, \"frame\" & \"local\">" template=index#2 arguments=(int32, 2, "frame" & "local")
/// @generic.instance id="FixedArray<int32, 2>" template=FixedArray arguments=(int32, 2)
/// @generic.instance id="index#2<int32, 2, \"bound0\" & \"local\">" template=index#2 arguments=(int32, 2, "bound0" & "local")
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=values type=FixedArray<int32, 2>
/// @resolution.name source=values target=values
/// @resolution.access source=values root=values

first satisfies int32;
/// @type.node source="first satisfies int32" type=int32
/// @type.node source=first type=int32
/// @resolution.name source=first target=first
/// @resolution.place source=first placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=first root=first

second satisfies int32;
/// @type.node source="second satisfies int32" type=int32
/// @type.node source=second type=int32
/// @resolution.name source=second target=second
/// @resolution.place source=second placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=second root=second
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: int32 };

let [x, y] = point;

=== dir ===
declare const point: { x: int32; y: int32 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: int32" type=int32
/// @type.symbol symbol=y#1 source="y: int32" type=int32

let [x, y] = point;
/// @resolution.rejected source=[x, y]
/// @type.symbol symbol=x#2 source=x type=<error>
/// @type.symbol symbol=y#2 source=y type=<error>
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=pattern-source-not-sequence-shaped message="type '{ x: int32; y: int32 }' cannot be destructured as a sequence pattern"
/// @diagnostic.label line=4 column=5 span="[x, y]" line_source="let [x, y] = point;"
"#,
    );
}
