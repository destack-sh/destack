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

let (count, label) = pair;
/// @type.symbol symbol=count source=count type=int32
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source="(count, label)" kind=tuple fields=[0: count, 1: label]
/// @resolution.pattern source=count kind=binding target=count
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
/// @type.symbol symbol=values source=values type=[int32; 2]

let [first, second] = values;
/// @type.symbol symbol=first source=first type=int32
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source="[first, second]" kind=sequence sequence=fixed_array length=2 fields=[0: first, 1: second]
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=values type=[int32; 2]
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
