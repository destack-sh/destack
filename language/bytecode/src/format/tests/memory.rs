use super::assert_format_eq;

/// Format memory operations and pointer calculations canonically.
#[test]
fn test_format_memory() {
    assert_format_eq(
        r#"
type Pair

export function copy(r0:pointer,r1:pointer,r2:uint64,r3:int32,r4:words<2>):int32{
slot s0:Pair=r4[2]
frame.store s0,r4
r6:words<2>=frame.load s0
r8:pointer=frame.address s0
store.int32 r8,r3
r9:int32=load.int32 r8
copy.bytes r1->r0,r2
prefetch.read r1
return r9
}
"#,
        r#"
type Pair

export function copy(r0: pointer, r1: pointer, r2: uint64, r3: int32, r4: words<2>): int32 {
    slot s0: Pair = r4[2]

    frame.store s0, r4
    r6: words<2> = frame.load s0
    r8: pointer = frame.address s0
    store.int32 r8, r3
    r9: int32 = load.int32 r8
    copy.bytes r1 -> r0, r2
    prefetch.read r1
    return r9
}
"#,
    );
}
