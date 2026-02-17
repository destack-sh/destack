use super::{
    assert_platform_error_code, assert_platform_error_codes, syscall_get_limit,
    syscall_get_priority, with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{
    ProcessId, ProcessLimit, ProcessLimitResource, ProcessSchedulerConfig, ProcessSchedulerPolicy,
};

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
        assert_platform_error_code(
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

/// Report expected errors for unsupported cgroup and job bindings.
#[cfg(unix)]
#[test]
fn test_process_cgroup_and_job_bindings_report_expected_errors() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;
        let missing_path = "/definitely/missing/destack-process-cgroup";
        let cgroup_resource = ProcessLimitResource(libc::RLIMIT_AS as u32);

        // missing cgroup paths should map to not-found or not-supported errors
        assert_platform_error_codes(
            context.destack_process_cgroup_get_limit(
                context.string_value(missing_path),
                cgroup_resource,
            ),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::NotSupported,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_process_cgroup_join(context.string_value(missing_path)),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::NotSupported,
            ],
        )?;
        assert_platform_error_codes(
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

        // job object bindings should report not-supported on unix
        assert_platform_error_code(
            context.destack_process_job_assign(
                context.string_value("destack-test-job"),
                context.process_id_slice_value(&[pid])?,
            ),
            PlatformErrorCode::NotSupported,
        )?;
        assert_platform_error_code(
            context.destack_process_job_set_limit(
                context.string_value("destack-test-job"),
                cgroup_resource,
                context.unified_value(ProcessLimit { soft: 1, hard: 1 }),
            ),
            PlatformErrorCode::NotSupported,
        )?;
        Ok(())
    });
}

/// Validate scheduler, affinity, yield, and umask process controls.
#[cfg(unix)]
#[test]
fn test_process_scheduler_and_affinity_validation() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;

        // affinity reads should either return values or explicit not-supported
        let affinity = context.destack_process_get_affinity(pid);
        match affinity {
            Ok(affinity) => {
                let affinity = context.cpu_list_from_value(affinity)?;
                assert!(!affinity.is_empty());
            }
            Err(error) => {
                assert_platform_error_code::<Vec<u32>>(
                    Err(error),
                    PlatformErrorCode::NotSupported,
                )?;
            }
        }

        assert_platform_error_codes(
            context.destack_process_set_affinity(pid, context.cpu_set_value(&[u32::MAX])?),
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        // scheduler reads should either return values or explicit not-supported
        let scheduler = context.destack_process_get_scheduler(pid);
        match scheduler {
            Ok(_) => {}
            Err(error) => {
                assert_platform_error_code::<ProcessSchedulerConfig>(
                    Err(error),
                    PlatformErrorCode::NotSupported,
                )?;
            }
        }

        assert_platform_error_codes(
            context.destack_process_set_scheduler(
                pid,
                context.unified_value(ProcessSchedulerConfig {
                    policy: ProcessSchedulerPolicy::Other,
                    priority: 0,
                    flags: 1,
                }),
            ),
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        context.destack_process_yield_now()?;

        let previous = context.destack_process_umask(0o022)?;
        let restored = context.destack_process_umask(previous)?;
        assert_eq!(restored, 0o022);

        Ok(())
    });
}
