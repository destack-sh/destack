use crate::tests::TestSession;

#[test]
fn test_extend_by_sign_when_widening_an_integer() {
    let session = TestSession::single(
        r#"
function widen(a: int32, b: uint32): int64 {
    return (a as int64) + (b as int64);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.widen(v0: int32, v1: uint32): int64 {
entry(v0: int32, v1: uint32):
    v2: int64 = cast.extend.s v0 -> int64
    v3: int64 = cast.extend.u v1 -> int64
    v4: int64 = int.add v2, v3
    return v4
}
"#,
    );
}

#[test]
fn test_truncate_when_narrowing_an_integer() {
    let session = TestSession::single(
        r#"
function narrow(value: int64): int8 {
    return value as int8;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.narrow(v0: int64): int8 {
entry(v0: int64):
    v1: int8 = cast.truncate v0 -> int8
    return v1
}
"#,
    );
}

#[test]
fn test_convert_an_integer_to_float_by_sign() {
    let session = TestSession::single(
        r#"
function ratio(hits: uint32, total: int32): float64 {
    return (hits as float64) / (total as float64);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.ratio(v0: uint32, v1: int32): float64 {
entry(v0: uint32, v1: int32):
    v2: float64 = cast.intToFloat.u v0 -> float64
    v3: float64 = cast.intToFloat.s v1 -> float64
    v4: float64 = float.div v2, v3
    return v4
}
"#,
    );
}

#[test]
fn test_saturate_when_converting_a_float_to_integer() {
    let session = TestSession::single(
        r#"
function whole(value: float64): int32 {
    return value as int32;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.whole(v0: float64): int32 {
entry(v0: float64):
    v1: int32 = cast.floatToIntSaturating.s v0 -> int32
    return v1
}
"#,
    );
}

#[test]
fn test_materialize_a_literal_cast_at_its_target_type() {
    let session = TestSession::single(
        r#"
function big(): int64 {
    return 1 as int64;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.big(): int64 {
entry:
    v0: int64 = 1
    return v0
}
"#,
    );
}
