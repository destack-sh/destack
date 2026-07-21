use super::assert_format_eq;

/// Format multiline function signatures and logical frame slots.
#[test]
fn test_format_function() {
    assert_format_eq(
        r#"
type Pair
export function update(r0:int32,r1:uint64):int32{
slot s0:Pair
r2:address=frame.address s0
r3:int32=move r0
return r3
}
"#,
        r#"
type Pair

export function update(r0: int32, r1: uint64): int32 {
    slot s0: Pair

    r2: address = frame.address s0
    r3: int32 = move r0
    return r3
}
"#,
    );
}
