use tspp_program as program;
use tspp_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::tests::{TestProgram, TestWorker};

use super::super::{Loader, Platform};

/// Resolve and execute one platform math import from linked native code.
#[test]
fn test_execute_native_import() {
    let program = TestProgram::mir(
        r#"
export function task(v0: float64): float64 {
entry(v0: float64):
    v1: float64 = intrinsic.math.float.cos(v0)
    return v1
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native import program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );

    let value = worker
        .run_entrypoint("task", 0)
        .expect("native import program should complete");

    assert_eq!(value.words(), &[program::Word::float64(1.0)]);
}
