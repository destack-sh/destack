use std::sync::atomic::{AtomicUsize, Ordering};

use tspp_program as program;
use tspp_repository::RuntimeOptions;

use crate::binding::{Binding, BindingTable, ReplayPayload};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::tests::{TestProgram, TestWorker};
use crate::worker::Activation;

use super::super::{Loader, Platform};

static CLEANUPS: AtomicUsize = AtomicUsize::new(0);

/// Run MIR cleanup while a native runtime failure unwinds.
#[test]
fn test_unwind_native_invoke() {
    let program = TestProgram::mir(
        r#"
@binding("runtime.fail", { provider: "runtime", effect: "deterministic", replay: "forbidden", affinity: "worker" })
external function fail(): void

@binding("runtime.cleanup", { provider: "runtime", effect: "deterministic", replay: "forbidden", affinity: "worker" })
external function cleanup(): void

export function task(v0: int32): int32 {
entry(v0: int32):
    invoke fail(): () => void => returned | unwind

returned:
    return v0

unwind:
    call cleanup(): () => void
    unwind.resume
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native test program should load");
    let mut worker = TestWorker::native(&RuntimeOptions::default(), program, bindings(), code);
    CLEANUPS.store(0, Ordering::SeqCst);

    let error = worker
        .run_entrypoint("task", 0)
        .expect_err("native failure should cross the engine boundary");

    assert_eq!(CLEANUPS.load(Ordering::SeqCst), 1);
    assert_eq!(
        *error,
        RuntimeError::Internal {
            message: "native unwind tracer".to_string(),
        },
    );
}

/// Return the exact bindings used by the native unwind tracer.
fn bindings() -> BindingTable {
    let mut bindings = BindingTable::new();
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("runtime.fail"),
        ReplayPayload::Results,
        fail,
    ));
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("runtime.cleanup"),
        ReplayPayload::Results,
        cleanup,
    ));

    bindings
}

/// Fail one native runtime invocation.
fn fail(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    _arguments: &[program::Word],
    _results: &mut [program::Word],
) -> RuntimeResult<()> {
    Err(RuntimeError::Internal {
        message: "native unwind tracer".to_string(),
    }
    .boxed())
}

/// Record one native cleanup invocation.
fn cleanup(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    _arguments: &[program::Word],
    _results: &mut [program::Word],
) -> RuntimeResult<()> {
    CLEANUPS.fetch_add(1, Ordering::SeqCst);

    Ok(())
}
