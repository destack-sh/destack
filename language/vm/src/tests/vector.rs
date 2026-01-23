use crate::memory::Value;
use crate::tests::run_mir_expect;

/// Vector splat and extract return the selected lane value.
#[test]
fn test_vector_splat_extract() {
    let mir = r#"
function @splat_extract(v0: i32) -> i32 {
block0(v0: i32):
    v1: vector<i32, 4> = vector.splat v0
    v2: i32 = iconst 2i32
    v3: i32 = vector.extract v1, v2
    return v3
}"#;
    run_mir_expect(mir, "splat_extract", &[Value::int32(7)], Value::int32(7));
}

/// Vector insert replaces the specified lane.
#[test]
fn test_vector_insert() {
    let mir = r#"
function @insert_lane(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: vector<i32, 4> = vector.splat v0
    v3: i32 = iconst 1i32
    v4: vector<i32, 4> = vector.insert v2, v3, v1
    v5: i32 = vector.extract v4, v3
    return v5
}"#;
    run_mir_expect(
        mir,
        "insert_lane",
        &[Value::int32(1), Value::int32(9)],
        Value::int32(9),
    );
}

/// Vector shuffle and reduce produce the expected reduction result.
#[test]
fn test_vector_shuffle_reduce() {
    let mir = r#"
function @shuffle_reduce(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: vector<i32, 4> = vector.splat v0
    v3: vector<i32, 4> = vector.splat v1
    v4: vector<i32, 4> = vector.shuffle v2, v3, [0, 1, 4, 5]
    v5: i32 = vector.reduce add, v4
    return v5
}"#;
    run_mir_expect(
        mir,
        "shuffle_reduce",
        &[Value::int32(1), Value::int32(2)],
        Value::int32(6),
    );
}
