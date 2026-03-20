use crate::diagnostic::RuntimeResult;
use crate::platform::core::{invalid_argument, monotonic_now_ns};
use crate::platform::fs::core as core_fs;
use crate::platform::os::state::{self, IntentEventStream, IntentQueuedEvent, IntentQueuedPayload};
use crate::platform::os::{
    IntentCustomActionEventValue, IntentCustomActionPayloadValue, IntentEventMetadataValue,
    IntentEventValue, IntentOpenFileEventValue, IntentOpenFilePayloadValue, IntentOpenOptions,
    IntentOpenUrlEventValue, IntentOpenUrlPayloadValue, IntentShareFilesEventValue,
    IntentShareFilesPayloadValue, IntentShareTextEventValue, IntentShareTextPayloadValue,
};
use crate::platform::{NativeAbiCodec, fs, resource};
use crate::runtime::BindingCallContext;

use crate::host::operation::intent as host_intent;

/// Intent can-open-url binding name.
pub(super) const INTENT_CAN_OPEN_URL_OPERATION: &str = "destack.os.intent.canOpenUrl";
/// Intent open-url binding name.
pub(super) const INTENT_OPEN_URL_OPERATION: &str = "destack.os.intent.openUrl";
/// Validate one URL target and return its lowercase scheme.
fn validate_url(url: &str, operation: &'static str) -> RuntimeResult<String> {
    if url.is_empty() {
        return Err(invalid_argument("url", "url must not be empty"));
    }

    if url.contains('\0') {
        return Err(invalid_argument("url", "url must not contain nul bytes"));
    }

    if url.chars().any(char::is_whitespace) {
        return Err(invalid_argument("url", "url must not contain whitespace"));
    }

    let Some((scheme, _)) = url.split_once(':') else {
        return Err(invalid_argument(
            "url",
            format!("{operation} requires an absolute url with one explicit scheme"),
        ));
    };

    let mut scheme_chars = scheme.chars();
    let Some(first_character) = scheme_chars.next() else {
        return Err(invalid_argument("url", "url scheme must not be empty"));
    };

    if !first_character.is_ascii_alphabetic() {
        return Err(invalid_argument(
            "url",
            "url scheme must start with one ascii alphabetic character",
        ));
    }

    if !scheme_chars
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
    {
        return Err(invalid_argument(
            "url",
            "url scheme contains unsupported characters",
        ));
    }

    Ok(scheme.to_ascii_lowercase())
}

/// Validate one optional content type payload.
fn validate_content_type(content_type: Option<&str>) -> RuntimeResult<()> {
    let Some(content_type) = content_type else {
        return Ok(());
    };

    if content_type.is_empty() {
        return Err(invalid_argument(
            "contentType",
            "contentType must not be empty",
        ));
    }

    if content_type.contains('\0') {
        return Err(invalid_argument(
            "contentType",
            "contentType must not contain nul bytes",
        ));
    }

    Ok(())
}

/// Query whether the host can route one URL target.
pub(crate) fn can_open_url(binding: &BindingCallContext, url: &str) -> RuntimeResult<bool> {
    let _ = validate_url(url, INTENT_CAN_OPEN_URL_OPERATION)?;

    binding
        .host()
        .submit_operation(host_intent::can_open_url(url.to_string()))
}

/// Open one URL target through the host shell or host bridge.
pub(crate) fn open_url(binding: &BindingCallContext, url: &str) -> RuntimeResult<()> {
    let _ = validate_url(url, INTENT_OPEN_URL_OPERATION)?;

    binding
        .host()
        .submit_operation(host_intent::open_url(url.to_string()))
}

/// Open one host path target through the host shell or host bridge.
pub(crate) fn open_path(binding: &BindingCallContext, path: fs::OsPath) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_intent::open_path(path))
}

/// Share one text payload through host share routing.
pub(crate) fn share_text(
    binding: &BindingCallContext,
    text: &str,
    content_type: Option<&str>,
) -> RuntimeResult<()> {
    validate_content_type(content_type)?;

    binding.host().submit_operation(host_intent::share_text(
        text.to_string(),
        content_type.map(str::to_string),
    ))
}

/// Share one path list through host share routing.
pub(crate) fn share_paths(
    binding: &BindingCallContext,
    paths: Vec<fs::OsPath>,
    content_type: Option<&str>,
) -> RuntimeResult<()> {
    validate_content_type(content_type)?;

    if paths.is_empty() {
        return Err(invalid_argument("paths", "paths must not be empty"));
    }

    for path in &paths {
        let path = core_fs::os_path_to_utf8_string(*path, "path")?;

        if path.is_empty() {
            return Err(invalid_argument(
                "paths",
                "paths must not contain empty entries",
            ));
        }
    }

    binding.host().submit_operation(host_intent::share_paths(
        paths,
        content_type.map(str::to_string),
    ))
}

/// Open one inbound intent stream.
pub(crate) fn open(
    binding: &BindingCallContext,
    options: IntentOpenOptions,
) -> RuntimeResult<resource::IntentHandle> {
    state::intent_open(binding, options)
}

/// Close one inbound intent stream.
pub(crate) fn close(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    state::intent_close(binding, handle)
}

/// Read one queued intent event.
pub(crate) fn read_value(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
    timeout_ns: u64,
) -> RuntimeResult<IntentEventValue> {
    let (stream, event) = state::intent_read(binding, handle, timeout_ns)?;

    encode_intent_event_value(binding, &stream, event)
}

/// Poll one queued intent event without blocking.
pub(crate) fn try_read_value(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<IntentEventValue> {
    let (stream, event) = state::intent_try_read(binding, handle)?;

    encode_intent_event_value(binding, &stream, event)
}

/// Encode one queued intent event into one public value payload.
fn encode_intent_event_value(
    binding: &BindingCallContext,
    stream: &IntentEventStream,
    event: IntentQueuedEvent,
) -> RuntimeResult<IntentEventValue> {
    let metadata = encode_intent_metadata_value(stream, event.source);

    match event.payload {
        IntentQueuedPayload::OpenUrl { url } => Ok(IntentEventValue::IntentOpenUrlEvent(
            IntentOpenUrlEventValue {
                kind: "openUrl".to_string(),
                metadata,
                payload: IntentOpenUrlPayloadValue { url },
            },
        )),
        IntentQueuedPayload::OpenFile { path, content_type } => {
            let path = state::intent_path_from_utf8(binding, path);
            let path = unsafe { <fs::OsPath as NativeAbiCodec>::into_value(path)? };

            Ok(IntentEventValue::IntentOpenFileEvent(
                IntentOpenFileEventValue {
                    kind: "openFile".to_string(),
                    metadata,
                    payload: IntentOpenFilePayloadValue {
                        path,
                        mime_type: content_type,
                    },
                },
            ))
        }
        IntentQueuedPayload::ShareText { text, content_type } => Ok(
            IntentEventValue::IntentShareTextEvent(IntentShareTextEventValue {
                kind: "shareText".to_string(),
                metadata,
                payload: IntentShareTextPayloadValue {
                    text,
                    mime_type: content_type,
                },
            }),
        ),
        IntentQueuedPayload::ShareFiles {
            paths,
            content_type,
        } => {
            let mut encoded_paths = Vec::with_capacity(paths.len());

            for path in paths {
                let path = state::intent_path_from_utf8(binding, path);
                encoded_paths.push(unsafe { <fs::OsPath as NativeAbiCodec>::into_value(path)? });
            }

            Ok(IntentEventValue::IntentShareFilesEvent(
                IntentShareFilesEventValue {
                    kind: "shareFiles".to_string(),
                    metadata,
                    payload: IntentShareFilesPayloadValue {
                        paths: encoded_paths,
                        mime_type: content_type,
                    },
                },
            ))
        }
        IntentQueuedPayload::CustomAction {
            action,
            url,
            paths,
            text,
            content_type,
        } => {
            let mut encoded_paths = Vec::with_capacity(paths.len());

            for path in paths {
                let path = state::intent_path_from_utf8(binding, path);
                encoded_paths.push(unsafe { <fs::OsPath as NativeAbiCodec>::into_value(path)? });
            }

            Ok(IntentEventValue::IntentCustomActionEvent(
                IntentCustomActionEventValue {
                    kind: "customAction".to_string(),
                    metadata,
                    payload: IntentCustomActionPayloadValue {
                        action,
                        url,
                        paths: encoded_paths,
                        text,
                        mime_type: content_type,
                    },
                },
            ))
        }
    }
}

/// Encode one queued intent metadata payload into one public value.
fn encode_intent_metadata_value(
    stream: &IntentEventStream,
    source: Option<String>,
) -> IntentEventMetadataValue {
    IntentEventMetadataValue {
        timestamp_ns: monotonic_now_ns(),
        sequence: stream.next_sequence(),
        source,
    }
}
