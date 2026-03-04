#[cfg(target_os = "linux")]
use super::assert_platform_error_codes_with_privileged_policy;
use super::{
    assert_platform_error_code_with_privileged_policy, syscall_get_limit, syscall_get_priority,
    with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{ProcessId, ProcessLimit, ProcessLimitResource};
#[cfg(target_os = "linux")]
use crate::platform::process::{ProcessSchedulerConfig, ProcessSchedulerPolicy};

/// Set and restore one resource limit while preserving original values.
#[cfg(unix)]
#[test]
fn test_process_limits_set_get_restore_roundtrip() {
    let resource = ProcessLimitResource(libc::RLIMIT_NOFILE as u32);
    let expected = syscall_get_limit(resource).expect("getrlimit should succeed");

    with_harness_context(|mut context| {
        // runtime limit reads should match direct syscall observations
        let original = context.destack_process_get_limit(resource)?;
        let original = original.into_inner();
        assert_eq!(original, expected);

        let target_soft = if original.soft > 1 {
            original.soft - 1
        } else {
            original.soft
        };
        let target = ProcessLimit {
            soft: target_soft.min(original.hard),
            hard: original.hard,
        };

        let mut observed_after_set = None;
        let set_result: RuntimeResult<()> = (|| {
            if target != original {
                context.destack_process_set_limit(resource, context.unified_value(target))?;
                let observed = context.destack_process_get_limit(resource)?;
                observed_after_set = Some(observed.into_inner());
            }

            Ok(())
        })();

        context.destack_process_set_limit(resource, context.unified_value(original))?;
        // restored limit should match the original snapshot
        let restored = context.destack_process_get_limit(resource)?;
        let restored = restored.into_inner();
        assert_eq!(restored, original);

        set_result?;
        if let Some(observed_after_set) = observed_after_set {
            assert_eq!(observed_after_set, target);
        }

        Ok(())
    });
}

/// Reject resource limits where soft exceeds hard.
#[cfg(unix)]
#[test]
fn test_process_limits_reject_invalid_soft_greater_than_hard() {
    let resource = ProcessLimitResource(libc::RLIMIT_NOFILE as u32);
    let invalid_limit = ProcessLimit { soft: 2, hard: 1 };

    with_harness_context(|mut context| {
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_set_limit(resource, context.unified_value(invalid_limit)),
            PlatformErrorCode::InvalidArgumentValue,
        )
    });
}

/// Read and write process priority for the current process id.
#[cfg(unix)]
#[test]
fn test_process_priority_get_set_roundtrip() {
    let pid = ProcessId(std::process::id());
    let expected = syscall_get_priority(pid).expect("getpriority syscall should succeed");

    with_harness_context(|mut context| {
        let actual = context.destack_process_get_priority(pid)?;
        assert_eq!(actual, expected);

        context.destack_process_set_priority(pid, actual)?;
        Ok(())
    });
}

/// Report expected errors for missing cgroup paths on Linux.
#[cfg(target_os = "linux")]
#[test]
fn test_process_cgroup_bindings_report_expected_errors() {
    with_harness_context(|mut context| {
        let missing_path = "/definitely/missing/destack-process-cgroup";
        let cgroup_resource = ProcessLimitResource(libc::RLIMIT_AS as u32);

        // missing cgroup paths should map to not-found or not-supported errors
        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_cgroup_get_limit(
                context.string_value(missing_path),
                cgroup_resource,
            ),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::NotSupported,
            ],
        )?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_cgroup_join(context.string_value(missing_path)),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::NotSupported,
            ],
        )?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_cgroup_set_limit(
                context.string_value(missing_path),
                cgroup_resource,
                context.unified_value(ProcessLimit { soft: 1, hard: 1 }),
            ),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        Ok(())
    });
}

/// Validate scheduler, affinity, yield, and umask process controls.
#[cfg(target_os = "linux")]
#[test]
fn test_process_scheduler_and_affinity_validation() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;

        // affinity reads should return at least one cpu
        let affinity = context.destack_process_get_affinity(pid)?;
        let affinity = context.cpu_list_from_value(affinity)?;
        assert!(!affinity.is_empty());

        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_set_affinity(pid, context.cpu_set_value(&[u32::MAX])?),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // scheduler reads should succeed
        let _scheduler = context.destack_process_get_scheduler(pid)?;

        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_set_scheduler(
                pid,
                context.unified_value(ProcessSchedulerConfig {
                    policy: ProcessSchedulerPolicy::Other,
                    priority: 0,
                    flags: 1,
                }),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        context.destack_process_yield_now()?;

        let previous = context.destack_process_umask(0o022)?;
        let restored = context.destack_process_umask(previous)?;
        assert_eq!(restored, 0o022);

        Ok(())
    });
}
