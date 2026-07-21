use super::assert_format_eq;

/// Format global, frame, offset, index, and distance pointers canonically.
#[test]
fn test_format_pointer_operations() {
    assert_format_eq(
        r#"
type Pair
local global state:Pair=zero

export function pointers(r0:uint64):int64{
slot s0:Pair
r1:pointer=global.address state
r2:pointer=frame.address s0
r3:pointer=pointer.offset r1,16
r4:pointer=pointer.index r2,r0,stride(8)
r5:int64=pointer.distance r4,r3
return r5
}
"#,
        r#"
type Pair

local global state: Pair = zero

export function pointers(r0: uint64): int64 {
    slot s0: Pair

    r1: pointer = global.address state
    r2: pointer = frame.address s0
    r3: pointer = pointer.offset r1, 16
    r4: pointer = pointer.index r2, r0, stride(8)
    r5: int64 = pointer.distance r4, r3
    return r5
}
"#,
    );
}
