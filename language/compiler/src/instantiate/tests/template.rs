use crate::tests::TestSession;

/// Instantiate one module's own template at the argument its call closes.
#[test]
fn test_instantiate_an_own_template_at_a_closed_call() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

function main(): int32 {
    return identity(1);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.ds",
        r#"
function test.main.main(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call test.main.identity<int32>(v0): (int32) => int32
    return v1
}

function test.main.identity<T>(v0: T): T;

shared function test.main.identity<int32>(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    return v1
}
"#,
    );
}

/// Instantiate an imported template by copying its body out of its own module.
#[test]
fn test_instantiate_an_imported_template_across_modules() {
    let session = TestSession::builder()
        .module(
            "identity.ds",
            r#"
export function identity<T>(value: T): T {
    return value;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { identity } from "./identity";

function main(): int32 {
    return identity(1);
}
"#,
        )
        .build();

    session.assert_mir_elaborated(
        "main.ds",
        r#"
function test.main.main(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call test.identity.identity<int32>(v0): (int32) => int32
    return v1
}

external function test.identity.identity<T>(T): T

shared function test.identity.identity<int32>(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    return v1
}
"#,
    );
}
