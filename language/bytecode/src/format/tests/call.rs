use super::assert_format_eq;

/// Format direct, indirect, dispatched, and unwinding calls with exact value types.
#[test]
fn test_format_calls() {
    assert_format_eq(
        r#"
type Constraint
type Concrete
external function add(int32,int32):int32
function closureBody(environment r0:ref<managed,space(local)>,r1:int32,r2:int32):int32{
r3:int32=int.add r1,r2
r4:ref<managed,space(local)>=function.environment.current
return r3
}
export function calls(r0:int32,r1:int32,r2:function,r4:ref<managed,space(local)>):int32{
r5:dynamic<Constraint,space(local)>=dynamic.bind r4:Concrete
r7:int32=call add(r0,r1)
r7:int32=call.indirect r2(r0,r1)
r7:int32=call.virtual r4,dispatch 0,slot 0(r0,r1)
r7:int32=call.dynamic r5,slot 0(r0,r1)
r8:typeId=dynamic.type r5
r9:ref<managed,space(local)>=dynamic.payload r5
r10:function=function.bind closureBody,r9
r12:ref<managed,space(local)>=function.environment r10
r7:int32=invoke.indirect r10(r0,r1)=>l0|l1
l0:return r7
l1:unwind.resume
}
"#,
        r#"
type Constraint

type Concrete

external function add(int32, int32): int32

function closureBody(environment r0: ref<managed, space(local)>, r1: int32, r2: int32): int32 {
    r3: int32 = int.add r1, r2
    r4: ref<managed, space(local)> = function.environment.current
    return r3
}

export function calls(r0: int32, r1: int32, r2: function, r4: ref<managed, space(local)>): int32 {
    r5: dynamic<Constraint, space(local)> = dynamic.bind r4: Concrete
    r7: int32 = call add(r0, r1)
    r7: int32 = call.indirect r2(r0, r1)
    r7: int32 = call.virtual r4, dispatch 0, slot 0(r0, r1)
    r7: int32 = call.dynamic r5, slot 0(r0, r1)
    r8: typeId = dynamic.type r5
    r9: ref<managed, space(local)> = dynamic.payload r5
    r10: function = function.bind closureBody, r9
    r12: ref<managed, space(local)> = function.environment r10
    r7: int32 = invoke.indirect r10(r0, r1) => l0 | l1

l0:
    return r7

l1:
    unwind.resume
}
"#,
    );
}
