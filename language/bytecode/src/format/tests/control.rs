use super::assert_format_eq;

/// Format control flow with canonical instruction labels.
#[test]
fn test_format_control_flow() {
    assert_format_eq(
        r#"
function f0(): t0 {
    branch r0,b0,b1
b0:constant.boolean r1,true
return r1
b1:constant.boolean r1,false
return r1
}
"#,
        r#"
function f0(): t0 {
    branch r0, b0, b1

b0:
    constant.boolean r1, true
    return r1

b1:
    constant.boolean r1, false
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
function f0(): t0 {
    check.nonzero.int32 r0 else b3
check.type r2, t0 else b3
check.null r3 else b3
branch.lt.int32 r0,r1=>b0,b2
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
function f0(): t0 {
    check.nonzero.int32 r0 else b3
    check.type r2, t0 else b3
    check.null r3 else b3
    branch.lt.int32 r0, r1 => b0, b2

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
function f0(): t0 {
    panic r0, t0
}
"#,
        r#"
function f0(): t0 {
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
function f0(): t0 {
    await r2:r3,f1,r0=>b0|b2|b3
b0:
yield r4:r5,r2:r3=>b1|b3
b1:
return r4:r5
b2:
return
b3:
unwind.resume
}
function f1(): t0 {
    return
}
"#,
        r#"
function f0(): t0 {
    await r2:r3, f1, r0 => b0 | b2 | b3

b0:
    yield r4:r5, r2:r3 => b1 | b3

b1:
    return r4:r5

b2:
    return

b3:
    unwind.resume
}

function f1(): t0 {
    return
}
"#,
    );
}
