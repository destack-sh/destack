use super::assert_format_eq;

/// Format register value operations with canonical operation names.
#[test]
fn test_format_value_operations() {
    assert_format_eq(
        r#"
type User

export function values(r0:int32,r1:boolean,r2:int128):(int32,boolean,typeId,int128,ref<managed,space(local)>,ref<managed,space(local)>){
r4:int32=move r0
r5:int32=select r1,r0,r4
r6:boolean=equal r0,r5
r7:typeId=constant.type User
r8:int128=select r1,r2,r2
r10:ref<managed,space(local)>=null
r11:ref<managed,space(local)>=undefined
return r5,r6,r7,r8,r10,r11
}
"#,
        r#"
type User

export function values(r0: int32, r1: boolean, r2: int128): (
    int32,
    boolean,
    typeId,
    int128,
    ref<managed, space(local)>,
    ref<managed, space(local)>
) {
    r4: int32 = move r0
    r5: int32 = select r1, r0, r4
    r6: boolean = equal r0, r5
    r7: typeId = constant.type User
    r8: int128 = select r1, r2, r2
    r10: ref<managed, space(local)> = null
    r11: ref<managed, space(local)> = undefined
    return r5, r6, r7, r8, r10, r11
}
"#,
    );
}

/// Format explicit storage initialization canonically.
#[test]
fn test_format_storage_values() {
    assert_format_eq(
        r#"
export function storage():(uninit<ref<managed,space(local)>>,uninit<ref<managed,space(local)>>){
r0:uninit<ref<managed,space(local)>>=uninit
r1:uninit<ref<managed,space(local)>>=zeroed
return r0,r1
}
"#,
        r#"
export function storage(): (
    uninit<ref<managed, space(local)>>,
    uninit<ref<managed, space(local)>>
) {
    r0: uninit<ref<managed, space(local)>> = uninit
    r1: uninit<ref<managed, space(local)>> = zeroed
    return r0, r1
}
"#,
    );
}
