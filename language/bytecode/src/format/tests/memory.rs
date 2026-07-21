use super::assert_format_eq;

/// Format memory operations and address calculations canonically.
#[test]
fn test_format_memory() {
    assert_format_eq(
        r#"
type Pair

export function copy(r0:address,r1:address,r2:uint64,r3:int32):int32{
slot s0:Pair
r4:address=frame.address s0
store.int32 r4,r3
r5:int32=load.int32 r4
copyBytes r1->r0,r2
prefetchRead r1
return r5
}
"#,
        r#"
type Pair

export function copy(r0: address, r1: address, r2: uint64, r3: int32): int32 {
    slot s0: Pair

    r4: address = frame.address s0
    store.int32 r4, r3
    r5: int32 = load.int32 r4
    copyBytes r1 -> r0, r2
    prefetchRead r1
    return r5
}
"#,
    );
}
