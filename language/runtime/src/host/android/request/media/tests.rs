use crate::host::android::tests::{
    callback_test_lock, register_android_bindings, register_android_runtime,
};
use crate::host::android::{
    AndroidHostBindings, AndroidHostMediaCallbacks, destack_host_android_media_delete,
    destack_host_android_media_describe, destack_host_android_media_import_path,
    destack_host_android_media_list,
};
use crate::host::core::registry::next_host_runtime_id;
use crate::host::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::{NativeSlice, NativeStringRef};

unsafe extern "C" fn test_media_list_callback(
    _runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let query_bytes = unsafe { payload.as_slice() }.unwrap();
    let query = serde_json::from_slice::<MediaQueryValue>(query_bytes).unwrap();
    assert_eq!(query.limit, Some(10));
    assert_eq!(query.cursor, None);
    assert_eq!(query.kinds, vec![MediaAssetKind::Image]);
    assert!(!query.include_hidden);

    write_json_output(
        output,
        output_written,
        &MediaPageValue {
            assets: vec![test_media_asset_descriptor()],
            next_cursor: "next".to_string(),
            has_more: true,
        },
    )
}

unsafe extern "C" fn test_media_describe_callback(
    _runtime_id: u64,
    id: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    assert_eq!(id, "asset-1");

    write_json_output(output, output_written, &test_media_asset_descriptor())
}

unsafe extern "C" fn test_media_import_path_callback(
    _runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let path = unsafe { path.as_str() }.unwrap();
    assert_eq!(path, "/tmp/example.jpg");
    assert_eq!(kind, MediaAssetKind::Image as i32);

    write_string_output(output_id, output_written, "imported-1")
}

unsafe extern "C" fn test_media_delete_callback(
    _runtime_id: u64,
    ids: NativeSlice<u8>,
    deleted_count: *mut u32,
) -> u32 {
    let ids = unsafe { ids.as_slice() }.unwrap();
    let ids = serde_json::from_slice::<Vec<String>>(ids).unwrap();
    assert_eq!(ids, vec!["asset-1".to_string(), "asset-2".to_string()]);
    let deleted_count = unsafe { &mut *deleted_count };
    *deleted_count = 2;

    HOST_STATUS_OK
}

#[test]
fn test_media_callbacks_report_missing_runtime_registration() {
    let _lock = callback_test_lock().lock().unwrap();
    let runtime_id = next_host_runtime_id().0;
    let status = unsafe {
        destack_host_android_media_describe(
            runtime_id,
            NativeStringRef::from("asset-1"),
            empty_output(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_media_callbacks_report_unsupported_without_registered_handler() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(runtime_id, AndroidHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let status = unsafe {
        destack_host_android_media_describe(
            runtime_id,
            NativeStringRef::from("asset-1"),
            empty_output(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_media_callbacks_route_registered_handlers() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            media: AndroidHostMediaCallbacks {
                list: Some(test_media_list_callback),
                describe: Some(test_media_describe_callback),
                import_path: Some(test_media_import_path_callback),
                delete: Some(test_media_delete_callback),
            },
            ..AndroidHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let query = serde_json::to_vec(&MediaQueryValue {
        cursor: None,
        limit: Some(10),
        kinds: vec![MediaAssetKind::Image],
        include_hidden: false,
    })
    .unwrap();
    let mut page_output = vec![0_u8; 512];
    let mut page_written = 0_u32;
    let list_status = unsafe {
        destack_host_android_media_list(
            runtime_id,
            NativeSlice {
                data: query.as_ptr() as *mut u8,
                len: query.len() as u32,
            },
            NativeSlice {
                data: page_output.as_mut_ptr(),
                len: page_output.len() as u32,
            },
            &mut page_written,
        )
    };
    assert_eq!(list_status, HOST_STATUS_OK);
    let page =
        serde_json::from_slice::<MediaPageValue>(&page_output[..page_written as usize]).unwrap();
    assert_eq!(page.assets, vec![test_media_asset_descriptor()]);
    assert_eq!(page.next_cursor, "next");
    assert!(page.has_more);

    let mut describe_output = vec![0_u8; 512];
    let mut describe_written = 0_u32;
    let describe_status = unsafe {
        destack_host_android_media_describe(
            runtime_id,
            NativeStringRef::from("asset-1"),
            NativeSlice {
                data: describe_output.as_mut_ptr(),
                len: describe_output.len() as u32,
            },
            &mut describe_written,
        )
    };
    assert_eq!(describe_status, HOST_STATUS_OK);
    let descriptor = serde_json::from_slice::<MediaAssetDescriptorValue>(
        &describe_output[..describe_written as usize],
    )
    .unwrap();
    assert_eq!(descriptor, test_media_asset_descriptor());

    let mut id_output = vec![0_u8; 64];
    let mut id_written = 0_u32;
    let import_status = unsafe {
        destack_host_android_media_import_path(
            runtime_id,
            NativeStringRef::from("/tmp/example.jpg"),
            MediaAssetKind::Image as i32,
            NativeSlice {
                data: id_output.as_mut_ptr(),
                len: id_output.len() as u32,
            },
            &mut id_written,
        )
    };
    assert_eq!(import_status, HOST_STATUS_OK);
    assert_eq!(
        std::str::from_utf8(&id_output[..id_written as usize]).unwrap(),
        "imported-1"
    );

    let ids = serde_json::to_vec(&vec!["asset-1".to_string(), "asset-2".to_string()]).unwrap();
    let mut deleted_count = 0_u32;
    let delete_status = unsafe {
        destack_host_android_media_delete(
            runtime_id,
            NativeSlice {
                data: ids.as_ptr() as *mut u8,
                len: ids.len() as u32,
            },
            &mut deleted_count,
        )
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
    assert_eq!(deleted_count, 2);
}

#[test]
fn test_media_delete_requires_one_output_pointer() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            media: AndroidHostMediaCallbacks {
                delete: Some(test_media_delete_callback),
                ..AndroidHostMediaCallbacks::default()
            },
            ..AndroidHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let ids = serde_json::to_vec(&vec!["asset-1".to_string()]).unwrap();
    let call_status = unsafe {
        destack_host_android_media_delete(
            runtime_id,
            NativeSlice {
                data: ids.as_ptr() as *mut u8,
                len: ids.len() as u32,
            },
            std::ptr::null_mut(),
        )
    };
    assert_eq!(call_status, HOST_STATUS_INVALID_ARGUMENT);
}

fn empty_output() -> NativeSlice<u8> {
    NativeSlice {
        data: std::ptr::null_mut(),
        len: 0,
    }
}

fn write_json_output<T: serde::Serialize>(
    output: NativeSlice<u8>,
    output_written: *mut u32,
    value: &T,
) -> u32 {
    let bytes = serde_json::to_vec(value).unwrap();
    write_bytes_output(output, output_written, &bytes)
}

fn write_string_output(output: NativeSlice<u8>, output_written: *mut u32, value: &str) -> u32 {
    write_bytes_output(output, output_written, value.as_bytes())
}

fn write_bytes_output(output: NativeSlice<u8>, output_written: *mut u32, bytes: &[u8]) -> u32 {
    let output_written = unsafe { &mut *output_written };

    if output.len < bytes.len() as u32 {
        *output_written = bytes.len() as u32;
        return crate::host::HOST_STATUS_BUFFER_TOO_SMALL;
    }

    let output = unsafe { output.as_mut_slice() }.unwrap();
    output[..bytes.len()].copy_from_slice(bytes);
    *output_written = bytes.len() as u32;

    HOST_STATUS_OK
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
