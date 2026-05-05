use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::VmArray;
use crate::platform::abi::VmAbi;
#[cfg(any(unix, not(any(unix, windows))))]
use crate::platform::fs::PathBytesAbi;
#[cfg(windows)]
use crate::platform::fs::PathUtf16Abi;
use crate::platform::fs::{self, core as core_fs};
use crate::platform::os::{MountEntry, MountEntryVm};
use crate::runtime::BindingCallContext;

/// One host-owned mount table entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MountEntryOwned {
    /// Source device or backing object.
    pub source: Option<String>,
    /// Target mount path.
    pub target: PathBuf,
    /// Filesystem type name when the host exposes one.
    pub file_system: Option<String>,
    /// Host-defined mount flags when the host exposes them.
    pub host_flags: Option<u64>,
}

/// Read the current host mount table and encode it for the native ABI.
pub(crate) fn read_mount_entries(binding: &BindingCallContext) -> RuntimeResult<Vec<MountEntry>> {
    let entries = super::target::read_mount_entries()?;
    let entries = entries
        .iter()
        .map(|entry| native_mount_entry(binding, entry))
        .collect();

    Ok(entries)
}

/// Read the current host mount table and encode it for the VM ABI.
pub(crate) fn read_mount_entries_vm(
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<MountEntryVm>> {
    let entries = super::target::read_mount_entries()?;
    let entries = entries
        .iter()
        .map(|entry| vm_mount_entry(context, entry))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(&mut context.write(), &entries)
}

/// Encode one mount entry for the native ABI.
fn native_mount_entry(binding: &BindingCallContext, entry: &MountEntryOwned) -> MountEntry {
    MountEntry {
        source: entry
            .source
            .as_ref()
            .map(|value| binding.store_string(value)),
        target: native_os_path_from_path(binding, &entry.target),
        file_system: entry
            .file_system
            .as_ref()
            .map(|value| binding.store_string(value)),
        host_flags: entry.host_flags,
    }
}

/// Encode one mount entry for the VM ABI.
fn vm_mount_entry(
    context: &mut vm::BindingContext<'_>,
    entry: &MountEntryOwned,
) -> RuntimeResult<MountEntryVm> {
    Ok(MountEntryVm {
        source: if let Some(value) = &entry.source {
            Some(vm::StringHandle::new(context.intern_string(value)?))
        } else {
            None
        },
        target: vm_os_path_from_path(context, &entry.target)?,
        file_system: if let Some(value) = &entry.file_system {
            Some(vm::StringHandle::new(context.intern_string(value)?))
        } else {
            None
        },
        host_flags: entry.host_flags,
    })
}

/// Encode one native path payload from a host path.
#[cfg(unix)]
fn native_os_path_from_path(binding: &BindingCallContext, path: &Path) -> fs::OsPath {
    let bytes = PathBytesAbi(binding.store_array_copy(path.as_os_str().as_bytes()));

    core_fs::path_ref_from_bytes(bytes)
}

/// Encode one native path payload from a host path.
#[cfg(windows)]
fn native_os_path_from_path(binding: &BindingCallContext, path: &Path) -> fs::OsPath {
    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let utf16 = PathUtf16Abi(binding.store_array(units));

    core_fs::path_ref_from_utf16(utf16)
}

/// Encode one native path payload from a host path.
#[cfg(not(any(unix, windows)))]
fn native_os_path_from_path(binding: &BindingCallContext, path: &Path) -> fs::OsPath {
    let bytes = PathBytesAbi(binding.store_array_copy(path.to_string_lossy().as_bytes()));

    core_fs::path_ref_from_bytes(bytes)
}

/// Encode one VM path payload from a host path.
#[cfg(unix)]
fn vm_os_path_from_path(
    context: &mut vm::BindingContext<'_>,
    path: &Path,
) -> RuntimeResult<fs::OsPathVm> {
    let bytes = PathBytesAbi::<VmAbi>(VmArray::from_bytes(
        &mut context.write(),
        path.as_os_str().as_bytes(),
    )?);
    let kind = vm::StringHandle::new(context.intern_string("bytes")?);

    Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
}

/// Encode one VM path payload from a host path.
#[cfg(windows)]
fn vm_os_path_from_path(
    context: &mut vm::BindingContext<'_>,
    path: &Path,
) -> RuntimeResult<fs::OsPathVm> {
    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let utf16 = PathUtf16Abi::<VmAbi>(VmArray::from_values(&mut context.write(), &units)?);
    let kind = vm::StringHandle::new(context.intern_string("utf16")?);

    Ok(fs::OsPathVm::OsPathUtf16(fs::OsPathUtf16Vm { kind, utf16 }))
}

/// Encode one VM path payload from a host path.
#[cfg(not(any(unix, windows)))]
fn vm_os_path_from_path(
    context: &mut vm::BindingContext<'_>,
    path: &Path,
) -> RuntimeResult<fs::OsPathVm> {
    let text = path.to_string_lossy();
    let bytes = PathBytesAbi::<VmAbi>(VmArray::from_bytes(&mut context.write(), text.as_bytes())?);
    let kind = vm::StringHandle::new(context.intern_string("bytes")?);

    Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
}
