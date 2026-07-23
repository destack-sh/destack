use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Zero storage across every register word even when the frame slot is reused.
#[test]
fn test_execute_zeroed_storage() {
    let mut machine = TestMachine::parse(
        r#"
export function zero(
    r0: uninit<ref<managed, space(local)>>,
): uninit<ref<managed, space(local)>> {
    r0: uninit<ref<managed, space(local)>> = zeroed
    return r0
}
"#,
        TestProgram::new(),
    );

    let value = machine.complete("zero", &[Word::from_bits(u64::MAX)]);

    assert_eq!(value, vec![Word::ZERO]);
}
