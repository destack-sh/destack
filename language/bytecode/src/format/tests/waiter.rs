use super::assert_format_eq;

/// Format runtime waiter operations canonically.
#[test]
fn test_format_waiters() {
    assert_format_eq(
        r#"
function settle(r0:t0,r1:t1):t2 {
waiter.queue r0,r1,t1
return
}
function cancel(r0:t0):t1 {
waiter.cancel r0
return
}
"#,
        r#"
function settle(r0: t0, r1: t1): t2 {
    waiter.queue r0, r1, t1
    return
}

function cancel(r0: t0): t1 {
    waiter.cancel r0
    return
}
"#,
    );
}
