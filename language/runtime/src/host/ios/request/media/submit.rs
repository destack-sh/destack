use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::media::{
    HostMediaAssetDescriptor, HostMediaAssetKind, HostMediaPage, HostMediaQueryPayload,
};
use crate::host::core::callback::{
    decode_callback_host_required_string_ref, decode_callback_host_status,
};
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::host::ios::abi::media::{
    destack_host_ios_media_delete, destack_host_ios_media_describe,
    destack_host_ios_media_import_path, destack_host_ios_media_list,
};
use crate::platform::fs::{OsPath, core as core_fs};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Return one iOS media request outcome when supported.
pub(crate) fn submit_media_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsMediaList { query } => {
            let page = submit_media_list(runtime_id, request.operation_name(), query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaPage(page),
            )))
        }
        HostRequest::OsMediaRead { id } => {
            let descriptor = submit_media_describe(runtime_id, request.operation_name(), id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaAssetDescriptor(descriptor),
            )))
        }
        HostRequest::OsMediaImportPath { path, kind } => {
            let id = submit_media_import_path(runtime_id, request.operation_name(), *path, *kind)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsMediaDelete { ids } => {
            let deleted_count = submit_media_delete(runtime_id, request.operation_name(), ids)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::U32(
                deleted_count,
            ))))
        }
        _ => Ok(None),
    }
}

/// Submit one iOS media-list request.
fn submit_media_list(
    runtime_id: u64,
    operation: &'static str,
    query: &MediaQueryValue,
) -> RuntimeResult<MediaPageValue> {
    let query = HostMediaQueryPayload::from(query);
    let mut output_page = MaybeUninit::<HostMediaPage>::uninit();
    let status =
        unsafe { destack_host_ios_media_list(runtime_id, query.abi(), output_page.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let output_page = unsafe { output_page.assume_init() };

    output_page.decode(operation)
}

/// Submit one iOS media-describe request.
fn submit_media_describe(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    let mut output_descriptor = MaybeUninit::<HostMediaAssetDescriptor>::uninit();
    let status = unsafe {
        destack_host_ios_media_describe(
            runtime_id,
            NativeStringRef::from(id),
            output_descriptor.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let output_descriptor = unsafe { output_descriptor.assume_init() };

    output_descriptor.decode(operation)
}

/// Submit one iOS media-import request.
fn submit_media_import_path(
    runtime_id: u64,
    operation: &'static str,
    path: OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<String> {
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let mut output_id = MaybeUninit::<NativeStringRef>::uninit();
    let status = unsafe {
        destack_host_ios_media_import_path(
            runtime_id,
            NativeStringRef::from(&path),
            HostMediaAssetKind::from(kind) as i32,
            output_id.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let output_id = unsafe { output_id.assume_init() };

    decode_callback_host_required_string_ref(operation, "HostMediaImportPath.output_id", output_id)
}

/// Submit one iOS media-delete request.
fn submit_media_delete(
    runtime_id: u64,
    operation: &'static str,
    ids: &[String],
) -> RuntimeResult<u32> {
    let ids = ids.iter().map(NativeStringRef::from).collect::<Vec<_>>();
    let mut deleted_count = 0_u32;
    let status = unsafe {
        destack_host_ios_media_delete(
            runtime_id,
            NativeStringSlice::from_slice(&ids),
            &mut deleted_count,
        )
    };
    decode_callback_host_status(status, operation)?;

    Ok(deleted_count)
}
