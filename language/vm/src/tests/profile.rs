use tspp_program::{Profile, ProfileOptions, Word};

use super::{TestMachine, TestProgram};

/// Record explicit counters and exact sampled word values through Program sites.
#[test]
fn test_record_profile() {
    let counter = TestProgram::counter(0, 0, 0);
    let sample = TestProgram::sample(0, 1, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    profile.increment c0
    profile.sample s0, r0
    return r0
}
"#,
        TestProgram::words().counters([counter]).samples([sample]),
    );
    let mut profile = Profile::new(machine.program(), ProfileOptions::STANDARD);

    let value = machine.complete_profiled(0, &[Word::uint64(37)], &mut profile);

    assert_eq!(value, vec![Word::uint64(37)]);
    assert_eq!(profile.counters[0].count, 1);
    assert_eq!(profile.samples[0].count, 1);
    assert_eq!(profile.samples[0].buckets[0].key.raw(), 37);
    assert_eq!(profile.samples[0].buckets[0].count, 1);
}
