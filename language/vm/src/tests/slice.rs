use destack_mir::Space;
use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Form one interior slice while preserving stable heap-relative references.
#[test]
fn test_execute_slice_view() {
    let site = TestMachine::slice_allocation(0, 0, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
type Element

export function view(
    r0: uint64,
    r1: uint64,
    r2: uint64,
): (slice<Element, managed, space(local)>, slice<Element, managed, space(local)>) {
    r3: uninit<slice<Element, managed, space(local)>> = new.local.managed.slice.uninit Element, r0
    r5: slice<Element, managed, space(local)> = new.complete r3
    r7: slice<Element, managed, space(local)> = slice.view r5, r1, r2
    return r5, r7
}
"#,
        TestProgram::new().allocations([site]),
    );

    let value = machine.complete("view", &[Word::uint64(4), Word::uint64(1), Word::uint64(2)]);

    assert_eq!(value[2].bits(), value[0].bits() + Word::BYTE_LEN as u64);
    assert_eq!(value[1], Word::uint64(4));
    assert_eq!(value[3], Word::uint64(2));
}
