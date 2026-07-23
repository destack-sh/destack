use super::assert_format_eq;

/// Format runtime and profiling instructions canonically.
#[test]
fn test_format_runtime_instructions() {
    assert_format_eq(
        r#"
export function observed(r0:uint64):void{
breakpoint
profile.increment counter(3)
profile.sample sampler(4),r0
return
}
"#,
        r#"
export function observed(r0: uint64): void {
    breakpoint
    profile.increment counter(3)
    profile.sample sampler(4), r0
    return
}
"#,
    );
}
