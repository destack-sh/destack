use super::assert_format_eq;

/// Format multiline function bodies and register addresses.
#[test]
fn test_format_function() {
    assert_format_eq(
        r#"
function f0(r0: t1, r1:r2: t2): t3 {
    address r2, r0
move r3, r0
return r3
}
"#,
        r#"
function f0(r0: t1, r1:r2: t2): t3 {
    address r2, r0
    move r3, r0
    return r3
}
"#,
    );
}

/// Format ordinary, async, generator, and async generator functions canonically.
#[test]
fn test_format_function_modifiers() {
    assert_format_eq(
        r#"
function regular(): t0

async function task(): t0

function* generate(): t0

async function* stream(): t0
"#,
        r#"
function regular(): t0

async function task(): t0

function* generate(): t0

async function* stream(): t0
"#,
    );
}
