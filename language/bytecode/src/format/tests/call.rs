use super::assert_format_eq;

/// Format direct, indirect, dispatched, and unwinding calls with exact value types.
#[test]
fn test_format_calls() {
    assert_format_eq(
        r#"
function f0

function f1 {
    int.add.int32 r3, r1,r2
move r4, r0
return r3
}
function f2 {
    dynamic.bind r5:r6, r4,d0
call r7, f0,r0:r1
call.indirect r7, r2:r3,r0:r1
call.virtual r7, r4,ref<managed,space(local)>,0,0,r0:r1
call.dynamic r7, r5:r6,0,r0:r1
dynamic.type r8, r5:r6
extract r9, r5:r6,0,8
function.bind r10:r11, f1,r9
extract r12, r10:r11,8,8
invoke.indirect r7, r10:r11,r0:r1=>b0|b1
b0:return r7
b1:unwind.resume
}
"#,
        r#"
function f0

function f1 {
    int.add.int32 r3, r1, r2
    move r4, r0
    return r3
}

function f2 {
    dynamic.bind r5:r6, r4, d0
    call r7, f0, r0:r1
    call.indirect r7, r2:r3, r0:r1
    call.virtual r7, r4, ref<managed, space(local)>, 0, 0, r0:r1
    call.dynamic r7, r5:r6, 0, r0:r1
    dynamic.type r8, r5:r6
    extract r9, r5:r6, 0, 8
    function.bind r10:r11, f1, r9
    extract r12, r10:r11, 8, 8
    invoke.indirect r7, r10:r11, r0:r1 => b0 | b1

b0:
    return r7

b1:
    unwind.resume
}
"#,
    );
}
