use super::assert_format_eq;

/// Format register value operations with canonical operation names.
#[test]
fn test_format_value_operations() {
    assert_format_eq(
        r#"
function f0 {
    move r4, r0
move r0, r1
select r5, r1,r4,r4
equal r6, r4,r5
constant r7, t0: typeId
select r8:r9, r1,r2:r3,r2:r3
constant r10:r11, null
constant r12:r13, undefined
equal.bytes r14,r8:r9,r12:r13,16
return r5:r14
}
"#,
        r#"
function f0 {
    move r4, r0
    move r0, r1
    select r5, r1, r4, r4
    equal r6, r4, r5
    constant r7, t0: typeId
    select r8:r9, r1, r2:r3, r2:r3
    constant r10:r11, null
    constant r12:r13, undefined
    equal.bytes r14, r8:r9, r12:r13, 16
    return r5:r14
}
"#,
    );
}

/// Format explicit zero initialization canonically.
#[test]
fn test_format_storage_values() {
    assert_format_eq(
        r#"
function f0 {
    constant r0, zeroed
return r0
}
"#,
        r#"
function f0 {
    constant r0, zeroed
    return r0
}
"#,
    );
}
