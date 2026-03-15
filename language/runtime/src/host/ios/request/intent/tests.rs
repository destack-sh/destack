use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::host::core::registry::HostRegistrationGuard;
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::host::ios::unregister_ios_bindings;
use crate::host::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    IosHostBindings, IosHostIntentCallbacks, Platform, destack_host_ios_intent_can_open_url,
    destack_host_ios_intent_open_path, destack_host_ios_intent_open_url,
    destack_host_ios_intent_share_paths, destack_host_ios_intent_share_text,
    destack_host_ios_register_bindings,
};
use crate::runtime::world::RuntimeId;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Shared runtime-id allocator for iOS host tests.
static TEST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(u64::MAX - 16_384);

/// Allocate one unique runtime id for this test process.
fn next_test_runtime_id() -> RuntimeId {
    RuntimeId(TEST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Register one temporary iOS host queue and keep registration state alive.
pub(crate) fn register_ios_runtime() -> (Arc<HostQueue>, HostRegistrationGuard, u64) {
    let runtime_id = next_test_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        Some(unregister_ios_bindings),
    );
    let runtime_id = registration.runtime_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped iOS host bindings payload.
pub(crate) fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
}

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
    let status = unsafe {
        destack_host_ios_intent_open_url(41, NativeStringRef::from("https://example.com"))
    };
    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_intent_callbacks_report_unsupported_without_registered_handler() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(runtime_id, IosHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let status = unsafe {
        destack_host_ios_intent_open_url(runtime_id, NativeStringRef::from("https://example.com"))
    };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_intent_callbacks_route_registered_handlers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            intent: IosHostIntentCallbacks {
                can_open_url: Some(test_can_open_url_callback),
                open_url: Some(test_open_url_callback),
                open_path: Some(test_open_path_callback),
                share_text: Some(test_share_text_callback),
                share_paths: Some(test_share_paths_callback),
            },
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let mut is_supported = false;
    let can_open_status = unsafe {
        destack_host_ios_intent_can_open_url(
            runtime_id,
            NativeStringRef::from("https://example.com"),
            &mut is_supported,
        )
    };
    assert_eq!(can_open_status, HOST_STATUS_OK);
    assert!(is_supported);

    let open_url_status = unsafe {
        destack_host_ios_intent_open_url(runtime_id, NativeStringRef::from("https://example.com"))
    };
    assert_eq!(open_url_status, HOST_STATUS_OK);

    let open_path_status = unsafe {
        destack_host_ios_intent_open_path(runtime_id, NativeStringRef::from("/tmp/example.txt"))
    };
    assert_eq!(open_path_status, HOST_STATUS_OK);

    let share_text_status = unsafe {
        destack_host_ios_intent_share_text(
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
        destack_host_ios_intent_share_paths(
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
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            intent: IosHostIntentCallbacks {
                can_open_url: Some(test_can_open_url_callback),
                ..IosHostIntentCallbacks::default()
            },
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let call_status = unsafe {
        destack_host_ios_intent_can_open_url(
            runtime_id,
            NativeStringRef::from("https://example.com"),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(call_status, HOST_STATUS_INVALID_ARGUMENT);
}
