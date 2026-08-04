use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::tests::{TestProgram, TestWorker};

use super::super::{Loader, Platform};

/// Execute linked tensor descriptors through host-native code.
#[test]
fn test_execute_native_tensor() {
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: tensor<int32, managed, mutable, (2, 2)> = tensor.splat v0
    v4: int32 = tensor.extract v3, [v1, v2]
    return v4
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native tensor program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );

    let value = worker
        .run_entrypoint("task", 37)
        .expect("native tensor program should complete");

    assert_eq!(value.words(), &[program::Word::int32(37)]);
}
