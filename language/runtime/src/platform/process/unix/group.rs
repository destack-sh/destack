#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::PlatformErrorCode;
use crate::platform::abi::{NativeSlice, NativeStringRef};
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::diagnostic::process_error_code_from_errno;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::process::core as core_process;

use crate::runtime::BindingCallContext;

use crate::platform::process::{ProcessId, ProcessLimit, ProcessLimitResource};

/// Resource selector for cgroup cpu controller limits.
#[cfg(target_os = "linux")]
const CGROUP_RESOURCE_CPU: u32 = libc::RLIMIT_CPU;
#[cfg(target_os = "android")]
const CGROUP_RESOURCE_CPU: u32 = libc::RLIMIT_CPU as u32;

/// Resource selector for cgroup memory controller limits.
#[cfg(target_os = "linux")]
const CGROUP_RESOURCE_MEMORY: u32 = libc::RLIMIT_AS;
#[cfg(target_os = "android")]
const CGROUP_RESOURCE_MEMORY: u32 = libc::RLIMIT_AS as u32;

/// Resource selector for cgroup process count limits.
#[cfg(target_os = "linux")]
const CGROUP_RESOURCE_PROCESSES: u32 = libc::RLIMIT_NPROC;
#[cfg(target_os = "android")]
const CGROUP_RESOURCE_PROCESSES: u32 = libc::RLIMIT_NPROC as u32;

/// Parsed control file shape for one cgroup resource.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[derive(Debug, Clone, Copy)]
enum CgroupLimitKind {
    /// One scalar file where `max` means unlimited.
    Scalar {
        /// Controller file name.
        file_name: &'static str,
    },
    /// Two-field cpu file `<quota|max> <period>`.
    Cpu,
}

/// Validate one cgroup directory path.
fn validate_cgroup_path(path: &str) -> RuntimeResult<()> {
    if path.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "cgroup path must not be empty",
        ))
        .boxed());
    }
    if path.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "cgroup path contains nul byte",
        ))
        .boxed());
    }

    Ok(())
}

/// Resolve one cgroup resource id into a controller file shape.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn cgroup_limit_kind(resource: u32) -> RuntimeResult<CgroupLimitKind> {
    if resource == CGROUP_RESOURCE_CPU {
        return Ok(CgroupLimitKind::Cpu);
    }
    if resource == CGROUP_RESOURCE_MEMORY {
        return Ok(CgroupLimitKind::Scalar {
            file_name: "memory.max",
        });
    }
    if resource == CGROUP_RESOURCE_PROCESSES {
        return Ok(CgroupLimitKind::Scalar {
            file_name: "pids.max",
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "resource",
        format!("unsupported cgroup resource selector {resource}"),
    ))
    .boxed())
}

/// Parse one scalar cgroup limit token.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn parse_scalar_limit(value: &str, label: &str) -> RuntimeResult<u64> {
    let value = value.trim();
    if value == "max" {
        return Ok(u64::MAX);
    }

    value.parse::<u64>().map_err(|error| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            None,
            Some(label.to_string()),
            format!("failed to parse cgroup limit value '{value}': {error}"),
        ))
        .boxed()
    })
}

/// Encode one scalar cgroup limit token.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn format_scalar_limit(value: u64) -> String {
    if value == u64::MAX {
        return "max".to_string();
    }

    value.to_string()
}

/// Parse one cgroup limit payload from controller text.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn parse_cgroup_limit(kind: CgroupLimitKind, raw: &str) -> RuntimeResult<ProcessLimit> {
    match kind {
        CgroupLimitKind::Scalar { file_name } => {
            let value = parse_scalar_limit(raw, file_name)?;
            Ok(ProcessLimit {
                soft: value,
                hard: value,
            })
        }
        CgroupLimitKind::Cpu => {
            let mut fields = raw.split_whitespace();
            let Some(quota_raw) = fields.next() else {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    None,
                    Some("cpu.max".to_string()),
                    "cpu.max is missing the quota field",
                ))
                .boxed());
            };
            let Some(period_raw) = fields.next() else {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    None,
                    Some("cpu.max".to_string()),
                    "cpu.max is missing the period field",
                ))
                .boxed());
            };
            if fields.next().is_some() {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    None,
                    Some("cpu.max".to_string()),
                    "cpu.max contains unexpected extra fields",
                ))
                .boxed());
            }

            let quota = parse_scalar_limit(quota_raw, "cpu.max")?;
            let period = period_raw.parse::<u64>().map_err(|error| {
                RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    None,
                    Some("cpu.max".to_string()),
                    format!("failed to parse cpu.max period '{period_raw}': {error}"),
                ))
                .boxed()
            })?;
            if period == 0 {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    None,
                    Some("cpu.max".to_string()),
                    "cpu.max period must be greater than zero",
                ))
                .boxed());
            }

            Ok(ProcessLimit {
                soft: quota,
                hard: period,
            })
        }
    }
}

/// Encode one cgroup limit payload into controller text.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn format_cgroup_limit(kind: CgroupLimitKind, limit: ProcessLimit) -> RuntimeResult<String> {
    match kind {
        CgroupLimitKind::Scalar { file_name } => {
            if limit.soft != limit.hard {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "limit",
                    format!("{file_name} requires soft and hard to be identical"),
                ))
                .boxed());
            }

            Ok(format_scalar_limit(limit.soft))
        }
        CgroupLimitKind::Cpu => {
            if limit.hard == 0 || limit.hard == u64::MAX {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "limit.hard",
                    "cpu period must be a finite value greater than zero",
                ))
                .boxed());
            }

            let quota = format_scalar_limit(limit.soft);
            Ok(format!("{quota} {}", limit.hard))
        }
    }
}

/// Build one control file path under a cgroup directory.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn cgroup_control_path(path: &str, file_name: &str) -> String {
    format!("{path}/{file_name}")
}

/// Build a process-domain error for one cgroup control file operation.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn cgroup_control_error(errno: i32, syscall: &str, control_path: &str) -> Box<RuntimeError> {
    let code = process_error_code_from_errno(errno).unwrap_or(PlatformErrorCode::Process);
    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(errno.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        format!("cgroup control file operation failed for {control_path}"),
    ))
    .boxed()
}

/// Read one cgroup controller file as UTF-8 text.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn read_cgroup_control_file(path: &str, file_name: &str) -> RuntimeResult<String> {
    let control_path = cgroup_control_path(path, file_name);
    let control_path_cstring = core_process::cstring_from_str(
        &control_path,
        "path",
        "cgroup control path contains nul byte",
    )?;

    let file_descriptor = unsafe { libc::open(control_path_cstring.as_ptr(), libc::O_RDONLY) };
    if file_descriptor < 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(cgroup_control_error(errno, "open", &control_path));
    }

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read_count = unsafe {
            libc::read(
                file_descriptor,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
            )
        };
        if read_count == 0 {
            break;
        }
        if read_count < 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            if errno == libc::EINTR {
                continue;
            }

            let _ = unsafe { libc::close(file_descriptor) };
            return Err(cgroup_control_error(errno, "read", &control_path));
        }

        bytes.extend_from_slice(&buffer[..read_count as usize]);
    }

    let close_result = unsafe { libc::close(file_descriptor) };
    if close_result != 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(cgroup_control_error(errno, "close", &control_path));
    }

    String::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            None,
            Some("read".to_string()),
            format!("cgroup control file is not valid utf8: {control_path}"),
        ))
        .boxed()
    })
}

/// Write one cgroup controller file as UTF-8 text.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn write_cgroup_control_file(path: &str, file_name: &str, value: &str) -> RuntimeResult<()> {
    let control_path = cgroup_control_path(path, file_name);
    let control_path_cstring = core_process::cstring_from_str(
        &control_path,
        "path",
        "cgroup control path contains nul byte",
    )?;

    let file_descriptor = unsafe { libc::open(control_path_cstring.as_ptr(), libc::O_WRONLY) };
    if file_descriptor < 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(cgroup_control_error(errno, "open", &control_path));
    }

    let bytes = value.as_bytes();
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let write_count = unsafe {
            libc::write(
                file_descriptor,
                bytes[offset..].as_ptr() as *const libc::c_void,
                bytes.len() - offset,
            )
        };
        if write_count < 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            if errno == libc::EINTR {
                continue;
            }

            let _ = unsafe { libc::close(file_descriptor) };
            return Err(cgroup_control_error(errno, "write", &control_path));
        }

        offset = offset.saturating_add(write_count as usize);
    }

    let close_result = unsafe { libc::close(file_descriptor) };
    if close_result != 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(cgroup_control_error(errno, "close", &control_path));
    }

    Ok(())
}

/// Read one control-group resource limit.
pub(crate) unsafe fn destack_process_cgroup_get_limit(
    _binding: &BindingCallContext,
    out: *mut ProcessLimit,
    path: NativeStringRef,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = unsafe { path.as_str()? };
    validate_cgroup_path(path)?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let kind = cgroup_limit_kind(resource.0)?;
        let raw = match kind {
            CgroupLimitKind::Scalar { file_name } => read_cgroup_control_file(path, file_name)?,
            CgroupLimitKind::Cpu => read_cgroup_control_file(path, "cpu.max")?,
        };
        let value = parse_cgroup_limit(kind, &raw)?;
        unsafe {
            *out = value;
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (path, resource);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.group.cgroupGetLimit",
        ))
        .boxed())
    }
}

/// Join one control group.
pub(crate) unsafe fn destack_process_cgroup_join(
    _binding: &BindingCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let path = unsafe { path.as_str()? };
    validate_cgroup_path(path)?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let pid = unsafe { libc::getpid() };
        write_cgroup_control_file(path, "cgroup.procs", &format!("{pid}\n"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.group.cgroupJoin",
        ))
        .boxed())
    }
}

/// Write one control-group resource limit.
pub(crate) unsafe fn destack_process_cgroup_set_limit(
    _binding: &BindingCallContext,
    path: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let path = unsafe { path.as_str()? };
    validate_cgroup_path(path)?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let kind = cgroup_limit_kind(resource.0)?;
        let encoded = format_cgroup_limit(kind, limit)?;
        match kind {
            CgroupLimitKind::Scalar { file_name } => {
                write_cgroup_control_file(path, file_name, &encoded)
            }
            CgroupLimitKind::Cpu => write_cgroup_control_file(path, "cpu.max", &encoded),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (resource, limit);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.group.cgroupSetLimit",
        ))
        .boxed())
    }
}

/// Assign processes to one Windows job object.
pub(crate) unsafe fn destack_process_job_assign(
    _binding: &BindingCallContext,
    name: NativeStringRef,
    pids: NativeSlice<ProcessId>,
) -> RuntimeResult<()> {
    let _ = unsafe { name.as_str()? };
    let _ = unsafe { pids.as_slice()? };

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign",
    ))
    .boxed())
}

/// Set one Windows job object resource limit.
pub(crate) unsafe fn destack_process_job_set_limit(
    _binding: &BindingCallContext,
    name: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = unsafe { name.as_str()? };
    let _ = (resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit",
    ))
    .boxed())
}

#[cfg(all(test, any(target_os = "linux", target_os = "android")))]
mod tests {
    use super::{
        CGROUP_RESOURCE_CPU, CGROUP_RESOURCE_MEMORY, CgroupLimitKind, cgroup_limit_kind,
        format_cgroup_limit, parse_cgroup_limit,
    };
    use crate::platform::process::ProcessLimit;

    /// Parse one scalar cgroup limit value into a process limit.
    #[test]
    fn test_parse_cgroup_scalar_limit() {
        let kind = cgroup_limit_kind(CGROUP_RESOURCE_MEMORY).expect("resource should map");
        let parsed = parse_cgroup_limit(kind, "4096\n").expect("parse should succeed");

        // scalar limits should map to identical soft and hard values
        assert_eq!(parsed.soft, 4096);
        assert_eq!(parsed.hard, 4096);
    }

    /// Parse one cpu cgroup limit value into a process limit.
    #[test]
    fn test_parse_cgroup_cpu_limit() {
        let kind = cgroup_limit_kind(CGROUP_RESOURCE_CPU).expect("resource should map");
        let parsed = parse_cgroup_limit(kind, "200000 100000\n").expect("parse should succeed");

        // cpu limits should preserve quota and period ordering
        assert_eq!(parsed.soft, 200000);
        assert_eq!(parsed.hard, 100000);
    }

    /// Reject scalar cgroup limit formatting when soft and hard differ.
    #[test]
    fn test_format_cgroup_scalar_limit_rejects_mismatch() {
        let encoded = format_cgroup_limit(
            CgroupLimitKind::Scalar {
                file_name: "memory.max",
            },
            ProcessLimit { soft: 1, hard: 2 },
        );
        // scalar limit encoding should fail on mismatched soft and hard values
        assert!(encoded.is_err());
    }

    /// Encode one cpu cgroup limit value with unlimited quota.
    #[test]
    fn test_format_cgroup_cpu_limit() {
        let encoded = format_cgroup_limit(
            CgroupLimitKind::Cpu,
            ProcessLimit {
                soft: u64::MAX,
                hard: 100000,
            },
        )
        .expect("encoding should succeed");

        // unlimited cpu quota should encode to the cgroup max keyword
        assert_eq!(encoded, "max 100000");
    }
}
