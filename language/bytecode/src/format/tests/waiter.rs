use super::assert_format_eq;

/// Format runtime waiter operations canonically.
#[test]
fn test_format_waiters() {
    assert_format_eq(
        r#"
function settle {
waiter.queue r2,r0,r1,t1
return
}
function cancel {
waiter.cancel r1,r0
return
}
"#,
        r#"
function settle {
    waiter.queue r2, r0, r1, t1
    return
}

function cancel {
    waiter.cancel r1, r0
    return
}
"#,
    );
}
