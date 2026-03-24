use std::mem::MaybeUninit;
use std::sync::{Arc, OnceLock};

use crate::host::abi::media::{
    HostMediaAssetDescriptor, HostMediaAssetKind, HostMediaPage, HostMediaQuery,
};
use crate::host::core::registry::HostSessionRegistrationGuard;
use crate::host::core::{HostQueue, HostSessionRegistry};
use crate::host::ios::abi::bindings::{IosHostBindings, destack_host_ios_register_bindings};
use crate::host::ios::abi::media::callbacks::IosHostMediaCallbacks;
use crate::host::ios::abi::media::ffi::{
    destack_host_ios_media_delete, destack_host_ios_media_describe,
    destack_host_ios_media_import_path, destack_host_ios_media_list,
};
use crate::host::ios::abi::registry::unregister_ios_bindings;
use crate::host::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    Platform,
};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{MediaAssetDescriptorValue, MediaPageValue};
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

/// Register one temporary iOS host queue and keep registration state alive.
fn register_ios_runtime() -> (Arc<HostQueue>, HostSessionRegistrationGuard, u64) {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::clone(&queue),
        Some(unregister_ios_bindings),
    );
    let runtime_id = registration.host_session_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped iOS host bindings payload.
fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
}

unsafe extern "C" fn test_media_list_callback(
    _runtime_id: u64,
    query: HostMediaQuery,
    output_page: *mut HostMediaPage,
) -> u32 {
    let kinds = unsafe { query.kinds.as_slice() }.unwrap();
    assert!(!query.has_cursor);
    assert!(query.has_limit);
    assert_eq!(query.limit, 10);
    assert_eq!(kinds, &[HostMediaAssetKind::Image]);
    assert!(!query.include_hidden);

    let assets = test_host_media_assets();
    unsafe {
        *output_page = HostMediaPage {
            assets: NativeSlice {
                data: assets.as_ptr() as *mut HostMediaAssetDescriptor,
                len: assets.len() as u32,
            },
            has_next_cursor: true,
            next_cursor: NativeStringRef::from("next"),
            has_more: true,
        };
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_media_describe_callback(
    _runtime_id: u64,
    id: NativeStringRef,
    output_descriptor: *mut HostMediaAssetDescriptor,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    assert_eq!(id, "asset-1");

    unsafe {
        *output_descriptor = test_host_media_asset_descriptor();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_media_import_path_callback(
    _runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: *mut NativeStringRef,
) -> u32 {
    let path = unsafe { path.as_str() }.unwrap();
    assert_eq!(path, "/tmp/example.jpg");
    assert_eq!(kind, HostMediaAssetKind::Image as i32);

    unsafe {
        *output_id = NativeStringRef::from("imported-1");
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_media_delete_callback(
    _runtime_id: u64,
    ids: NativeStringSlice,
    deleted_count: *mut u32,
) -> u32 {
    let ids = unsafe { ids.as_slice() }.unwrap();
    let ids = ids
        .iter()
        .map(|value| unsafe { value.as_str() }.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["asset-1", "asset-2"]);
    let deleted_count = unsafe { &mut *deleted_count };
    *deleted_count = 2;

    HOST_STATUS_OK
}

#[test]
fn test_media_callbacks_report_missing_runtime_registration() {
    let runtime_id = HostSessionRegistry::allocate_session_id().0;
    let mut output_descriptor = MaybeUninit::<HostMediaAssetDescriptor>::uninit();
    let status = unsafe {
        destack_host_ios_media_describe(
            runtime_id,
            NativeStringRef::from("asset-1"),
            output_descriptor.as_mut_ptr(),
        )
    };
    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_media_callbacks_report_unsupported_without_registered_handler() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(runtime_id, IosHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let mut output_descriptor = MaybeUninit::<HostMediaAssetDescriptor>::uninit();
    let status = unsafe {
        destack_host_ios_media_describe(
            runtime_id,
            NativeStringRef::from("asset-1"),
            output_descriptor.as_mut_ptr(),
        )
    };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_media_callbacks_route_registered_handlers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            media: IosHostMediaCallbacks {
                list: Some(test_media_list_callback),
                describe: Some(test_media_describe_callback),
                import_path: Some(test_media_import_path_callback),
                delete: Some(test_media_delete_callback),
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let kinds = [HostMediaAssetKind::Image];
    let query = HostMediaQuery {
        has_cursor: false,
        cursor: NativeStringRef::from(""),
        has_limit: true,
        limit: 10,
        kinds: NativeSlice {
            data: kinds.as_ptr() as *mut HostMediaAssetKind,
            len: kinds.len() as u32,
        },
        include_hidden: false,
    };
    let mut page_output = MaybeUninit::<HostMediaPage>::uninit();
    let list_status =
        unsafe { destack_host_ios_media_list(runtime_id, query, page_output.as_mut_ptr()) };
    assert_eq!(list_status, HOST_STATUS_OK);
    let page = unsafe { page_output.assume_init() };
    let page = decode_media_page(page);
    assert_eq!(page.assets, vec![test_media_asset_descriptor()]);
    assert_eq!(page.next_cursor, "next".to_string());
    assert!(page.has_more);

    let mut describe_output = MaybeUninit::<HostMediaAssetDescriptor>::uninit();
    let describe_status = unsafe {
        destack_host_ios_media_describe(
            runtime_id,
            NativeStringRef::from("asset-1"),
            describe_output.as_mut_ptr(),
        )
    };
    assert_eq!(describe_status, HOST_STATUS_OK);
    let descriptor = unsafe { describe_output.assume_init() };
    let descriptor = decode_media_asset_descriptor(descriptor);
    assert_eq!(descriptor, test_media_asset_descriptor());

    let mut id_output = MaybeUninit::<NativeStringRef>::uninit();
    let import_status = unsafe {
        destack_host_ios_media_import_path(
            runtime_id,
            NativeStringRef::from("/tmp/example.jpg"),
            HostMediaAssetKind::Image as i32,
            id_output.as_mut_ptr(),
        )
    };
    assert_eq!(import_status, HOST_STATUS_OK);
    let id_output = unsafe { id_output.assume_init() };
    assert_eq!(unsafe { id_output.as_str() }.unwrap(), "imported-1");

    let ids = [
        NativeStringRef::from("asset-1"),
        NativeStringRef::from("asset-2"),
    ];
    let mut deleted_count = 0_u32;
    let delete_status = unsafe {
        destack_host_ios_media_delete(
            runtime_id,
            NativeStringSlice::from_slice(&ids),
            &mut deleted_count,
        )
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
    assert_eq!(deleted_count, 2);
}

#[test]
fn test_media_delete_requires_one_output_pointer() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            media: IosHostMediaCallbacks {
                delete: Some(test_media_delete_callback),
                ..IosHostMediaCallbacks::default()
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let ids = [NativeStringRef::from("asset-1")];
    let call_status = unsafe {
        destack_host_ios_media_delete(
            runtime_id,
            NativeStringSlice::from_slice(&ids),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(call_status, HOST_STATUS_INVALID_ARGUMENT);
}

fn test_media_asset_descriptor() -> MediaAssetDescriptorValue {
    MediaAssetDescriptorValue {
        id: "asset-1".to_string(),
        uri: "content://example/asset-1".to_string(),
        filename: "example.jpg".to_string(),
        mime_type: "image/jpeg".to_string(),
        kind: MediaAssetKind::Image,
        width: 800,
        height: 600,
        duration_ms: 0,
        size_bytes: 123,
        created_unix_ns: 456,
        modified_unix_ns: 789,
    }
}

fn test_host_media_assets() -> &'static [HostMediaAssetDescriptor] {
    static ASSETS: OnceLock<Vec<HostMediaAssetDescriptor>> = OnceLock::new();

    ASSETS.get_or_init(|| vec![test_host_media_asset_descriptor()])
}

fn test_host_media_asset_descriptor() -> HostMediaAssetDescriptor {
    HostMediaAssetDescriptor {
        id: NativeStringRef::from("asset-1"),
        uri: NativeStringRef::from("content://example/asset-1"),
        filename: NativeStringRef::from("example.jpg"),
        mime_type: NativeStringRef::from("image/jpeg"),
        kind: HostMediaAssetKind::Image,
        width: 800,
        height: 600,
        duration_ms: 0,
        size_bytes: 123,
        created_unix_ns: 456,
        modified_unix_ns: 789,
    }
}

fn decode_media_page(value: HostMediaPage) -> MediaPageValue {
    value.decode("mediaList").unwrap()
}

fn decode_media_asset_descriptor(value: HostMediaAssetDescriptor) -> MediaAssetDescriptorValue {
    value.decode("mediaDescribe").unwrap()
}
