use std::ffi::{CStr, CString};
use std::ptr;

use crate::{
    DestackDocument, DestackDocumentKind, DestackEdits, DestackError, DestackFormatRequest,
    DestackModule, DestackSession, DestackSessionFileArray, DestackSource, DestackStatus,
    destack_edits_add_set_text, destack_error_destroy, destack_error_message,
    destack_session_destroy, destack_session_file_array_destroy, destack_session_files,
    destack_session_open, destack_source_add_text, destack_source_destroy, destack_source_memory,
};

pub(super) struct TestSession {
    pub(super) inner: *mut DestackSession,
}

impl TestSession {
    pub(super) fn open(files: &[(&str, &str)]) -> Self {
        let mut error = ptr::null_mut();
        let mut source = ptr::null_mut();
        let root = test_root();
        let root = CString::new(root.to_string_lossy().into_owned()).unwrap();

        // create memory source
        let status = unsafe { destack_source_memory(root.as_ptr(), &mut source, &mut error) };
        assert_ok(status, error);

        // seed explicit test files
        for (path, text) in files {
            add_source_text(source, path, text);
        }

        let mut inner = ptr::null_mut();

        // open session through the C ABI
        let status = unsafe { destack_session_open(source, &mut inner, &mut error) };
        assert_ok(status, error);

        // release source after session opening
        unsafe {
            destack_source_destroy(source);
        }

        Self { inner }
    }

    pub(super) fn paths(&self) -> Vec<String> {
        let mut error = ptr::null_mut();
        let mut files = DestackSessionFileArray {
            ptr: ptr::null_mut(),
            len: 0,
        };

        // list source files through the C ABI
        let status = unsafe { destack_session_files(self.inner, &mut files, &mut error) };
        assert_ok(status, error);

        let mut paths = Vec::with_capacity(files.len);

        // copy returned path strings
        for index in 0..files.len {
            let file = unsafe { files.ptr.add(index).as_ref() }.unwrap();
            let path = unsafe { CStr::from_ptr(file.path) };

            paths.push(path.to_string_lossy().into_owned());
        }

        // release returned file array
        unsafe {
            destack_session_file_array_destroy(files);
        }

        paths
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        // release session handle
        unsafe {
            destack_session_destroy(self.inner);
        }
    }
}

fn add_source_text(source: *mut DestackSource, path: &str, text: &str) {
    let mut error = ptr::null_mut();
    let path = CString::new(path).unwrap();
    let text = CString::new(text).unwrap();

    // add text file through source API
    let status =
        unsafe { destack_source_add_text(source, path.as_ptr(), text.as_ptr(), &mut error) };

    assert_ok(status, error);
}

pub(super) fn add_edit_text(edits: *mut DestackEdits, path: &str, text: &str) {
    let mut error = ptr::null_mut();
    let path = CString::new(path).unwrap();
    let text = CString::new(text).unwrap();

    // add text replacement through edit API
    let status =
        unsafe { destack_edits_add_set_text(edits, path.as_ptr(), text.as_ptr(), &mut error) };

    assert_ok(status, error);
}

pub(super) fn format_text_request(path: &str, text: &str) -> DestackFormatRequest {
    let path = CString::new(path).unwrap().into_raw();
    let text = CString::new(text).unwrap().into_raw();

    // transfer strings into generated request ownership
    DestackFormatRequest {
        document: DestackDocument {
            kind: DestackDocumentKind::Text,
            module_module: DestackModule::empty(),
            path,
            text_text: text,
        },
    }
}

pub(super) fn assert_ok(status: DestackStatus, error: *mut DestackError) {
    if status == DestackStatus::Ok {
        return;
    }

    // surface C ABI error message
    let message = unsafe { destack_error_message(error) };
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    let message = message.into_owned();

    // release error handle before panic
    unsafe {
        destack_error_destroy(error);
    }

    panic!("C ABI call failed: {message}");
}

pub(super) fn string(value: *const std::ffi::c_char) -> String {
    let value = unsafe { CStr::from_ptr(value) };

    value.to_string_lossy().into_owned()
}

fn test_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join("destack-bridge-capi-tests");

    // ensure repository cache has a writable parent
    std::fs::create_dir_all(&root).unwrap();

    root
}
