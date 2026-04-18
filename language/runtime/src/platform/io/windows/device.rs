use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_ALWAYS, OPEN_EXISTING, ReadFile, WriteFile,
};

use super::core::{host_control_ioctl, require_out};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::fs::OsPath;
use crate::platform::io::{DescriptorRequest, DescriptorResult};
use crate::platform::resource::{self, ResourceEntry, ResourceFinalizer, ResourceKind};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use std::os::windows::ffi::OsStrExt;

/// Access mode bits mirrored from the portable open-flag contract.
const OPEN_ACCESS_MODE_MASK: u32 = 0b11;
/// Read-only access mode.
const OPEN_ACCESS_READ_ONLY: u32 = 0;
/// Write-only access mode.
const OPEN_ACCESS_WRITE_ONLY: u32 = 1;
/// Open flags that would create or rewrite filesystem entries instead of opening devices.
const DEVICE_MUTATING_OPEN_FLAGS: u32 =
    libc::O_CREAT as u32 | libc::O_EXCL as u32 | libc::O_TRUNC as u32;

/// Finalizer that closes one owned raw device handle.
#[derive(Debug)]
struct WindowsDeviceFinalizer {
    /// Handle to close.
    handle: HANDLE,
}

impl ResourceFinalizer for WindowsDeviceFinalizer {
    /// Close one owned raw device handle.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

/// Resolve one device handle into one windows handle.
fn device_handle(
    binding: &BindingCallContext,
    handle: resource::DeviceHandle,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let handle = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Device {
                return None;
            }

            entry.handle()
        })
        .flatten()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                format!("unknown device handle for {operation}"),
            ))
            .boxed()
        })?;

    Ok(handle as HANDLE)
}

/// Map one generic open-flag payload into windows desired access.
fn device_desired_access(flags: u32) -> u32 {
    let mut access = 0u32;
    let access_mode = flags & OPEN_ACCESS_MODE_MASK;

    if access_mode != OPEN_ACCESS_WRITE_ONLY {
        access |= FILE_GENERIC_READ;
    }

    if access_mode != OPEN_ACCESS_READ_ONLY {
        access |= FILE_GENERIC_WRITE;
    }

    access
}

/// Map one generic open-flag payload into one windows creation mode.
fn device_creation(flags: u32) -> u32 {
    if flags & libc::O_CREAT as u32 != 0 {
        OPEN_ALWAYS
    } else {
        OPEN_EXISTING
    }
}

/// Reject open flags that are incompatible with raw device semantics.
fn validate_device_open_flags(flags: u32) -> RuntimeResult<()> {
    if flags & DEVICE_MUTATING_OPEN_FLAGS != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "device open does not support create, exclusive, or truncate flags",
        ))
        .boxed());
    }

    let supported_flags = OPEN_ACCESS_MODE_MASK;
    let unknown_flags = flags & !supported_flags;
    if unknown_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "device open only supports read and write access mode bits on Windows",
        ))
        .boxed());
    }

    Ok(())
}

/// Return whether one Windows path string is in the device namespace.
fn is_windows_device_path(path: &std::path::Path) -> bool {
    let path = path.as_os_str().to_string_lossy();
    path.starts_with(r"\\.\")
}

/// Open one raw device endpoint.
pub(crate) unsafe fn destack_io_device_open(
    binding: &BindingCallContext,
    out: *mut resource::DeviceHandle,
    path: OsPath,
    flags: u32,
    mode: u32,
) -> RuntimeResult<()> {
    require_out(out)?;

    let _ = mode;
    validate_device_open_flags(flags)?;
    let path = match path {
        OsPath::OsPathBytes(path) => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            core_platform::pathbuf_from_utf8("path", bytes)?
        }
        OsPath::OsPathUtf16(path) => {
            let units = unsafe { path.utf16.0.as_slice()? };
            core_platform::pathbuf_from_utf16("path", units)?
        }
    };
    if !is_windows_device_path(&path) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path is not one Windows device namespace path",
        ))
        .boxed());
    }
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            device_desired_access(flags),
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            device_creation(flags),
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(core_platform::io_error("CreateFileW"));
    }

    let inherit_status = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if inherit_status == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(core_platform::io_error("SetHandleInformation"));
    }

    let entry = ResourceEntry::new(ResourceKind::Device)
        .with_handle(handle as _)
        .with_finalizer(WindowsDeviceFinalizer { handle });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::DeviceHandle(resource_id));
    }

    Ok(())
}

/// Close one raw device endpoint.
pub(crate) unsafe fn destack_io_device_close(
    binding: &BindingCallContext,
    handle: resource::DeviceHandle,
) -> RuntimeResult<()> {
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown device handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Read bytes from one raw device endpoint.
pub(crate) unsafe fn destack_io_device_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    require_out(out)?;

    let handle = device_handle(binding, handle, "destack.io.device.read")?;
    let bytes = unsafe { buffer.as_mut_slice()? };
    let length = u32::try_from(bytes.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer length exceeds windows u32 range",
        ))
        .boxed()
    })?;

    let mut read = 0u32;
    let status = unsafe {
        ReadFile(
            handle,
            bytes.as_mut_ptr(),
            length,
            &mut read,
            std::ptr::null_mut(),
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("ReadFile"));
    }

    unsafe {
        out.write(u64::from(read));
    }

    Ok(())
}

/// Write bytes to one raw device endpoint.
pub(crate) unsafe fn destack_io_device_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::DeviceHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    require_out(out)?;

    let handle = device_handle(binding, handle, "destack.io.device.write")?;
    let bytes = unsafe { buffer.as_slice()? };
    let length = u32::try_from(bytes.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer length exceeds windows u32 range",
        ))
        .boxed()
    })?;

    let mut written = 0u32;
    let status = unsafe {
        WriteFile(
            handle,
            bytes.as_ptr(),
            length,
            &mut written,
            std::ptr::null_mut(),
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("WriteFile"));
    }

    unsafe {
        out.write(u64::from(written));
    }

    Ok(())
}

/// Run one device-specific control request.
pub(crate) unsafe fn destack_io_device_control(
    binding: &BindingCallContext,
    out: *mut DescriptorResult,
    handle: resource::DeviceHandle,
    request: DescriptorRequest,
) -> RuntimeResult<()> {
    require_out(out)?;
    let _ = device_handle(binding, handle, "destack.io.device.control")?;

    let value = host_control_ioctl(binding, handle.0, request)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}
