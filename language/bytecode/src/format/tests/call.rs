use super::assert_format_eq;

/// Format direct, indirect, dispatched, and unwinding calls with exact value types.
#[test]
fn test_format_calls() {
    assert_format_eq(
        r#"
type Constraint
type Concrete
type Binary=(int32,int32)=>int32
external function add(int32,int32):int32
function closureBody(environment r0:ref<borrowed,space(local)>,r1:int32,r2:int32):int32{
r3:int32=int.add r1,r2
r4:ref<borrowed,space(local)>=function.environment.current
return r3
}
export function calls(r0:int32,r1:int32,r2:function<Binary>,r4:ref<managed,space(local)>):int32{
r5:dynamic<Constraint>=dynamic.bind r4:Concrete
r7:int32=call add(r0,r1)
r7:int32=call.indirect r2(r0,r1)
r7:int32=call.virtual r4,slot 0(r0,r1):Binary
r7:int32=call.dynamic r5,slot 0(r0,r1):Binary
r8:typeId=dynamic.type r5
r9:ref<managed,space(local)>=dynamic.payload r5
r10:function<Binary>=function.bind closureBody,r9
r12:functionPointer<Binary>=function.pointer r10
r13:ref<managed,space(local)>=function.environment r10
r7:int32=invoke.indirect r12(r0,r1)=>l0|l1
l0:return r7
l1:unwind.resume
}
"#,
        r#"
type Constraint

type Concrete

type Binary = (int32, int32) => int32

external function add(int32, int32): int32

function closureBody(environment r0: ref<borrowed, space(local)>, r1: int32, r2: int32): int32 {
    r3: int32 = int.add r1, r2
    r4: ref<borrowed, space(local)> = function.environment.current
    return r3
}

export function calls(
    r0: int32,
    r1: int32,
    r2: function<Binary>,
    r4: ref<managed, space(local)>,
): int32 {
    r5: dynamic<Constraint> = dynamic.bind r4: Concrete
    r7: int32 = call add(r0, r1)
    r7: int32 = call.indirect r2(r0, r1)
    r7: int32 = call.virtual r4, slot 0(r0, r1): Binary
    r7: int32 = call.dynamic r5, slot 0(r0, r1): Binary
    r8: typeId = dynamic.type r5
    r9: ref<managed, space(local)> = dynamic.payload r5
    r10: function<Binary> = function.bind closureBody, r9
    r12: functionPointer<Binary> = function.pointer r10
    r13: ref<managed, space(local)> = function.environment r10
    r7: int32 = invoke.indirect r12(r0, r1) => l0 | l1

l0:
    return r7

l1:
    unwind.resume
}
"#,
    );
}
