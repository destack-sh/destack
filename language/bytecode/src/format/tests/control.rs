use super::assert_format_eq;

/// Format control flow with canonical instruction labels.
#[test]
fn test_format_control_flow() {
    assert_format_eq(
        r#"
function f0 {
    branch r0 => b0 | b1
b0:constant r1, true: boolean
return r1
b1:constant r1, false: boolean
return r1
}
"#,
        r#"
function f0 {
    branch r0 => b0 | b1

b0:
    constant r1, true: boolean
    return r1

b1:
    constant r1, false: boolean
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
function f0 {
    check.nonzero r0:int32 | b3
check.shift r0,32:int32 | b3
check.narrow r0:int64->int32 | b3
check.add.overflow r0,r1:int32 | b3
check.sub.overflow r0,r1:int32 | b3
check.mul.overflow r0,r1:int32 | b3
check.bounds r0,r1:uint64 | b3
check.range r0,r1,r2:uint64 | b3
poll
check.type r2, t0 | b3
check.nullish r3 | b3
branch.lt r0,r1:int32=>b0 | b2
b0:
switch r0{0=>b1,default=>b2}
b1:
return r0
b2:
return r1
b3:
trap bounds
}
"#,
        r#"
function f0 {
    check.nonzero r0: int32 | b3
    check.shift r0, 32: int32 | b3
    check.narrow r0: int64 -> int32 | b3
    check.add.overflow r0, r1: int32 | b3
    check.sub.overflow r0, r1: int32 | b3
    check.mul.overflow r0, r1: int32 | b3
    check.bounds r0, r1: uint64 | b3
    check.range r0, r1, r2: uint64 | b3
    poll
    check.type r2, t0 | b3
    check.nullish r3 | b3
    branch.lt r0, r1: int32 => b0 | b2

b0:
    switch r0 { 0 => b1, default => b2 }

b1:
    return r0

b2:
    return r1

b3:
    trap bounds
}
"#,
    );
}

/// Format panic values canonically.
#[test]
fn test_format_panic() {
    assert_format_eq(
        r#"
function f0 {
    panic r0, t0
}
"#,
        r#"
function f0 {
    panic r0, t0
}
"#,
    );
}

/// Format await and yield suspension canonically.
#[test]
fn test_format_suspension() {
    assert_format_eq(
        r#"
function f0 {
    await r2:r3,f1,r0=>b0|b2|b3
b0:
yield r4:r5,r6:r7,r2:r3=>b1|b2|b3
b1:
return r4:r5
b2:
return
b3:
unwind.resume
}
function f1 {
    return
}
"#,
        r#"
function f0 {
    await r2:r3, f1, r0 => b0 | b2 | b3

b0:
    yield r4:r5, r6:r7, r2:r3 => b1 | b2 | b3

b1:
    return r4:r5

b2:
    return

b3:
    unwind.resume
}

function f1 {
    return
}
"#,
    );
}
