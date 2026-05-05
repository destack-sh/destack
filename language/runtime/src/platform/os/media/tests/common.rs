use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use destack_vm as vm;
use destack_workspace::{RuntimeAppPermission, RuntimeOptions};

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeStringRef, VmAbi};
use crate::platform::fs::core as core_fs;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::platform::os::media::{MediaTestRoots, set_media_test_roots};
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    MediaAssetDescriptor, MediaAssetDescriptorVm, MediaPage, MediaPageVm, MediaQuery, MediaQueryVm,
};
use crate::platform::{NativeAbiCodec, NativeArray, VmAbiCodec, VmArray, fs};
/// Shared lock for the process-global desktop media test roots.
static TEST_MEDIA_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Monotonic nonce for media test roots.
static TEST_MEDIA_NONCE: AtomicU64 = AtomicU64::new(1);

/// Enable desktop media declarations for media tests.
pub(super) fn enable_media_declarations(options: &mut RuntimeOptions) {
    options
        .app
        .permissions
        .insert(RuntimeAppPermission::MediaRead);
    options
        .app
        .permissions
        .insert(RuntimeAppPermission::MediaWrite);
}

/// Run one desktop media harness pass with deterministic media roots installed.
pub(super) fn with_desktop_media_context<T>(
    label: &str,
    mut callback: impl for<'call> FnMut(
        HarnessContext<'call>,
        &DesktopMediaTestRootGuard,
    ) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let _guard = TEST_MEDIA_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let roots = install_desktop_media_test_roots(label);

    let mut result = None;

    with_configured_harness_context(enable_media_declarations, |context| {
        result = Some(callback(context, &roots));
        Ok(())
    });

    drop(roots);

    result.expect("desktop media harness should capture one result")
}

/// Build one media query payload for the active harness.
pub(super) fn media_query_harness_value(
    context: &mut HarnessContext<'_>,
    query: MediaQueryValue,
) -> HarnessValue<MediaQuery, MediaQueryVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let query = MediaQueryVm::from_value(&mut vm_context.write(), query)
                .expect("media query should encode");

            HarnessValue::Vm(query)
        }
        None => HarnessValue::Native(MediaQuery::from_value(context.call_context, query)),
    }
}

/// Build one harness string payload for the active lane.
pub(super) fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let value = vm::StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm media test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Build one harness string array payload for the active lane.
pub(super) fn string_array_harness_value(
    context: &mut HarnessContext<'_>,
    values: &[&str],
) -> RuntimeResult<HarnessValue<NativeArray<NativeStringRef>, VmArray<vm::StringHandle>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let mut handles = Vec::with_capacity(values.len());

            // intern one string handle per identifier
            for value in values {
                let handle = vm::StringHandle::new(
                    vm_context
                        .intern_string(value)
                        .expect("vm media test id should intern"),
                );
                handles.push(handle);
            }

            let handles = VmArray::from_values(&mut vm_context.write(), &handles)?;

            Ok(HarnessValue::Vm(handles))
        }
        None => {
            let values = values
                .iter()
                .map(|value| context.call_context.store_string(value))
                .collect::<Vec<_>>();

            Ok(HarnessValue::Native(native_array(values)))
        }
    }
}

/// Build one harness path payload for the active lane.
pub(super) fn path_harness_value(
    context: &mut HarnessContext<'_>,
    value: &Path,
) -> RuntimeResult<HarnessValue<fs::OsPath, fs::OsPathVm>> {
    let value = value.to_string_lossy();

    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let path = vm_path_from_utf8(vm_context, &value)?;

            Ok(HarnessValue::Vm(path))
        }
        None => {
            let path = core_fs::os_path_from_utf8_string(context.call_context, value.to_string());

            Ok(HarnessValue::Native(path))
        }
    }
}

/// Decode one media page payload into owned values.
pub(super) fn decode_media_page(
    context: &mut HarnessContext<'_>,
    page: HarnessValue<MediaPage, MediaPageVm>,
) -> RuntimeResult<MediaPageValue> {
    match page {
        HarnessValue::Native(page) => unsafe { MediaPage::into_value(page) },
        HarnessValue::Vm(page) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm media page decode requires one vm context")
                    as *mut vm::BindingContext<'_>)
            };

            MediaPageVm::into_value(page, &vm_context.read())
        }
    }
}

/// Decode one media descriptor payload into owned values.
pub(super) fn decode_media_descriptor(
    context: &mut HarnessContext<'_>,
    descriptor: HarnessValue<MediaAssetDescriptor, MediaAssetDescriptorVm>,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    match descriptor {
        HarnessValue::Native(descriptor) => unsafe { MediaAssetDescriptor::into_value(descriptor) },
        HarnessValue::Vm(descriptor) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm media descriptor decode requires one vm context")
                    as *mut vm::BindingContext<'_>)
            };

            MediaAssetDescriptorVm::into_value(descriptor, &vm_context.read())
        }
    }
}

/// Decode one harness string result into one owned string.
pub(super) fn decode_string(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeStringRef, vm::StringHandle>,
) -> RuntimeResult<String> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_str() }?.to_string()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm media string decode requires one vm context")
                    as *mut vm::BindingContext<'_>)
            };

            Ok(vm_context
                .string_ref(value)
                .map_err(Box::from)?
                .as_str()
                .to_string())
        }
    }
}

/// Build one deterministic desktop media root set for one test case.
pub(super) fn install_desktop_media_test_roots(label: &str) -> DesktopMediaTestRootGuard {
    let nonce = TEST_MEDIA_NONCE.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();
    let base_directory =
        std::env::temp_dir().join(format!("destack-os-media-{label}-{process_id}-{nonce}"));
    let pictures = base_directory.join("Pictures");
    let videos = base_directory.join("Videos");
    let music = base_directory.join("Music");

    // seed one deterministic desktop media library snapshot
    std::fs::create_dir_all(&pictures).expect("desktop media pictures root should create");
    std::fs::create_dir_all(&videos).expect("desktop media videos root should create");
    std::fs::create_dir_all(&music).expect("desktop media music root should create");
    std::fs::write(
        pictures.join("photo.png"),
        [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 4, 0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 252, 255, 31,
            0, 3, 3, 2, 0, 238, 217, 63, 201, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ],
    )
    .expect("desktop media photo should write");
    std::fs::write(videos.join("video.mp4"), b"mp4").expect("desktop media video should write");
    std::fs::write(music.join("song.mp3"), b"mp3").expect("desktop media song should write");

    #[cfg(unix)]
    std::fs::write(pictures.join(".hidden.jpg"), b"hidden")
        .expect("desktop media hidden asset should write");

    set_media_test_roots(Some(MediaTestRoots {
        pictures: pictures.clone(),
        videos: videos.clone(),
        music: music.clone(),
    }));

    DesktopMediaTestRootGuard { base_directory }
}

/// One scoped desktop media test-root installation.
pub(super) struct DesktopMediaTestRootGuard {
    /// The temporary root directory for this test case.
    pub(super) base_directory: PathBuf,
}

impl Drop for DesktopMediaTestRootGuard {
    /// Clear the active desktop media roots after one test.
    fn drop(&mut self) {
        set_media_test_roots(None);
        let _ = std::fs::remove_dir_all(&self.base_directory);
    }
}

/// Build one leaked native array payload for one test input vector.
pub(super) fn native_array<T>(values: Vec<T>) -> NativeArray<T> {
    let mut values = ManuallyDrop::new(values);

    NativeArray {
        data: values.as_mut_ptr(),
        len: values.len() as u32,
        capacity: values.capacity() as u32,
    }
}

/// Encode one UTF-8 path string into one VM path payload.
pub(super) fn vm_path_from_utf8(
    context: &mut vm::BindingContext<'_>,
    value: &str,
) -> RuntimeResult<fs::OsPathVm> {
    #[cfg(unix)]
    {
        let bytes =
            fs::PathBytesAbi::<VmAbi>(VmArray::from_bytes(&mut context.write(), value.as_bytes())?);
        let kind = vm::StringHandle::new(context.intern_string("bytes")?);

        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }

    #[cfg(windows)]
    {
        let units = value.encode_utf16().collect::<Vec<_>>();
        let utf16 = fs::PathUtf16Abi::<VmAbi>(VmArray::from_values(&mut context.write(), &units)?);
        let kind = vm::StringHandle::new(context.intern_string("utf16")?);

        Ok(fs::OsPathVm::OsPathUtf16(fs::OsPathUtf16Vm { kind, utf16 }))
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes =
            fs::PathBytesAbi::<VmAbi>(VmArray::from_bytes(&mut context.write(), value.as_bytes())?);
        let kind = vm::StringHandle::new(context.intern_string("bytes")?);

        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }
}
