use std::path::PathBuf;

use crate::platform::abi::NativeAbi;
use crate::platform::fs::{OsPath, PathBytesAbi, PathEncoding, PathUtf16Abi};
use crate::platform::{NativeArray, NativeSlice};
use crate::tests::runtime::TestRuntime;

/// Harness interface for filesystem bindings.
pub(crate) trait FsHarness {
    /// Return the runtime backing this harness.
    fn runtime(&self) -> &TestRuntime;

    /// Run a native or VM call context around the callback.
    fn with_context<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R;
}

/// Native filesystem harness backed by native bindings.
pub(crate) struct NativeFsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeFsHarness {
    /// Create a new native filesystem harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

impl FsHarness for NativeFsHarness {
    fn runtime(&self) -> &TestRuntime {
        &self.runtime
    }

    fn with_context<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.runtime.with_native_call_context(|_| callback())
    }
}

/// Run a test with the native filesystem harness.
pub(crate) fn with_native_harness<F, R>(callback: F) -> R
where
    F: FnOnce(&NativeFsHarness) -> R,
{
    let harness = NativeFsHarness::new();
    callback(&harness)
}

/// Create a unique temp directory for a test.
pub(crate) fn temp_dir(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();

    std::env::temp_dir().join(format!("destack_runtime_{label}_{nonce}"))
}

/// Build a NativeSlice from a mutable byte buffer.
pub(crate) fn native_slice_mut(buffer: &mut [u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
    }
}

/// Build a NativeSlice from an immutable byte buffer.
pub(crate) fn native_slice(buffer: &[u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_ptr() as *mut u8,
        len: buffer.len() as u32,
    }
}

/// Build a NativeArray from a byte buffer.
pub(crate) fn native_array(buffer: &mut [u8]) -> NativeArray<u8> {
    NativeArray {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
        capacity: buffer.len() as u32,
    }
}

/// Build a NativeArray from a UTF-16 buffer.
#[cfg(windows)]
pub(crate) fn native_array_u16(buffer: &mut [u16]) -> NativeArray<u16> {
    NativeArray {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
        capacity: buffer.len() as u32,
    }
}

/// Build an empty UTF-16 ABI path.
fn empty_path_utf16() -> PathUtf16Abi<NativeAbi> {
    PathUtf16Abi(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an OsPath ABI value from a path.
#[cfg(unix)]
pub(crate) fn path_bytes(path: &std::path::Path) -> (Vec<u8>, OsPath) {
    use std::os::unix::ffi::OsStrExt;

    let mut bytes = path.as_os_str().as_bytes().to_vec();
    let path = OsPath {
        encoding: PathEncoding::Bytes,
        bytes: PathBytesAbi::<NativeAbi>(native_array(&mut bytes)),
        utf16: empty_path_utf16(),
    };
    (bytes, path)
}

/// Build an OsPath ABI value from a path.
#[cfg(windows)]
pub(crate) fn path_bytes(path: &std::path::Path) -> (Vec<u8>, OsPath) {
    let value = path.to_str().expect("path must be utf8 for windows tests");
    let mut bytes = value.as_bytes().to_vec();
    let path = OsPath {
        encoding: PathEncoding::Bytes,
        bytes: PathBytesAbi::<NativeAbi>(native_array(&mut bytes)),
        utf16: empty_path_utf16(),
    };
    (bytes, path)
}

/// Build an OsPath ABI value from a path.
#[cfg(not(any(unix, windows)))]
pub(crate) fn path_bytes(_path: &std::path::Path) -> (Vec<u8>, OsPath) {
    unreachable!("unsupported platform for fs tests");
}

/// Build an OsPath ABI value with UTF-16 encoding from a path.
#[cfg(windows)]
pub(crate) fn path_utf16(path: &std::path::Path) -> (Vec<u16>, OsPath) {
    use std::os::windows::ffi::OsStrExt;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let path = OsPath {
        encoding: PathEncoding::Utf16,
        bytes: PathBytesAbi::<NativeAbi>(NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }),
        utf16: PathUtf16Abi::<NativeAbi>(native_array_u16(&mut wide)),
    };
    (wide, path)
}

/// Decode an OsPath ABI value into a string for test assertions.
pub(crate) fn os_path_string(path: &OsPath) -> String {
    // decode bytes paths
    if path.encoding == PathEncoding::Bytes {
        let bytes = unsafe { path.bytes.0.as_slice() }.expect("bytes path should be valid");
        return String::from_utf8_lossy(bytes).into_owned();
    }

    // decode utf16 paths
    let code_units = unsafe { path.utf16.0.as_slice() }.expect("utf16 path should be valid");
    String::from_utf16_lossy(code_units)
}
