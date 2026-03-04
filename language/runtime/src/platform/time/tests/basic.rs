use super::{
    assert_platform_error_codes, clock_metadata_from_value, result_or_skip_not_supported,
    with_harness_context,
};

use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::time::{ClockId, ClockMetadata, SleepClock};

/// Sample wall and monotonic clocks and verify nondecreasing monotonic behavior.
#[cfg(any(unix, windows))]
#[test]
fn test_time_wall_and_mono_samples() {
    with_harness_context(|mut context| {
        let wall_first = context.destack_time_wall_ns()?;
        let mono_first = context.destack_time_mono_ns()?;
        let mono_second = context.destack_time_mono_ns()?;

        // monotonic samples should not move backwards
        assert!(mono_second >= mono_first);

        let now_wall = context.destack_time_now_ns(ClockId::Wall)?;
        let now_mono = context.destack_time_now_ns(ClockId::Monotonic)?;

        // sampled clock aliases should stay consistent
        assert!(now_wall >= wall_first);
        assert!(now_mono >= mono_first);

        Ok(())
    });
}

/// Query clock metadata for wall and monotonic clocks.
#[cfg(any(unix, windows))]
#[test]
fn test_time_clock_metadata_for_wall_and_monotonic() {
    with_harness_context(|mut context| {
        let wall_info = context.destack_time_clock_metadata(ClockId::Wall)?;
        let wall_info = clock_metadata_from_value(wall_info);
        assert_eq!(wall_info.id, ClockId::Wall);
        assert!(wall_info.resolution_ns > 0);
        assert!(!wall_info.is_monotonic);

        let mono_info = context.destack_time_clock_metadata(ClockId::Monotonic)?;
        let mono_info = clock_metadata_from_value(mono_info);
        assert_eq!(mono_info.id, ClockId::Monotonic);
        assert!(mono_info.resolution_ns > 0);
        assert!(mono_info.is_monotonic);

        Ok(())
    });
}

/// Exercise process and thread CPU clocks.
#[cfg(any(unix, windows))]
#[test]
fn test_time_cpu_clocks() {
    with_harness_context(|mut context| {
        let _ = result_or_skip_not_supported(context.destack_time_process_cpu_ns())?;

        let _ = result_or_skip_not_supported(context.destack_time_thread_cpu_ns())?;

        Ok(())
    });
}

/// Sample extended clocks and verify notSupported behavior on unsupported hosts.
#[cfg(any(unix, windows))]
#[test]
fn test_time_extended_clock_samples() {
    with_harness_context(|mut context| {
        let _ = result_or_skip_not_supported(context.destack_time_now_ns(ClockId::Boot))?;
        let _ = result_or_skip_not_supported(context.destack_time_now_ns(ClockId::MonotonicRaw))?;

        Ok(())
    });
}

/// Exercise sleep calls across wall and monotonic domains.
#[cfg(any(unix, windows))]
#[test]
fn test_time_sleep_calls() {
    with_harness_context(|mut context| {
        context.destack_time_sleep_ns(0)?;
        context.destack_time_sleep_on_ns(0, SleepClock::Wall)?;
        context.destack_time_sleep_on_ns(0, SleepClock::Monotonic)?;

        let wall_deadline = context.destack_time_wall_ns()?;
        context.destack_time_sleep_until_ns(wall_deadline)?;
        context.destack_time_sleep_until_on_ns(wall_deadline, SleepClock::Wall)?;

        let mono_deadline = context.destack_time_mono_ns()?;
        context.destack_time_sleep_until_on_ns(mono_deadline, SleepClock::Monotonic)?;

        Ok(())
    });
}

/// Query extended clock metadata and verify notSupported behavior on unsupported hosts.
#[cfg(any(unix, windows))]
#[test]
fn test_time_extended_clock_metadata() {
    with_harness_context(|mut context| {
        let boot_result = context.destack_time_clock_metadata(ClockId::Boot);
        if let Err(error) = boot_result {
            assert_platform_error_codes::<ClockMetadata>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::IoInvalidData,
                ],
            )?;
        }

        let raw_result = context.destack_time_clock_metadata(ClockId::MonotonicRaw);
        if let Err(error) = raw_result {
            assert_platform_error_codes::<ClockMetadata>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::IoInvalidData,
                ],
            )?;
        }

        Ok(())
    });
}
