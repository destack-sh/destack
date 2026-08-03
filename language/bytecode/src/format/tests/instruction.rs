use super::assert_format_eq;

/// Format runtime and profiling instructions canonically.
#[test]
fn test_format_runtime_instructions() {
    assert_format_eq(
        r#"
function f0 {
    breakpoint
profile.increment c3
profile.sample s4,r0
return
}
"#,
        r#"
function f0 {
    breakpoint
    profile.increment c3
    profile.sample s4, r0
    return
}
"#,
    );
}
