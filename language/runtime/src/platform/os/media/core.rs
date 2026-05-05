use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::host::operation::media as host_media;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::platform::os::{MediaAssetDescriptorVm, MediaAssetKind, MediaPageVm};
use crate::platform::{VmAbiCodec, fs};
use crate::runtime::BindingCallContext;

/// List media assets through the active host session.
pub(crate) fn list(
    binding: &BindingCallContext,
    query: MediaQueryValue,
) -> RuntimeResult<MediaPageValue> {
    binding.host().submit_operation(host_media::list(query))
}

/// Read one media asset through the active host session.
pub(crate) fn read(
    binding: &BindingCallContext,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    binding
        .host()
        .submit_operation(host_media::read(id.to_string()))
}

/// Import one media asset path through the active host session.
pub(crate) fn import_path(
    binding: &BindingCallContext,
    path: fs::OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<String> {
    binding
        .host()
        .submit_operation(host_media::import_path(path, kind))
}

/// Delete media assets through the active host session.
pub(crate) fn delete(binding: &BindingCallContext, ids: Vec<String>) -> RuntimeResult<u32> {
    binding.host().submit_operation(host_media::delete(ids))
}

/// Encode one media page into one VM value.
pub(crate) fn page_vm(
    context: &mut vm::BindingContext<'_>,
    page: MediaPageValue,
) -> RuntimeResult<MediaPageVm> {
    MediaPageVm::from_value(&mut context.write(), page)
}

/// Encode one media asset descriptor into one VM value.
pub(crate) fn descriptor_vm(
    context: &mut vm::BindingContext<'_>,
    descriptor: MediaAssetDescriptorValue,
) -> RuntimeResult<MediaAssetDescriptorVm> {
    MediaAssetDescriptorVm::from_value(&mut context.write(), descriptor)
}

/// Encode one identifier into one VM string handle.
pub(crate) fn id_vm(
    context: &mut vm::BindingContext<'_>,
    id: &str,
) -> RuntimeResult<vm::StringHandle> {
    <vm::StringHandle as VmAbiCodec>::from_value(&mut context.write(), id.to_string())
}
