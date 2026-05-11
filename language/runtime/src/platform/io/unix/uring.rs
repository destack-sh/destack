#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use io_uring::IoUring;
#[cfg(target_os = "linux")]
use libc::c_void;
#[cfg(target_os = "linux")]
use parking_lot::Mutex;

use super::core::require_out;
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "linux")]
use crate::platform::PlatformErrorCode;
#[cfg(target_os = "linux")]
use crate::platform::ResourceId;
use crate::platform::abi::NativeSlice;
#[cfg(target_os = "linux")]
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::io::{UringFeatures, UringParameters};
#[cfg(target_os = "linux")]
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resource label for io_uring ring payloads.
#[cfg(target_os = "linux")]
const URING_RESOURCE_LABEL: &str = "io.uring";
#[cfg(target_os = "linux")]
const IORING_SETUP_IOPOLL: u32 = 1 << 0;
#[cfg(target_os = "linux")]
const IORING_SETUP_SQPOLL: u32 = 1 << 1;
#[cfg(target_os = "linux")]
const IORING_SETUP_CLAMP: u32 = 1 << 4;
#[cfg(target_os = "linux")]
const IORING_SETUP_R_DISABLED: u32 = 1 << 6;
#[cfg(target_os = "linux")]
const IORING_SETUP_SUBMIT_ALL: u32 = 1 << 7;
#[cfg(target_os = "linux")]
const IORING_SETUP_COOP_TASKRUN: u32 = 1 << 8;
#[cfg(target_os = "linux")]
const IORING_SETUP_TASKRUN_FLAG: u32 = 1 << 9;
#[cfg(target_os = "linux")]
const IORING_SETUP_SINGLE_ISSUER: u32 = 1 << 12;
#[cfg(target_os = "linux")]
const IORING_SETUP_DEFER_TASKRUN: u32 = 1 << 13;

/// Runtime payload for one io_uring ring instance.
#[cfg(target_os = "linux")]
struct UringResource {
    /// Shared ring state guarded for concurrent runtime access.
    state: Mutex<UringState>,
}

#[cfg(target_os = "linux")]
impl UringResource {
    /// Construct one ring payload from one concrete io_uring instance.
    fn new(ring: IoUring) -> Self {
        Self {
            state: Mutex::new(UringState {
                ring,
                has_registered_files: false,
                has_registered_buffers: false,
            }),
        }
    }
}

/// Mutable io_uring state tracked by one runtime ring resource.
#[cfg(target_os = "linux")]
struct UringState {
    /// Underlying io_uring ring instance.
    ring: IoUring,
    /// Whether fixed files are currently registered.
    has_registered_files: bool,
    /// Whether fixed buffers are currently registered.
    has_registered_buffers: bool,
}

/// Build one not-found error for io_uring handles.
#[cfg(target_os = "linux")]
fn uring_not_found(operation: &'static str, handle: resource::UringHandle) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("io_uring handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Build one io error from one explicit errno and message.
#[cfg(target_os = "linux")]
fn io_error_with_errno(operation: &'static str, errno: i32, detail: String) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        io_error_code_from_errno(errno),
        None,
        Some(errno),
        Some(operation.to_string()),
        None,
        detail,
    ))
    .boxed()
}

/// Build one io error from one std::io::Error.
#[cfg(target_os = "linux")]
fn io_error_from_std(operation: &'static str, error: std::io::Error) -> Box<RuntimeError> {
    if let Some(errno) = error.raw_os_error() {
        return io_error_with_errno(operation, errno, format!("{operation} failed: {error}"));
    }

    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("{operation} failed: {error}"),
    ))
    .boxed()
}

/// Build one not-found error for io targets.
#[cfg(target_os = "linux")]
fn io_target_not_found(operation: &'static str, target: ResourceId) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("target resource {} not found", target.0),
    ))
    .boxed()
}

/// Resolve one io_uring payload from one uring handle.
#[cfg(target_os = "linux")]
fn resolve_uring_resource(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<Arc<UringResource>> {
    // resolve one io_uring resource payload
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Uring {
                return None;
            }

            if entry.label.as_deref() != Some(URING_RESOURCE_LABEL) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<UringResource>>())
                .cloned()
        })
        .flatten();

    match resolved {
        Some(resource) => Ok(resource),
        None => Err(uring_not_found("destack.io.uring.handle", handle)),
    }
}

/// Close one io_uring ring.
pub(crate) unsafe fn destack_io_uring_close(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        resolve_uring_resource(binding, handle)?;

        let removed = binding.worker().resources.remove_and_finalize(
            &binding.world(),
            handle.0,
            Some(binding.engine()),
        );
        if !removed {
            return Err(uring_not_found("destack.io.uring.close", handle));
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.close")).boxed())
    }
}

/// Query io_uring feature support.
pub(crate) unsafe fn destack_io_uring_features(
    binding: &BindingCallContext,
    out: *mut UringFeatures,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    require_out(out)?;

    #[cfg(target_os = "linux")]
    {
        let resource = resolve_uring_resource(binding, handle)?;
        let state = resource.state.lock();
        let parameters = state.ring.params();
        let value = UringFeatures {
            has_submission_polling: parameters.is_setup_sqpoll(),
            has_kernel_polling: parameters.is_setup_iopoll(),
            has_fixed_files: true,
            has_fixed_buffers: true,
            max_entries: parameters.sq_entries(),
        };

        unsafe {
            out.write(value);
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.features")).boxed())
    }
}

/// Open one io_uring ring.
pub(crate) unsafe fn destack_io_uring_open(
    binding: &BindingCallContext,
    out: *mut resource::UringHandle,
    parameters: UringParameters,
) -> RuntimeResult<()> {
    require_out(out)?;

    #[cfg(target_os = "linux")]
    {
        if parameters.entries == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "parameters.entries",
                "entries must be greater than zero",
            ))
            .boxed());
        }

        let mut builder = IoUring::builder();
        let mut remaining = parameters.flags;
        if (remaining & IORING_SETUP_IOPOLL) != 0 {
            builder.setup_iopoll();
            remaining &= !IORING_SETUP_IOPOLL;
        }
        if (remaining & IORING_SETUP_SQPOLL) != 0 {
            builder.setup_sqpoll(parameters.sq_thread_idle_ms);
            remaining &= !IORING_SETUP_SQPOLL;
        }
        if (remaining & IORING_SETUP_CLAMP) != 0 {
            builder.setup_clamp();
            remaining &= !IORING_SETUP_CLAMP;
        }
        if (remaining & IORING_SETUP_R_DISABLED) != 0 {
            builder.setup_r_disabled();
            remaining &= !IORING_SETUP_R_DISABLED;
        }
        if (remaining & IORING_SETUP_SUBMIT_ALL) != 0 {
            builder.setup_submit_all();
            remaining &= !IORING_SETUP_SUBMIT_ALL;
        }
        if (remaining & IORING_SETUP_COOP_TASKRUN) != 0 {
            builder.setup_coop_taskrun();
            remaining &= !IORING_SETUP_COOP_TASKRUN;
        }
        if (remaining & IORING_SETUP_TASKRUN_FLAG) != 0 {
            builder.setup_taskrun_flag();
            remaining &= !IORING_SETUP_TASKRUN_FLAG;
        }
        if (remaining & IORING_SETUP_SINGLE_ISSUER) != 0 {
            builder.setup_single_issuer();
            remaining &= !IORING_SETUP_SINGLE_ISSUER;
        }
        if (remaining & IORING_SETUP_DEFER_TASKRUN) != 0 {
            builder.setup_defer_taskrun();
            remaining &= !IORING_SETUP_DEFER_TASKRUN;
        }

        if remaining != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "parameters.flags",
                format!("unsupported io_uring setup flags: {remaining:#x}"),
            ))
            .boxed());
        }

        let ring = builder
            .build(parameters.entries)
            .map_err(|error| io_error_from_std("io_uring_setup", error))?;
        let resource = Arc::new(UringResource::new(ring));
        let entry = ResourceEntry::new(ResourceKind::Uring)
            .with_label(URING_RESOURCE_LABEL)
            .with_payload(resource);
        let value =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));

        unsafe {
            out.write(resource::UringHandle(value));
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, parameters);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.open")).boxed())
    }
}

/// Register fixed buffers with a ring.
pub(crate) unsafe fn destack_io_uring_register_buffers(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
    addresses: NativeSlice<u64>,
    lengths: NativeSlice<u32>,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let addresses = unsafe { addresses.as_slice()? };
        let lengths = unsafe { lengths.as_slice()? };
        if addresses.len() != lengths.len() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "lengths",
                "addresses and lengths must have matching lengths",
            ))
            .boxed());
        }

        let mut iovecs = Vec::with_capacity(addresses.len());
        for (address, length) in addresses.iter().zip(lengths) {
            if *length != 0 && *address == 0 {
                return Err(RuntimeError::from(PlatformError::null_pointer("addresses")).boxed());
            }

            iovecs.push(libc::iovec {
                iov_base: *address as *mut c_void,
                iov_len: *length as usize,
            });
        }

        let resource = resolve_uring_resource(binding, handle)?;
        let mut state = resource.state.lock();
        unsafe {
            state
                .ring
                .submitter()
                .register_buffers(&iovecs)
                .map_err(|error| io_error_from_std("io_uring_register_buffers", error))?;
        }
        state.has_registered_buffers = true;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, addresses, lengths);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.io.uring.registerBuffers",
        ))
        .boxed())
    }
}

/// Register fixed files with a ring.
pub(crate) unsafe fn destack_io_uring_register_files(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
    files: NativeSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let files = unsafe { files.as_slice()? };
        let mut descriptors = Vec::with_capacity(files.len());
        for file in files {
            let fd = binding
                .worker()
                .resources
                .with_entry(*file, |entry| entry.fd())
                .flatten()
                .ok_or_else(|| io_target_not_found("destack.io.uring.registerFiles", *file))?;
            descriptors.push(fd);
        }

        let resource = resolve_uring_resource(binding, handle)?;
        let mut state = resource.state.lock();
        state
            .ring
            .submitter()
            .register_files(&descriptors)
            .map_err(|error| io_error_from_std("io_uring_register_files", error))?;
        state.has_registered_files = true;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, files);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.io.uring.registerFiles",
        ))
        .boxed())
    }
}

/// Unregister fixed buffers for a ring.
pub(crate) unsafe fn destack_io_uring_unregister_buffers(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let resource = resolve_uring_resource(binding, handle)?;
        let mut state = resource.state.lock();
        if !state.has_registered_buffers {
            return Ok(());
        }

        state
            .ring
            .submitter()
            .unregister_buffers()
            .map_err(|error| io_error_from_std("io_uring_unregister_buffers", error))?;
        state.has_registered_buffers = false;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.io.uring.unregisterBuffers",
        ))
        .boxed())
    }
}

/// Unregister fixed files for a ring.
pub(crate) unsafe fn destack_io_uring_unregister_files(
    binding: &BindingCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let resource = resolve_uring_resource(binding, handle)?;
        let mut state = resource.state.lock();
        if !state.has_registered_files {
            return Ok(());
        }

        state
            .ring
            .submitter()
            .unregister_files()
            .map_err(|error| io_error_from_std("io_uring_unregister_files", error))?;
        state.has_registered_files = false;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.io.uring.unregisterFiles",
        ))
        .boxed())
    }
}
