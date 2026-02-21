use super::super::AudioClockDomain;
use super::with_harness_context;

#[cfg(any(unix, windows))]
#[test]
fn test_audio_clock_now_returns_monotonic_and_wall_samples() {
    with_harness_context(|mut context| {
        let monotonic_ns = context.destack_audio_clock_now(AudioClockDomain::Monotonic)?;
        let wall_ns = context.destack_audio_clock_now(AudioClockDomain::Wall)?;
        assert!(monotonic_ns > 0, "clock.now monotonic should be non-zero");
        assert!(wall_ns > 0, "clock.now wall should be non-zero");

        Ok(())
    });
}
