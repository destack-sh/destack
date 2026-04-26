use crate::Word;
use crate::tests::run_mir_expect;

/// function.address produces a callable pointer for call.indirect.
#[test]
fn test_function_addr_indirect_call() {
    let mir = r#"
function add(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: (int32) -> int32 = function.address add
    v2: int32 = call.indirect v1(v0): (int32) -> int32
    return v2
}"#;
    run_mir_expect(mir, "caller", &[Word::int32(21)], Word::int32(42));
}

/// Exceptional direct calls branch to the success continuation on return.
#[test]
fn test_exceptional_call_branches_to_normal_target() {
    let mir = r#"
function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    invoke double(v0): (int32) -> int32 -> b1(v1), catch b2
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
b2(v5: ref<void, managed, readonly>):
    v6: int32 = 0int32
    return v6
}"#;

    run_mir_expect(mir, "caller", &[Word::int32(8)], Word::int32(26));
}

/// Exceptional direct calls branch to the exception continuation on throw.
#[test]
fn test_exceptional_call_branches_to_unwind_target() {
    let mir = r#"
type Error { code: int32 }

function thrower(): int32 {
b0:
    v0: int32 = 1int32
    v1: Error = struct Error (v0)
    v2: ref<Error, managed, readonly> = new Error
    store v2, v1
    throw v2
}

function caller(): int32 {
b0:
    invoke thrower(): () -> int32 -> b1, catch b2
b1(v0: int32):
    return v0
b2(v1: ref<Error, managed, readonly>):
    v2: int32 = 9int32
    return v2
}"#;

    run_mir_expect(mir, "caller", &[], Word::int32(9));
}

/// Thrown exceptions skip plain call resumes until one exception continuation handles them.
#[test]
fn test_exceptional_call_unwinds_past_plain_call_resume() {
    let mir = r#"
type Error { code: int32 }

function thrower(): int32 {
b0:
    v0: int32 = 1int32
    v1: Error = struct Error (v0)
    v2: ref<Error, managed, readonly> = new Error
    store v2, v1
    throw v2
}

function middle(): int32 {
b0:
    v0: int32 = call thrower(): () -> int32
    return v0
}

function caller(): int32 {
b0:
    invoke middle(): () -> int32 -> b1, catch b2
b1(v0: int32):
    return v0
b2(v1: ref<Error, managed, readonly>):
    v2: int32 = 13int32
    return v2
}"#;

    run_mir_expect(mir, "caller", &[], Word::int32(13));
}
