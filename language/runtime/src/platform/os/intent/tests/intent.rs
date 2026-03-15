use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::VmAbi;
use crate::platform::diagnostic::{PlatformError, PlatformErrorCode};
use crate::platform::fs::core as core_fs;
use crate::platform::os::tests::{HarnessValue, OsHarnessContext, with_harness_context};
use crate::platform::os::{IntentEvent, IntentEventVm, IntentOpenOptions, IntentOpenOptionsVm};
use crate::platform::{NativeStringRef, VmArray, fs};
use crate::tests::platform::assert_runtime_error_code;

/// Build one harness string payload for the active lane.
fn string_harness_value(
    context: &mut OsHarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
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
    context: &mut OsHarnessContext<'_>,
    value: &str,
) -> RuntimeResult<HarnessValue<fs::OsPath, fs::OsPathVm>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let path = vm_path_from_utf8(vm_context, value)?;

            Ok(HarnessValue::Vm(path))
        }
        None => {
            let path = core_fs::os_path_from_utf8_string(context.call_context, value.to_string());

            Ok(HarnessValue::Native(path))
        }
    }
}

/// Build one harness open-options payload for the active lane.
fn open_options_harness_value(
    context: &OsHarnessContext<'_>,
    value: IntentOpenOptions,
) -> HarnessValue<IntentOpenOptions, IntentOpenOptionsVm> {
    if context.vm_context.is_some() {
        return HarnessValue::Vm(value);
    }

    HarnessValue::Native(value)
}

/// One decoded plain intent event used by tests.
#[derive(Debug, PartialEq, Eq)]
enum DecodedIntentEvent {
    /// Decoded open-url payload.
    OpenUrl {
        /// Monotonic sequence number for this stream.
        sequence: u64,
        /// Host source package or process identifier when available.
        source: Option<String>,
        /// URL payload.
        url: String,
    },
    /// Decoded open-file payload.
    OpenFile {
        /// Monotonic sequence number for this stream.
        sequence: u64,
        /// Host source package or process identifier when available.
        source: Option<String>,
        /// Path payload.
        path: String,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
}

/// Encode one UTF-8 path string into one VM path payload.
fn vm_path_from_utf8(
    context: &mut vm::ExternalCallContext<'_>,
    value: &str,
) -> RuntimeResult<fs::OsPathVm> {
    #[cfg(unix)]
    {
        let bytes = fs::PathBytesAbi::<VmAbi>(VmArray::from_bytes(context, value.as_bytes())?);
        let kind = vm::StringHandle::new(context.intern_string("bytes")?);

        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }

    #[cfg(windows)]
    {
        let units = value.encode_utf16().collect::<Vec<_>>();
        let utf16 = fs::PathUtf16Abi::<VmAbi>(VmArray::from_values(context, &units)?);
        let kind = vm::StringHandle::new(context.intern_string("utf16")?);

        Ok(fs::OsPathVm::OsPathUtf16(fs::OsPathUtf16Vm { kind, utf16 }))
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = fs::PathBytesAbi::<VmAbi>(VmArray::from_bytes(context, value.as_bytes())?);
        let kind = vm::StringHandle::new(context.intern_string("bytes")?);

        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }
}

/// Decode one harness event payload into one plain test value.
fn decode_intent_event(
    context: &mut OsHarnessContext<'_>,
    value: HarnessValue<IntentEvent, IntentEventVm>,
) -> RuntimeResult<DecodedIntentEvent> {
    match value {
        HarnessValue::Native(value) => decode_native_intent_event(value),
        HarnessValue::Vm(value) => decode_vm_intent_event(context, value),
    }
}

/// Decode one native event payload into one plain test value.
fn decode_native_intent_event(value: IntentEvent) -> RuntimeResult<DecodedIntentEvent> {
    match value {
        IntentEvent::IntentOpenUrlEvent(event) => {
            let url = unsafe { event.payload.url.as_str() }?.to_string();
            let source = event
                .metadata
                .source
                .map(|source| unsafe { source.as_str().map(str::to_string) })
                .transpose()?;

            Ok(DecodedIntentEvent::OpenUrl {
                sequence: event.metadata.sequence,
                source,
                url,
            })
        }
        IntentEvent::IntentOpenFileEvent(event) => {
            let path = core_fs::os_path_to_utf8_string(event.payload.path, "path")?;
            let source = event
                .metadata
                .source
                .map(|source| unsafe { source.as_str().map(str::to_string) })
                .transpose()?;
            let mime_type = event
                .payload
                .mime_type
                .map(|mime_type| unsafe { mime_type.as_str().map(str::to_string) })
                .transpose()?;

            Ok(DecodedIntentEvent::OpenFile {
                sequence: event.metadata.sequence,
                source,
                path,
                mime_type,
            })
        }
        _ => panic!("unexpected intent event kind"),
    }
}

/// Decode one VM event payload into one plain test value.
fn decode_vm_intent_event(
    context: &mut OsHarnessContext<'_>,
    value: IntentEventVm,
) -> RuntimeResult<DecodedIntentEvent> {
    let vm_context = unsafe {
        &mut *(context
            .vm_context
            .expect("vm intent event decode requires one vm context")
            as *mut vm::ExternalCallContext<'_>)
    };

    match value {
        IntentEventVm::IntentOpenUrlEvent(event) => {
            let url = vm_context
                .string_ref(event.payload.url)
                .map_err(Box::from)?
                .as_str()
                .to_string();
            let source = match event.metadata.source {
                Some(source) => Some(
                    vm_context
                        .string_ref(source)
                        .map_err(Box::from)?
                        .as_str()
                        .to_string(),
                ),
                None => None,
            };

            Ok(DecodedIntentEvent::OpenUrl {
                sequence: event.metadata.sequence,
                source,
                url,
            })
        }
        IntentEventVm::IntentOpenFileEvent(event) => {
            let path = decode_vm_path(vm_context, event.payload.path)?;
            let source = match event.metadata.source {
                Some(source) => Some(
                    vm_context
                        .string_ref(source)
                        .map_err(Box::from)?
                        .as_str()
                        .to_string(),
                ),
                None => None,
            };
            let mime_type = match event.payload.mime_type {
                Some(mime_type) => Some(
                    vm_context
                        .string_ref(mime_type)
                        .map_err(Box::from)?
                        .as_str()
                        .to_string(),
                ),
                None => None,
            };

            Ok(DecodedIntentEvent::OpenFile {
                sequence: event.metadata.sequence,
                source,
                path,
                mime_type,
            })
        }
        _ => panic!("unexpected intent event kind"),
    }
}

/// Decode one VM path payload into one UTF-8 string.
fn decode_vm_path(
    context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<String> {
    #[cfg(unix)]
    {
        let fs::OsPathVm::OsPathBytes(path) = path else {
            panic!("expected one byte path on unix");
        };
        let bytes = path.bytes.0.read_bytes(context)?;

        Ok(String::from_utf8(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path bytes are not valid utf8",
            ))
        })?)
    }

    #[cfg(windows)]
    {
        let fs::OsPathVm::OsPathUtf16(path) = path else {
            panic!("expected one utf16 path on windows");
        };
        let units = path.utf16.0.read_values(context)?;

        Ok(String::from_utf16(&units).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path utf16 is not valid",
            ))
        })?)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let fs::OsPathVm::OsPathBytes(path) = path else {
            panic!("expected one byte path");
        };
        let bytes = path.bytes.0.read_bytes(context)?;

        Ok(String::from_utf8(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path bytes are not valid utf8",
            ))
        })?)
    }
}

/// Verify invalid URL inputs fail before any host launcher runs.
#[cfg(any(unix, windows))]
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
#[cfg(any(unix, windows))]
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
#[cfg(any(unix, windows))]
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

/// Verify one opened intent stream reports wouldBlock before any host event arrives.
#[test]
fn test_intent_stream_try_read_reports_would_block_without_events() {
    with_harness_context(|mut context| {
        let options = open_options_harness_value(
            &context,
            IntentOpenOptions {
                include_open_url: true,
                include_open_file: false,
                include_share: false,
                include_custom_action: false,
            },
        );
        let handle = context.destack_os_intent_open(options)?;

        let error = match context.destack_os_intent_try_read(handle) {
            Ok(_) => panic!("intentTryRead should report wouldBlock without one queued event"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoWouldBlock);

        context.destack_os_intent_close(handle)?;

        Ok(())
    });
}

/// Verify one stream delivers injected open-url intent events with coherent metadata.
#[test]
fn test_intent_stream_reads_injected_open_url_events() {
    with_harness_context(|mut context| {
        let options = open_options_harness_value(
            &context,
            IntentOpenOptions {
                include_open_url: true,
                include_open_file: false,
                include_share: false,
                include_custom_action: false,
            },
        );
        let handle = context.destack_os_intent_open(options)?;

        context.enqueue_intent_open_url_event(Some("com.example.source"), "https://example.com")?;

        let event = context.destack_os_intent_try_read(handle)?;
        let event = decode_intent_event(&mut context, event)?;
        assert_eq!(
            event,
            DecodedIntentEvent::OpenUrl {
                sequence: 0,
                source: Some("com.example.source".to_string()),
                url: "https://example.com".to_string(),
            },
        );

        context.destack_os_intent_close(handle)?;

        Ok(())
    });
}

/// Verify one stream filters out disabled intent event kinds.
#[test]
fn test_intent_stream_filters_disabled_event_kinds() {
    with_harness_context(|mut context| {
        let options = open_options_harness_value(
            &context,
            IntentOpenOptions {
                include_open_url: true,
                include_open_file: false,
                include_share: false,
                include_custom_action: false,
            },
        );
        let handle = context.destack_os_intent_open(options)?;

        context.enqueue_intent_open_file_event(
            Some("com.example.source"),
            "/tmp/example.txt",
            Some("text/plain"),
        )?;

        let error = match context.destack_os_intent_try_read(handle) {
            Ok(_) => panic!("intentTryRead should ignore filtered event kinds"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoWouldBlock);

        context.destack_os_intent_close(handle)?;

        Ok(())
    });
}

/// Verify closed intent stream handles reject further reads.
#[test]
fn test_intent_stream_close_invalidates_the_handle() {
    with_harness_context(|mut context| {
        let options = open_options_harness_value(
            &context,
            IntentOpenOptions {
                include_open_url: true,
                include_open_file: true,
                include_share: true,
                include_custom_action: true,
            },
        );
        let handle = context.destack_os_intent_open(options)?;
        context.destack_os_intent_close(handle)?;

        let error = match context.destack_os_intent_try_read(handle) {
            Ok(_) => panic!("intentTryRead should reject one closed handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
