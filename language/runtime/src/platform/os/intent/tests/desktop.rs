use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeStringRef, VmAbi};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::core as core_fs;
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_harness_context};
use crate::platform::{VmArray, fs};
use crate::tests::platform::assert_runtime_error_code;

/// Build one harness string payload for the active lane.
fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let value = vm::StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm intent test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Build one harness path payload for the active lane.
fn path_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> RuntimeResult<HarnessValue<fs::OsPath, fs::OsPathVm>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let path = vm_path_from_utf8(vm_context, value)?;

            Ok(HarnessValue::Vm(path))
        }
        None => {
            let path = core_fs::os_path_from_utf8_string(context.call_context, value.to_string());

            Ok(HarnessValue::Native(path))
        }
    }
}

/// Encode one UTF-8 path string into one VM path payload.
fn vm_path_from_utf8(
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

/// Verify invalid URL inputs fail before any host launcher runs.
#[test]
fn test_intent_url_operations_reject_invalid_urls() {
    with_harness_context(|mut context| {
        let invalid_url = "not a url";
        let can_open_url = string_harness_value(&mut context, invalid_url);
        let open_url = string_harness_value(&mut context, invalid_url);

        let error = match context.destack_os_intent_can_open_url(can_open_url) {
            Ok(_) => panic!("intentCanOpenUrl should reject invalid urls"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        let error = match context.destack_os_intent_open_url(open_url) {
            Ok(_) => panic!("intentOpenUrl should reject invalid urls"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify valid URL routing queries do not fall back to argument errors.
#[test]
fn test_intent_can_open_url_accepts_absolute_urls() {
    with_harness_context(|mut context| {
        let url = string_harness_value(&mut context, "https://example.com");
        let result = context.destack_os_intent_can_open_url(url)?;

        let _ = result;
        Ok(())
    });
}

/// Verify empty path inputs fail before any host launcher runs.
#[test]
fn test_intent_open_path_rejects_empty_path() {
    with_harness_context(|mut context| {
        let path = path_harness_value(&mut context, "")?;
        let error = match context.destack_os_intent_open_path(path) {
            Ok(_) => panic!("intentOpenPath should reject empty paths"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
