use crate::host::android::AndroidHostBindings;
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings, register_android_runtime,
};
use crate::host::{
    AndroidHostIntentCallbacks, HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND,
    HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK, destack_host_android_intent_can_open_url,
    destack_host_android_intent_open_path, destack_host_android_intent_open_url,
    destack_host_android_intent_share_paths, destack_host_android_intent_share_text,
};
use crate::runtime::{NativeStringRef, NativeStringSlice};

unsafe extern "C" fn test_can_open_url_callback(
    _runtime_id: u64,
    url: NativeStringRef,
    is_supported: *mut bool,
) -> u32 {
    let url = unsafe { url.as_str() }.unwrap();
    let is_supported = unsafe { &mut *is_supported };
    *is_supported = url == "https://example.com";

    HOST_STATUS_OK
}

unsafe extern "C" fn test_open_url_callback(_runtime_id: u64, url: NativeStringRef) -> u32 {
    let url = unsafe { url.as_str() }.unwrap();
    assert_eq!(url, "https://example.com");

    HOST_STATUS_OK
}

unsafe extern "C" fn test_open_path_callback(_runtime_id: u64, path: NativeStringRef) -> u32 {
    let path = unsafe { path.as_str() }.unwrap();
    assert_eq!(path, "/tmp/example.txt");

    HOST_STATUS_OK
}

unsafe extern "C" fn test_share_text_callback(
    _runtime_id: u64,
    text: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32 {
    let text = unsafe { text.as_str() }.unwrap();
    let mime_type = unsafe { mime_type.as_str() }.unwrap();

    assert_eq!(text, "hello");
    assert!(has_mime_type);
    assert_eq!(mime_type, "text/plain");

    HOST_STATUS_OK
}

unsafe extern "C" fn test_share_paths_callback(
    _runtime_id: u64,
    paths: NativeStringSlice,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32 {
    let paths = unsafe { paths.as_slice() }.unwrap();
    let first_path = unsafe { paths[0].as_str() }.unwrap();
    let second_path = unsafe { paths[1].as_str() }.unwrap();
    let mime_type = unsafe { mime_type.as_str() }.unwrap();

    assert_eq!(first_path, "/tmp/a.txt");
    assert_eq!(second_path, "/tmp/b.txt");
    assert!(has_mime_type);
    assert_eq!(mime_type, "text/plain");

    HOST_STATUS_OK
}

#[test]
fn test_intent_callbacks_report_missing_runtime_registration() {
    let _lock = callback_test_lock().lock().unwrap();
    let status = unsafe {
        destack_host_android_intent_open_url(41, NativeStringRef::from("https://example.com"))
    };
    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_intent_callbacks_report_unsupported_without_registered_handler() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(runtime_id, AndroidHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let status = unsafe {
        destack_host_android_intent_open_url(
            runtime_id,
            NativeStringRef::from("https://example.com"),
        )
    };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_intent_callbacks_route_registered_handlers() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            intent: AndroidHostIntentCallbacks {
                can_open_url: Some(test_can_open_url_callback),
                open_url: Some(test_open_url_callback),
                open_path: Some(test_open_path_callback),
                share_text: Some(test_share_text_callback),
                share_paths: Some(test_share_paths_callback),
            },
            ..AndroidHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let mut is_supported = false;
    let can_open_status = unsafe {
        destack_host_android_intent_can_open_url(
            runtime_id,
            NativeStringRef::from("https://example.com"),
            &mut is_supported,
        )
    };
    assert_eq!(can_open_status, HOST_STATUS_OK);
    assert!(is_supported);

    let open_url_status = unsafe {
        destack_host_android_intent_open_url(
            runtime_id,
            NativeStringRef::from("https://example.com"),
        )
    };
    assert_eq!(open_url_status, HOST_STATUS_OK);

    let open_path_status = unsafe {
        destack_host_android_intent_open_path(runtime_id, NativeStringRef::from("/tmp/example.txt"))
    };
    assert_eq!(open_path_status, HOST_STATUS_OK);

    let share_text_status = unsafe {
        destack_host_android_intent_share_text(
            runtime_id,
            NativeStringRef::from("hello"),
            true,
            NativeStringRef::from("text/plain"),
        )
    };
    assert_eq!(share_text_status, HOST_STATUS_OK);

    let path_refs = [
        NativeStringRef::from("/tmp/a.txt"),
        NativeStringRef::from("/tmp/b.txt"),
    ];
    let share_paths_status = unsafe {
        destack_host_android_intent_share_paths(
            runtime_id,
            NativeStringSlice::from_slice(&path_refs),
            true,
            NativeStringRef::from("text/plain"),
        )
    };
    assert_eq!(share_paths_status, HOST_STATUS_OK);
}

#[test]
fn test_can_open_url_requires_one_output_pointer() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            intent: AndroidHostIntentCallbacks {
                can_open_url: Some(test_can_open_url_callback),
                ..AndroidHostIntentCallbacks::default()
            },
            ..AndroidHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let call_status = unsafe {
        destack_host_android_intent_can_open_url(
            runtime_id,
            NativeStringRef::from("https://example.com"),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(call_status, HOST_STATUS_INVALID_ARGUMENT);
}
