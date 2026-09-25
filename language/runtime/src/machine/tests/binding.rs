use tspp_program as program;
use tspp_repository::RuntimeOptions;

use crate::binding::{Binding, BindingTable, ReplayPayload};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::family_name;
use crate::machine::native::{Loader, Platform};
use crate::tests::{TestProgram, TestWorker};
use crate::worker::Activation;

/// Execute one Program implementation attached to a binding identity.
#[test]
fn test_execute_binding_definition() {
    let program = TestProgram::mir(
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "pure", affinity: "worker" })
export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}
"#,
    )
    .build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, BindingTable::new());

    let value = worker
        .run_entrypoint("task", 41)
        .expect("program binding implementation should execute");

    assert_eq!(value.words(), &[program::Word::int32(42)]);
}

/// Execute one linked binding through the bytecode machine and worker runtime table.
#[test]
fn test_execute_vm_binding() {
    let program = touch_program().build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, bindings());

    let value = worker
        .run_entrypoint("task", 41)
        .expect("runtime binding should execute");
    assert_eq!(value.words(), &[program::Word::int32(42)]);

    // preserve the exact runtime failure outside language panic and unwind
    let error = worker
        .run_entrypoint("task", 13)
        .expect_err("runtime binding failure should propagate");
    assert_eq!(
        *error,
        RuntimeError::Internal {
            message: "runtime.touch rejected 13".to_string(),
        }
    );
}

/// Execute one binding selected by the active target family.
#[test]
fn test_execute_binding_family() {
    let source = format!(
        r#"
@binding("runtime.touch", {{ provider: "runtime", effect: "pure", affinity: "worker", families: ["{}"], hosts: ["unavailable"] }})
external function touch(int32): int32

export function task(v0: int32): int32 {{
entry(v0: int32):
    v1: int32 = call touch(v0): (int32) => int32
    return v1
}}
"#,
        family_name()
    );
    let program = TestProgram::mir(&source).build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, bindings());

    let value = worker
        .run_entrypoint("task", 41)
        .expect("target family should select runtime binding");

    assert_eq!(value.words(), &[program::Word::int32(42)]);
}

/// Execute one linked binding through native code and the same worker runtime table.
#[test]
fn test_execute_native_binding() {
    let program = touch_program().compile_native().build();
    let [binding] = program.bindings() else {
        panic!("native test should link one runtime binding");
    };
    assert!(binding.is_imported());
    let code = Platform
        .load(&program)
        .expect("native binding program should load");
    let mut worker = TestWorker::native(&RuntimeOptions::default(), program, bindings(), code);

    let value = worker
        .run_entrypoint("task", 41)
        .expect("native runtime binding should execute");
    assert_eq!(value.words(), &[program::Word::int32(42)]);

    // preserve the same runtime failure across the native ABI callback
    let error = worker
        .run_entrypoint("task", 13)
        .expect_err("native runtime binding failure should propagate");
    assert_eq!(
        *error,
        RuntimeError::Internal {
            message: "runtime.touch rejected 13".to_string(),
        }
    );
}

/// Return 42 from one exact int32 argument.
fn touch(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    arguments: &[program::Word],
    result: &mut [program::Word],
) -> RuntimeResult<()> {
    assert_eq!(arguments.len(), 1);
    assert_eq!(result.len(), 1);

    // reject one value to exercise exact runtime error propagation
    if arguments[0] != program::Word::int32(41) {
        return Err(RuntimeError::Internal {
            message: format!("runtime.touch rejected {}", arguments[0].bits()),
        }
        .boxed());
    }
    result[0] = program::Word::int32(42);

    Ok(())
}

/// Build the runtime binding table used by binding integration tests.
fn bindings() -> BindingTable {
    let mut bindings = BindingTable::new();
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("runtime.touch"),
        ReplayPayload::Results,
        touch,
    ));

    bindings
}

/// Return the external runtime binding program used by binding integration tests.
fn touch_program() -> TestProgram {
    TestProgram::mir(
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "pure", affinity: "worker" })
external function touch(int32): int32

export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call touch(v0): (int32) => int32
    return v1
}

"#,
    )
}
