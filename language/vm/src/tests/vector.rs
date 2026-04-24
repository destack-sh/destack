use crate::Value;
use crate::tests::run_mir_expect;

/// Vector splat and extract return the selected lane value.
#[test]
fn test_vector_splat_extract() {
    let mir = r#"
function splatExtract(v0: int32): int32 {
b0(v0: int32):
    v1: vector<int32, 4> = vector.splat v0
    v2: int32 = 2int32
    v3: int32 = vector.extract v1, v2
    return v3
}"#;
    run_mir_expect(mir, "splatExtract", &[Value::int32(7)], Value::int32(7));
}

/// Vector insert replaces the specified lane.
#[test]
fn test_vector_insert() {
    let mir = r#"
function insertLane(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: vector<int32, 4> = vector.splat v0
    v3: int32 = 1int32
    v4: vector<int32, 4> = vector.insert v2, v3, v1
    v5: int32 = vector.extract v4, v3
    return v5
}"#;
    run_mir_expect(
        mir,
        "insertLane",
        &[Value::int32(1), Value::int32(9)],
        Value::int32(9),
    );
}

/// Vector shuffle and reduce produce the expected reduction result.
#[test]
fn test_vector_shuffle_reduce() {
    let mir = r#"
function shuffleReduce(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: vector<int32, 4> = vector.splat v0
    v3: vector<int32, 4> = vector.splat v1
    v4: vector<int32, 4> = vector.shuffle v2, v3, [0, 1, 4, 5]
    v5: int32 = vector.reduce add, v4
    return v5
}"#;
    run_mir_expect(
        mir,
        "shuffleReduce",
        &[Value::int32(1), Value::int32(2)],
        Value::int32(6),
    );
}

/// Vector compare produces boolean lane results.
#[test]
fn test_vector_compare() {
    let mir = r#"
function compareLanes(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: vector<int32, 4> = vector.splat v0
    v3: vector<int32, 4> = vector.splat v1
    v4: vector<boolean, 4> = vector.compare int.eq, v2, v3
    v5: int32 = 0int32
    v6: boolean = vector.extract v4, v5
    return v6
}"#;
    // verify the lane comparison result
    run_mir_expect(
        mir,
        "compareLanes",
        &[Value::int32(7), Value::int32(7)],
        Value::bool(true),
    );
}

/// Vector convert applies the requested conversion mode.
#[test]
fn test_vector_convert() {
    let mir = r#"
function convertLanes(v0: float64): int32 {
b0(v0: float64):
    v1: vector<float64, 2> = vector.splat v0
    v2: vector<int32, 2> = vector.convert roundTowardZero, v1
    v3: int32 = 0int32
    v4: int32 = vector.extract v2, v3
    return v4
}"#;
    // verify the rounded conversion result
    run_mir_expect(mir, "convertLanes", &[Value::float64(3.9)], Value::int32(3));
}
