#![allow(dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use destack_vm::BindingContext;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::tests::{HarnessContext, HarnessValue};
use crate::platform::os::{
    HostIdentity, HostIdentityVm, MountEntry, MountEntryVm, PermissionEntry, PermissionEntryVm,
    SystemSnapshot, SystemSnapshotVm,
};
#[cfg(unix)]
use crate::platform::os::{LoadAverage, LoadAverageVm};
use crate::platform::{NativeArray, VmArray, fs};

/// Return one raw VM context pointer when this harness run uses VM bindings.
fn vm_context_pointer(context: &HarnessContext<'_>) -> Option<*mut ()> {
    context.vm_context
}

/// Return current unix time in nanoseconds.
pub(crate) fn now_unix_ns() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch");

    u64::try_from(now.as_nanos()).expect("system time nanoseconds should fit in u64")
}

/// Decode one host-identity harness value into owned fields.
pub(crate) fn decode_host_identity_value(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<HostIdentity, HostIdentityVm>,
) -> RuntimeResult<(String, String, String, String)> {
    match value {
        HarnessValue::Native(value) => {
            let hostname = unsafe { value.hostname.as_str() }?.to_string();
            let kernel = unsafe { value.kernel.as_str() }?.to_string();
            let release = unsafe { value.release.as_str() }?.to_string();
            let architecture = unsafe { value.architecture.as_str() }?.to_string();

            Ok((hostname, kernel, release, architecture))
        }
        HarnessValue::Vm(value) => {
            // decode the VM strings through the active VM context
            let vm_context = vm_context_pointer(context).expect("vm context should be available");
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let hostname = vm_context
                .string_ref(value.hostname)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let kernel = vm_context
                .string_ref(value.kernel)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let release = vm_context
                .string_ref(value.release)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let architecture = vm_context
                .string_ref(value.architecture)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();

            Ok((hostname, kernel, release, architecture))
        }
    }
}

/// Decode one load-average harness value.
#[cfg(unix)]
pub(crate) fn decode_load_average_value(
    value: HarnessValue<LoadAverage, LoadAverageVm>,
) -> LoadAverage {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Decode one system-snapshot harness value.
pub(crate) fn decode_system_snapshot_value(
    value: HarnessValue<SystemSnapshot, SystemSnapshotVm>,
) -> SystemSnapshot {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Decode one permission-entry array harness value.
pub(crate) fn decode_permission_entries_value(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeArray<PermissionEntry>, VmArray<PermissionEntryVm>>,
) -> RuntimeResult<Vec<PermissionEntry>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            Ok(values.to_vec())
        }
        HarnessValue::Vm(value) => {
            // read the VM array through the active VM context
            let vm_context = vm_context_pointer(context).expect("vm context should be available");
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };

            value.read_values(&vm_context.read())
        }
    }
}

/// Decode one mount-entry array harness value.
pub(crate) fn decode_mount_entries_value(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeArray<MountEntry>, VmArray<MountEntryVm>>,
) -> RuntimeResult<Vec<MountEntryValue>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            let mut entries = Vec::with_capacity(values.len());

            for value in values {
                entries.push(MountEntryValue {
                    source: if let Some(value) = value.source {
                        Some(unsafe { value.as_str() }?.to_string())
                    } else {
                        None
                    },
                    target_is_empty: native_os_path_is_empty(value.target)?,
                    file_system: if let Some(value) = value.file_system {
                        Some(unsafe { value.as_str() }?.to_string())
                    } else {
                        None
                    },
                    host_flags: value.host_flags,
                });
            }

            Ok(entries)
        }
        HarnessValue::Vm(value) => {
            // decode the VM mount entries through the active VM context
            let vm_context = vm_context_pointer(context).expect("vm context should be available");
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let values = value.read_values(&vm_context.read())?;
            let mut entries = Vec::with_capacity(values.len());

            for value in values {
                let source = if let Some(value) = value.source {
                    Some(
                        vm_context
                            .string_ref(value)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string(),
                    )
                } else {
                    None
                };
                let file_system = if let Some(value) = value.file_system {
                    Some(
                        vm_context
                            .string_ref(value)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string(),
                    )
                } else {
                    None
                };

                entries.push(MountEntryValue {
                    source,
                    target_is_empty: vm_os_path_is_empty(vm_context, value.target)?,
                    file_system,
                    host_flags: value.host_flags,
                });
            }

            Ok(entries)
        }
    }
}

/// Decoded mount entry for shared test assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MountEntryValue {
    /// Source device or backing object.
    pub source: Option<String>,
    /// Whether the target path was empty.
    pub target_is_empty: bool,
    /// Filesystem type name.
    pub file_system: Option<String>,
    /// Host-defined flags bitmask.
    pub host_flags: Option<u64>,
}

/// Return whether one native `OsPath` payload is empty.
fn native_os_path_is_empty(path: fs::OsPath) -> RuntimeResult<bool> {
    match path {
        fs::OsPath::OsPathBytes(path) => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            Ok(bytes.is_empty())
        }
        fs::OsPath::OsPathUtf16(path) => {
            let utf16 = unsafe { path.utf16.0.as_slice()? };
            Ok(utf16.is_empty())
        }
    }
}

/// Return whether one VM `OsPath` payload is empty.
fn vm_os_path_is_empty(
    context: &mut BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<bool> {
    match path {
        fs::OsPathVm::OsPathBytes(path) => {
            let bytes = path.bytes.0.read_bytes(&context.read())?;
            Ok(bytes.is_empty())
        }
        fs::OsPathVm::OsPathUtf16(path) => {
            let utf16 = path.utf16.0.read_values(&context.read())?;
            Ok(utf16.is_empty())
        }
    }
}
