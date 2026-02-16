use super::{syscall_get_limit, syscall_get_priority, with_harness_context};
use crate::platform::process::{ProcessId, ProcessLimitResource};

#[cfg(unix)]
#[test]
fn test_process_limits_roundtrip() {
    let resource = ProcessLimitResource(libc::RLIMIT_NOFILE as u32);
    let expected = syscall_get_limit(resource).expect("getrlimit should succeed");

    with_harness_context(|mut context| {
        let actual = context.get_limit(resource)?;
        assert_eq!(actual, expected);

        context.set_limit(resource, actual)?;
        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_process_priority_roundtrip() {
    let pid = ProcessId(std::process::id());
    let expected = syscall_get_priority(pid).expect("getpriority syscall should succeed");

    with_harness_context(|mut context| {
        let actual = context.get_priority(pid)?;
        assert_eq!(actual, expected);

        context.set_priority(pid, actual)?;
        Ok(())
    });
}
