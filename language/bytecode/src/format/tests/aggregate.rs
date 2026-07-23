use super::assert_format_eq;

/// Format packed aggregate and variant operations canonically.
#[test]
fn test_format_aggregate_operations() {
    assert_format_eq(
        r#"
type Pair

type Choice
export function values(r0:int32,r1:int32,r2:words<2>):int32{
r4:words<2>=aggregate Pair(r0,r1)
r6:int32=field.get r4,Pair,0
r7:words<2>=field.set r4,Pair,1,r1
r9:int32=element.get r7,Pair,0
r10:words<2>=element.set r7,Pair,1,r0
r12:words<2>=variant.new Choice,1,r1
r14:uint32=variant.tag r12,Choice
r15:int32=variant.payload r12,Choice,1
return r15
}
"#,
        r#"
type Pair

type Choice

export function values(r0: int32, r1: int32, r2: words<2>): int32 {
    r4: words<2> = aggregate Pair (r0, r1)
    r6: int32 = field.get r4, Pair, 0
    r7: words<2> = field.set r4, Pair, 1, r1
    r9: int32 = element.get r7, Pair, 0
    r10: words<2> = element.set r7, Pair, 1, r0
    r12: words<2> = variant.new Choice, 1, r1
    r14: uint32 = variant.tag r12, Choice
    r15: int32 = variant.payload r12, Choice, 1
    return r15
}
"#,
    );
}
