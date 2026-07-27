use super::assert_format_eq;

/// Format generator and asynchronous continuation ownership canonically.
#[test]
fn test_format_continuations() {
    assert_format_eq(
        r#"
function generator {
    yield r1,r2,r0=>b0|b1|b2
b0:return r1
b1:return r1
b2:unwind.resume
}
function resume {
    continuation.new r2,generator,r0
continuation.resume r3:r4,r5,r6:r7,r2,r1=>b0|b1|b2
b0:return r3:r4
b1:return r6:r7
b2:unwind.resume
}
function complete {
    continuation.new r2,generator,r0
continuation.complete r3:r4,r5,r6:r7,r2,r1=>b0|b1|b2
b0:continuation.destroy r5
return r3:r4
b1:return r6:r7
b2:unwind.resume
}
"#,
        r#"
function generator {
    yield r1, r2, r0 => b0 | b1 | b2

b0:
    return r1

b1:
    return r1

b2:
    unwind.resume
}

function resume {
    continuation.new r2, generator, r0
    continuation.resume r3:r4, r5, r6:r7, r2, r1 => b0 | b1 | b2

b0:
    return r3:r4

b1:
    return r6:r7

b2:
    unwind.resume
}

function complete {
    continuation.new r2, generator, r0
    continuation.complete r3:r4, r5, r6:r7, r2, r1 => b0 | b1 | b2

b0:
    continuation.destroy r5
    return r3:r4

b1:
    return r6:r7

b2:
    unwind.resume
}
"#,
    );
}
