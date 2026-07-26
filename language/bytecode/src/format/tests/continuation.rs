use super::assert_format_eq;

/// Format generator and asynchronous continuation ownership canonically.
#[test]
fn test_format_continuations() {
    assert_format_eq(
        r#"
function* generator(): t0 {
    yield r1,r0=>b0|b1
b0:return r1
b1:unwind.resume
}
function owner(): t0 {
    continuation.new r2,generator,r0
resume r3:r4,r5,r6:r7,r2,r1=>b0|b1|b2
b0:return r3:r4
b1:return r6:r7
b2:unwind.resume
}
"#,
        r#"
function* generator(): t0 {
    yield r1, r0 => b0 | b1

b0:
    return r1

b1:
    unwind.resume
}

function owner(): t0 {
    continuation.new r2, generator, r0
    resume r3:r4, r5, r6:r7, r2, r1 => b0 | b1 | b2

b0:
    return r3:r4

b1:
    return r6:r7

b2:
    unwind.resume
}
"#,
    );
}
