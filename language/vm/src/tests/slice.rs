use destack_mir::Space;
use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Form one interior slice while preserving stable heap-relative references.
#[test]
fn test_execute_slice_view() {
    let site = TestProgram::slice_allocation(0, 0, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.local.managed.slice.uninit r3:r4, a0, r0
    move r5:r6, r3:r4
    slice.view r7:r8, r5:r6, 8, r1, r2
    return r5:r8
}
"#,
        TestProgram::words().allocations([site]),
    );

    let value = machine.complete(0, &[Word::uint64(4), Word::uint64(1), Word::uint64(2)]);

    assert_eq!(value[2].bits(), value[0].bits() + Word::BYTE_LEN as u64);
    assert_eq!(value[1], Word::uint64(4));
    assert_eq!(value[3], Word::uint64(2));
}
