use super::assert_format_eq;

/// Format control flow with canonical instruction labels.
#[test]
fn test_format_control_flow() {
    assert_format_eq(
        r#"
export function choose(r0:boolean):boolean{
branch r0,l0,l1
l0:r1:boolean=true
return r1
l1:r1:boolean=false
return r1
}
"#,
        r#"
export function choose(r0: boolean): boolean {
    branch r0, l0, l1

l0:
    r1: boolean = true
    return r1

l1:
    r1: boolean = false
    return r1
}
"#,
    );
}

/// Format checks, fused branches, switches, and traps canonically.
#[test]
fn test_format_checked_control_flow() {
    assert_format_eq(
        r#"
type User

export function choose(r0:int32,r1:int32,r2:typeId,r3:address):int32{
check.nonzero.int32 r0 else l3
check.type r2:User else l3
check.null r3 else l3
branch.lt.int32 r0,r1=>l0,l2
l0:
switch r0{0=>l1,default=>l2}
l1:
return r0
l2:
return r1
l3:
trap bounds
}
"#,
        r#"
type User

export function choose(r0: int32, r1: int32, r2: typeId, r3: address): int32 {
    check.nonzero.int32 r0 else l3
    check.type r2: User else l3
    check.null r3 else l3
    branch.lt.int32 r0, r1 => l0, l2

l0:
    switch r0 { 0 => l1, default => l2 }

l1:
    return r0

l2:
    return r1

l3:
    trap bounds
}
"#,
    );
}

/// Format continuation edges and panic values canonically.
#[test]
fn test_format_suspension_and_panic() {
    assert_format_eq(
        r#"
export function suspend(r0:int32) resume(int32):int32{
r1:int32=yield r0=>l0|l1
l0:return r1
l1:unwind.resume
}

export function fail():void{
r0:ref<managed,space(local)>=catch
panic r0
}
"#,
        r#"
export function suspend(r0: int32) resume(int32): int32 {
    r1: int32 = yield r0 => l0 | l1

l0:
    return r1

l1:
    unwind.resume
}

export function fail(): void {
    r0: ref<managed, space(local)> = catch
    panic r0
}
"#,
    );
}
