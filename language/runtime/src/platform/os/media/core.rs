use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::host::operation::media as host_media;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaEventValue, MediaPageValue, MediaQueryValue,
    MediaWatchOptionsValue,
};
use crate::platform::os::{MediaAssetDescriptorVm, MediaAssetKind, MediaEventVm, MediaPageVm};
use crate::platform::{VmAbiCodec, fs, resource};
use crate::runtime::BindingCallContext;

use crate::platform::os::state;

/// List media assets through the active host session.
pub(crate) fn list(
    binding: &BindingCallContext,
    query: MediaQueryValue,
) -> RuntimeResult<MediaPageValue> {
    binding.host().submit_operation(host_media::list(query))
}

/// Describe one media asset through the active host session.
pub(crate) fn describe(
    binding: &BindingCallContext,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    binding
        .host()
        .submit_operation(host_media::describe(id.to_string()))
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

/// Open one media watch stream.
pub(crate) fn watch_open(
    binding: &BindingCallContext,
    options: MediaWatchOptionsValue,
) -> RuntimeResult<resource::MediaWatchHandle> {
    state::media_watch_open(binding, options)
}

/// Close one media watch stream.
pub(crate) fn watch_close(
    binding: &BindingCallContext,
    handle: resource::MediaWatchHandle,
) -> RuntimeResult<()> {
    state::media_watch_close(binding, handle)
}

/// Wait for one media watch event.
pub(crate) fn watch_read(
    binding: &BindingCallContext,
    handle: resource::MediaWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<MediaEventValue> {
    state::media_watch_read(binding, handle, timeout_ns)
}

/// Poll one media watch event without blocking.
pub(crate) fn watch_try_read(
    binding: &BindingCallContext,
    handle: resource::MediaWatchHandle,
) -> RuntimeResult<MediaEventValue> {
    state::media_watch_try_read(binding, handle)
}

/// Encode one media page into one VM value.
pub(crate) fn page_vm(
    context: &mut vm::ExternalCallContext<'_>,
    page: MediaPageValue,
) -> RuntimeResult<MediaPageVm> {
    MediaPageVm::from_value(context, page)
}

/// Encode one media asset descriptor into one VM value.
pub(crate) fn descriptor_vm(
    context: &mut vm::ExternalCallContext<'_>,
    descriptor: MediaAssetDescriptorValue,
) -> RuntimeResult<MediaAssetDescriptorVm> {
    MediaAssetDescriptorVm::from_value(context, descriptor)
}

/// Encode one media event into one VM value.
pub(crate) fn event_vm(
    context: &mut vm::ExternalCallContext<'_>,
    event: MediaEventValue,
) -> RuntimeResult<MediaEventVm> {
    MediaEventVm::from_value(context, event)
}

/// Encode one identifier into one VM string handle.
pub(crate) fn id_vm(
    context: &mut vm::ExternalCallContext<'_>,
    id: &str,
) -> RuntimeResult<vm::StringHandle> {
    <vm::StringHandle as VmAbiCodec>::from_value(context, id.to_string())
}
