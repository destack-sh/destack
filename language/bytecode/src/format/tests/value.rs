use super::assert_format_eq;

/// Format register value operations with canonical operation names.
#[test]
fn test_format_value_operations() {
    assert_format_eq(
        r#"
type User

export function values(r0:int32,r1:boolean,r2:int128):(int32,boolean,typeId,int128){
r4:int32=move r0
r5:int32=select r1,r0,r4
r6:boolean=equal r0,r5
r7:typeId=type.id User
r8:int128=select r1,r2,r2
return r5,r6,r7,r8
}
"#,
        r#"
type User

export function values(r0: int32, r1: boolean, r2: int128): (int32, boolean, typeId, int128) {
    r4: int32 = move r0
    r5: int32 = select r1, r0, r4
    r6: boolean = equal r0, r5
    r7: typeId = type.id User
    r8: int128 = select r1, r2, r2
    return r5, r6, r7, r8
}
"#,
    );
}
