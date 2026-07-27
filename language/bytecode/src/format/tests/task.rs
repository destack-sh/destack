use super::assert_format_eq;

/// Format completed, eager, parked, cancelled, and detached tasks canonically.
#[test]
fn test_format_tasks() {
    assert_format_eq(
        r#"
function tasks {
task.resolve r2,r0:r1,t0
task.start r4,r3
task.park r4,r5
task.cancel r4
task.detach r4
return
}
"#,
        r#"
function tasks {
    task.resolve r2, r0:r1, t0
    task.start r4, r3
    task.park r4, r5
    task.cancel r4
    task.detach r4
    return
}
"#,
    );
}
