use super::with_harness_context;

/// Exit a forked child through process bindings and verify the exit code.
#[cfg(unix)]
#[test]
fn test_process_exit_in_child() {
    with_harness_context(|mut context| {
        let child = unsafe { libc::fork() };
        if child == 0 {
            let _ = context.destack_process_exit(23);
            unsafe {
                libc::_exit(111);
            }
        }

        // parent should observe the child exit code set by context.exit
        assert!(child > 0);
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        assert_eq!(waited, child);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 23);

        Ok(())
    });
}
