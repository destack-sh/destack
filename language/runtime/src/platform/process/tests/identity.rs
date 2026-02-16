use super::{
    syscall_getpgid, syscall_group_ids, syscall_groups, syscall_user_ids, with_harness_context,
};
use crate::platform::process::{ProcessId, Signal};

#[cfg(unix)]
#[test]
fn test_process_kill_zero_to_self() {
    let pid = ProcessId(std::process::id());

    with_harness_context(|mut context| {
        context.kill(pid, Signal(0))?;
        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_process_identity_extended_syscall_parity() {
    let expected_group_ids = syscall_group_ids().expect("group ids syscall should succeed");
    let expected_groups = syscall_groups().expect("groups syscall should succeed");
    let expected_user_ids = syscall_user_ids().expect("user ids syscall should succeed");

    with_harness_context(|mut context| {
        assert_eq!(context.group_ids()?, expected_group_ids);
        assert_eq!(context.groups()?, expected_groups);
        assert_eq!(context.user_ids()?, expected_user_ids);
        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_process_getpgid_matches_syscall() {
    let pid = ProcessId(std::process::id());
    let expected = syscall_getpgid(pid).expect("getpgid syscall should succeed");

    with_harness_context(|mut context| {
        assert_eq!(context.getpgid(pid)?, expected);
        Ok(())
    });
}
