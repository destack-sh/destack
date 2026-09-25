use crate::tests::{DirRows, TestSession};

#[test]
fn test_postfix_increment_records_place() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
const before = value++;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 1;
const before: int32 = value++;

=== dir ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

const before = value++;
/// @type.symbol symbol=before source=before type=int32
/// @resolution.pattern source=before kind=binding target=before
/// @type.node source=value type=int32
/// @type.node source=value++ type=int32
/// @resolution.name source=value target=value
/// @resolution.assignment source=value read=binding(value) write=binding(value) type=int32
/// @resolution.access source=value root=value
/// @resolution.operator source=value++ type=int32 operator="++" kind=builtin operands=[value as int32 families=(integer)]
"#,
    );
}

#[test]
fn test_prefix_decrement_records_place() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
const after = --value;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 1;
const after: int32 = --value;

=== dir ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

const after = --value;
/// @type.symbol symbol=after source=after type=int32
/// @resolution.pattern source=after kind=binding target=after
/// @type.node source=--value type=int32
/// @resolution.operator source=--value type=int32 operator="--" kind=builtin operands=[value as int32 families=(integer)]
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.assignment source=value read=binding(value) write=binding(value) type=int32
/// @resolution.access source=value root=value
"#,
    );
}
