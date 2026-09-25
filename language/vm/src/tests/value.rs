use tspp_program::Word;

use super::{TestMachine, TestProgram};

/// Zero storage across every register word even when the frame slot is reused.
#[test]
fn test_execute_zeroed_storage() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.zeroed r0
    return r0
}
"#,
        TestProgram::words(),
    );

    let value = machine.complete(0, &[Word::from_bits(u64::MAX)]);

    assert_eq!(value, vec![Word::ZERO]);
}
