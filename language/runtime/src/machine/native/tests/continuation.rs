use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::tests::{TestProgram, TestWorker};

use super::super::{Loader, Platform};

/// Continue one native deoptimization through canonical bytecode state.
#[test]
fn test_deoptimize_native_continuation() {
    let program = TestProgram::mir(
        r#"
function* generate(v0: int32): int32 {
entry(v0: int32):
    yield v0 => resumed | completed | unwind

resumed(v1: int32):
    return v1

completed(v2: int32):
    return v2

unwind:
    unwind.resume
}

export function task(v0: int32): int32 {
entry(v0: int32):
    v1: continuation<int32, int32, int32> = continuation.new generate(v0)
    continuation.resume v1(v0) => yielded | returned | unwind

yielded(v2: int32, v3: continuation<int32, int32, int32>):
    continuation.destroy v3
    return v2

returned(v4: int32):
    return v4

unwind:
    unwind.resume
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native continuation program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );

    let value = worker
        .run_entrypoint("task", 37)
        .expect("deoptimized continuation should complete");

    assert_eq!(value.words(), &[program::Word::int32(37)]);
}
