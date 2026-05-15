use crate::Value;
use crate::diagnostic::Error;
use crate::tests::{assert_runtime_error_matches, run_mir, run_mir_expect};

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
    run_mir_expect(mir, "caller", &[Value::int32(21)], Value::int32(42));
}

/// Direct call terminators branch to the explicit continuation on return.
#[test]
fn test_call_terminator_branches_to_target() {
    let mir = r#"
function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    call double(v0): (int32) -> int32 -> b1(v1)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(8)], Value::int32(26));
}

/// Imported calls fail at the VM boundary.
#[test]
fn test_imported_call_requires_runtime_boundary() {
    let mir = r#"
external function touch(): void

function caller(): int32 {
b0:
    call touch(): () -> void -> b1
b1:
    v0: int32 = 7int32
    return v0
}"#;
    let result = run_mir(mir, "caller", &[]);

    assert_runtime_error_matches!(
        result,
        Error::BindingCallForbidden { ref name } if name == "touch",
    );
}
