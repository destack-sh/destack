use super::assert_format_eq;

/// Format scalar operations and multi-result assignments canonically.
#[test]
fn test_format_scalar_operations() {
    assert_format_eq(
        r#"
export function add(r0:int32,r1:int32):(int32,boolean){
r2:int32,r3:boolean=int.add.overflowing r0,r1
return r2,r3
}
"#,
        r#"
export function add(r0: int32, r1: int32): (int32, boolean) {
    r2: int32, r3: boolean = int.add.overflowing r0, r1
    return r2, r3
}
"#,
    );
}

/// Format scalar and address conversions with one canonical cast form.
#[test]
fn test_format_casts() {
    assert_format_eq(
        r#"
export function convert(r0:int64,r1:float64,r2:address,r3:uint64):(int32,uint64,address,uint64,float32){
r4:int32=cast.truncate r0->int32
r5:uint64=cast.floatToInt.u r1->uint64
r6:address=cast.intToAddress r3->address
r7:uint64=cast.addressToInt r2->uint64
r8:float32=cast.floatTruncate r1->float32
return r4,r5,r6,r7,r8
}
"#,
        r#"
export function convert(r0: int64, r1: float64, r2: address, r3: uint64): (
    int32,
    uint64,
    address,
    uint64,
    float32
) {
    r4: int32 = cast.truncate r0 -> int32
    r5: uint64 = cast.floatToInt.u r1 -> uint64
    r6: address = cast.intToAddress r3 -> address
    r7: uint64 = cast.addressToInt r2 -> uint64
    r8: float32 = cast.floatTruncate r1 -> float32
    return r4, r5, r6, r7, r8
}
"#,
    );
}

/// Preserve signed and unsigned 128-bit literal interpretation.
#[test]
fn test_format_wide_literals() {
    assert_format_eq(
        r#"
export function literals():(int128,uint128){
r0:int128=-1
r2:uint128=340282366920938463463374607431768211455
return r0,r2
}
"#,
        r#"
export function literals(): (int128, uint128) {
    r0: int128 = -1
    r2: uint128 = 340282366920938463463374607431768211455
    return r0, r2
}
"#,
    );
}

/// Preserve non-finite floating-point values and exact NaN payloads.
#[test]
fn test_format_non_finite_literals() {
    assert_format_eq(
        r#"
export function literals():(float16,float32,float64){
r0:float16=bits(0x7e01)
r1:float32=Infinity
r2:float64=-Infinity
return r0,r1,r2
}
"#,
        r#"
export function literals(): (float16, float32, float64) {
    r0: float16 = bits(0x7e01)
    r1: float32 = Infinity
    r2: float64 = -Infinity
    return r0, r1, r2
}
"#,
    );
}
