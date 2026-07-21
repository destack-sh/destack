use super::assert_format_eq;

/// Format global, frame, offset, element, and distance addresses canonically.
#[test]
fn test_format_address_operations() {
    assert_format_eq(
        r#"
type Pair
local global state:Pair=zero

export function addresses(r0:uint64):int64{
slot s0:Pair
r1:address=global.address state
r2:address=frame.address s0
r3:address=address.offset r1,16
r4:address=address.element r2,r0,stride(8)
r5:int64=address.distance r4,r3
return r5
}
"#,
        r#"
type Pair

local global state: Pair = zero

export function addresses(r0: uint64): int64 {
    slot s0: Pair

    r1: address = global.address state
    r2: address = frame.address s0
    r3: address = address.offset r1, 16
    r4: address = address.element r2, r0, stride(8)
    r5: int64 = address.distance r4, r3
    return r5
}
"#,
    );
}
