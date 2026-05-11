use std::ffi::CString;
use std::os::fd::RawFd;

use libc::{O_CLOEXEC, c_void};

use super::core::{host_control_ioctl, require_out};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeSlice;
use crate::platform::fs::{OsPath, OsPathBytes, OsPathUtf16};
use crate::platform::io::{DescriptorRequest, DescriptorResult, core as io_core};
use crate::platform::resource::{self, ResourceEntry, ResourceFinalizer, ResourceKind};
use crate::runtime::BindingCallContext;

/// Open flags that would create or rewrite filesystem entries instead of opening devices.
const DEVICE_MUTATING_OPEN_FLAGS: u32 =
    libc::O_CREAT as u32 | libc::O_EXCL as u32 | libc::O_TRUNC as u32;

/// Finalizer that closes one owned raw device descriptor.
#[derive(Debug)]
struct UnixDeviceFinalizer {
    /// Descriptor to close.
    descriptor: RawFd,
}

impl ResourceFinalizer for UnixDeviceFinalizer {
    /// Close one owned raw device descriptor.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// Resolve one device handle into one unix descriptor.
fn device_descriptor(
    binding: &BindingCallContext,
    handle: resource::DeviceHandle,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Device {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                format!("unknown device handle for {operation}"),
            ))
            .boxed()
        })?;

    Ok(descriptor)
}

/// Resolve one unix device path into one owned c string.
fn device_path_cstring(path: OsPath, field: &'static str) -> RuntimeResult<CString> {
    match path {
        OsPath::OsPathBytes(OsPathBytes { bytes, .. }) => {
            let path = unsafe { bytes.0.as_slice()? };
            CString::new(path).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    field,
                    "path bytes contain an interior nul byte",
                ))
                .boxed()
            })
        }
        OsPath::OsPathUtf16(OsPathUtf16 { utf16, .. }) => {
            let path = unsafe { utf16.0.as_slice()? };
            let path = String::from_utf16(path).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    field,
                    "path utf16 payload is not valid Unicode",
                ))
                .boxed()
            })?;
            CString::new(path.into_bytes()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    field,
                    "path contains an interior nul byte",
                ))
                .boxed()
            })
        }
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

    Ok(())
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

    validate_device_open_flags(flags)?;

    let path = device_path_cstring(path, "path")?;
    let descriptor = unsafe { libc::open(path.as_ptr(), flags as libc::c_int | O_CLOEXEC, mode) };
    if descriptor < 0 {
        return Err(io_core::io_error_from_errno("open"));
    }

    // reject ordinary files and keep the device lane specific to real device nodes
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let status = unsafe { libc::fstat(descriptor, stat.as_mut_ptr()) };
    if status != 0 {
        let error = io_core::io_error_from_errno("fstat");
        unsafe {
            libc::close(descriptor);
        }
        return Err(error);
    }
    let stat = unsafe { stat.assume_init() };
    let file_type = stat.st_mode & libc::S_IFMT;
    let is_device = file_type == libc::S_IFCHR || file_type == libc::S_IFBLK;
    if !is_device {
        unsafe {
            libc::close(descriptor);
        }
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path is not one character or block device",
        ))
        .boxed());
    }

    let entry = ResourceEntry::new(ResourceKind::Device)
        .with_fd(descriptor)
        .with_finalizer(UnixDeviceFinalizer { descriptor });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

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
        binding.world(),
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

    let descriptor = device_descriptor(binding, handle, "destack.io.device.read")?;
    let bytes = unsafe { buffer.as_mut_slice()? };
    let read = unsafe { libc::read(descriptor, bytes.as_mut_ptr().cast::<c_void>(), bytes.len()) };
    if read < 0 {
        return Err(io_core::io_error_from_errno("read"));
    }

    unsafe {
        out.write(read as u64);
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

    let descriptor = device_descriptor(binding, handle, "destack.io.device.write")?;
    let bytes = unsafe { buffer.as_slice()? };
    let written = unsafe { libc::write(descriptor, bytes.as_ptr().cast::<c_void>(), bytes.len()) };
    if written < 0 {
        return Err(io_core::io_error_from_errno("write"));
    }

    unsafe {
        out.write(written as u64);
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
    let _ = device_descriptor(binding, handle, "destack.io.device.control")?;

    let value = host_control_ioctl(binding, handle.0, request)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}
