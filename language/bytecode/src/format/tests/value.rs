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
constant.type r7, t0
select r8:r9, r1,r2:r3,r2:r3
constant.null r10
constant.undefined r11
return r5:r11
}
"#,
        r#"
function f0 {
    move r4, r0
    move r0, r1
    select r5, r1, r4, r4
    equal r6, r4, r5
    constant.type r7, t0
    select r8:r9, r1, r2:r3, r2:r3
    constant.null r10
    constant.undefined r11
    return r5:r11
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
    constant.zeroed r0
return r0
}
"#,
        r#"
function f0 {
    constant.zeroed r0
    return r0
}
"#,
    );
}
