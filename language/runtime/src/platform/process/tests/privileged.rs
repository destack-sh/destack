use super::{unique_env_name, with_harness_context, with_native_harness_context};
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::process::{ProcessId, ProcessNamespaceKind, ProcessUnshareFlags};

#[cfg(any(target_os = "linux", target_os = "android"))]
fn net_namespace_link(pid: libc::pid_t) -> std::io::Result<String> {
    let path = format!("/proc/{pid}/ns/net");
    std::fs::read_link(path).map(|path| path.to_string_lossy().to_string())
}

fn is_privileged_test_mode() -> bool {
    let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Reapply current identity values through setter bindings in privileged mode.
#[cfg(unix)]
#[test]
fn test_process_identity_setters_succeed_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_harness_context(|mut context| {
        let current_uid = context.uid()?;
        let current_gid = context.gid()?;
        let current_user_ids = context.user_ids()?;
        let current_group_ids = context.group_ids()?;
        let current_groups = context.groups()?;

        context.set_uid(current_uid)?;
        context.set_euid(current_user_ids.effective)?;
        context.set_gid(current_gid)?;
        context.set_egid(current_group_ids.effective)?;
        context.set_user_ids(current_user_ids)?;
        context.set_group_ids(current_group_ids)?;
        context.set_groups(&current_groups)?;

        // identity reads should remain unchanged after setter calls
        assert_eq!(context.uid()?, current_uid);
        assert_eq!(context.gid()?, current_gid);
        assert_eq!(context.user_ids()?, current_user_ids);
        assert_eq!(context.group_ids()?, current_group_ids);
        assert_eq!(context.groups()?, current_groups);

        Ok(())
    });
}

/// Raise process priority and restore it in privileged mode.
#[cfg(unix)]
#[test]
fn test_process_set_priority_negative_succeeds_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_harness_context(|mut context| {
        let pid = context.pid()?;
        let original_priority = context.get_priority(pid)?;
        let target_priority = if original_priority <= -1 {
            original_priority
        } else {
            -1
        };

        context.set_priority(pid, target_priority)?;
        let lowered_priority = context.get_priority(pid)?;
        // observed priority should be at or above the requested level
        assert!(lowered_priority <= target_priority);
        context.set_priority(pid, original_priority)?;

        Ok(())
    });
}

/// Chroot a forked child into a temporary directory in privileged mode.
#[cfg(unix)]
#[test]
fn test_process_chroot_succeeds_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_native_harness_context(|mut context| {
        let root_path = std::env::temp_dir().join(unique_env_name("CHROOT_ROOT"));
        std::fs::create_dir_all(&root_path).expect("chroot root directory should be created");
        let root_path_string = root_path.to_string_lossy().to_string();

        let child = unsafe { libc::fork() };
        if child == 0 {
            if context.chroot(&root_path_string).is_err() {
                unsafe {
                    libc::_exit(111);
                }
            }
            if context.chdir("/").is_err() {
                unsafe {
                    libc::_exit(112);
                }
            }
            match context.cwd() {
                Ok(cwd) if cwd == "/" => unsafe {
                    libc::_exit(0);
                },
                _ => unsafe {
                    libc::_exit(113);
                },
            }
        }

        // parent should observe a clean child exit after chroot and chdir
        assert!(child > 0);
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        let _ = std::fs::remove_dir_all(&root_path);
        assert_eq!(waited, child);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);

        Ok(())
    });
}

/// Switch into the current network namespace path in privileged mode.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_process_set_network_namespace_succeeds_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_native_harness_context(|mut context| {
        let child = unsafe { libc::fork() };
        if child == 0 {
            let target = unsafe { libc::fork() };
            if target == 0 {
                let unshare_result =
                    context.unshare(ProcessUnshareFlags(libc::CLONE_NEWNET as u64));
                if unshare_result.is_err() {
                    unsafe {
                        libc::_exit(121);
                    }
                }
                unsafe {
                    libc::sleep(2);
                    libc::_exit(0);
                }
            }
            if target < 0 {
                unsafe {
                    libc::_exit(122);
                }
            }

            let target_namespace = match net_namespace_link(target) {
                Ok(namespace) => namespace,
                Err(_) => unsafe {
                    libc::_exit(123);
                },
            };
            let original_namespace = match net_namespace_link(unsafe { libc::getpid() }) {
                Ok(namespace) => namespace,
                Err(_) => unsafe {
                    libc::_exit(124);
                },
            };
            if target_namespace == original_namespace {
                unsafe {
                    libc::_exit(125);
                }
            }

            let target_path = format!("/proc/{target}/ns/net");
            if context.set_network_namespace(&target_path).is_err() {
                unsafe {
                    libc::_exit(126);
                }
            }

            let switched_namespace = match net_namespace_link(unsafe { libc::getpid() }) {
                Ok(namespace) => namespace,
                Err(_) => unsafe {
                    libc::_exit(127);
                },
            };
            if switched_namespace != target_namespace {
                unsafe {
                    libc::_exit(128);
                }
            }

            let mut target_status: libc::c_int = 0;
            let _ = unsafe { libc::waitpid(target, &mut target_status, 0) };
            unsafe {
                libc::_exit(0);
            }
        }

        // parent should observe a clean child exit after namespace transition checks
        assert!(child > 0);
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        assert_eq!(waited, child);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);

        Ok(())
    });
}

/// Enter the current process network namespace through setns in privileged mode.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_process_setns_self_network_namespace_succeeds_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_native_harness_context(|mut context| {
        let child = unsafe { libc::fork() };
        if child == 0 {
            let target = unsafe { libc::fork() };
            if target == 0 {
                let unshare_result =
                    context.unshare(ProcessUnshareFlags(libc::CLONE_NEWNET as u64));
                if unshare_result.is_err() {
                    unsafe {
                        libc::_exit(131);
                    }
                }
                unsafe {
                    libc::sleep(2);
                    libc::_exit(0);
                }
            }
            if target < 0 {
                unsafe {
                    libc::_exit(132);
                }
            }

            let target_namespace = match net_namespace_link(target) {
                Ok(namespace) => namespace,
                Err(_) => unsafe {
                    libc::_exit(133);
                },
            };
            let original_namespace = match net_namespace_link(unsafe { libc::getpid() }) {
                Ok(namespace) => namespace,
                Err(_) => unsafe {
                    libc::_exit(134);
                },
            };
            if target_namespace == original_namespace {
                unsafe {
                    libc::_exit(135);
                }
            }

            if context
                .setns(ProcessId(target as u32), ProcessNamespaceKind::Network)
                .is_err()
            {
                unsafe {
                    libc::_exit(136);
                }
            }

            let switched_namespace = match net_namespace_link(unsafe { libc::getpid() }) {
                Ok(namespace) => namespace,
                Err(_) => unsafe {
                    libc::_exit(137);
                },
            };
            if switched_namespace != target_namespace {
                unsafe {
                    libc::_exit(138);
                }
            }

            let mut target_status: libc::c_int = 0;
            let _ = unsafe { libc::waitpid(target, &mut target_status, 0) };
            unsafe {
                libc::_exit(0);
            }
        }

        // parent should observe a clean child exit after setns transition checks
        assert!(child > 0);
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        assert_eq!(waited, child);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);

        Ok(())
    });
}
