#[cfg(not(feature = "generator"))]
use crate::diagnostic::RuntimeResult;
#[cfg(not(feature = "generator"))]
use crate::platform::NativeAbiCodec;
#[cfg(not(feature = "generator"))]
use crate::platform::abi::NativeStringRef;
#[cfg(not(feature = "generator"))]
use crate::runtime::BindingCallContext;

/// Process-global host-session handle passed through the host ABI.
#[cfg(not(feature = "generator"))]
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) type HostSessionHandle = u64;

/// One optional host string reference.
#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalStringRef {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped string reference.
    pub value: NativeStringRef,
}

#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
impl HostOptionalStringRef {
    /// Return one absent optional string reference.
    pub(crate) fn none() -> Self {
        Self {
            has_value: false,
            value: NativeStringRef::from(""),
        }
    }
}

#[cfg(not(feature = "generator"))]
impl NativeAbiCodec for HostOptionalStringRef {
    type Value = Option<String>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if !self.has_value {
            return Ok(None);
        }

        Ok(Some(unsafe {
            <NativeStringRef as NativeAbiCodec>::into_value(self.value)?
        }))
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value: <NativeStringRef as NativeAbiCodec>::from_value(binding, value),
            },
            None => Self::none(),
        }
    }
}

/// One optional host `u32`.
#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalU32 {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped integer value.
    pub value: u32,
}

#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
impl HostOptionalU32 {
    /// Return one absent optional `u32`.
    pub(crate) const fn none() -> Self {
        Self {
            has_value: false,
            value: 0,
        }
    }
}

#[cfg(not(feature = "generator"))]
impl NativeAbiCodec for HostOptionalU32 {
    type Value = Option<u32>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if self.has_value {
            Ok(Some(self.value))
        } else {
            Ok(None)
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value,
            },
            None => Self::none(),
        }
    }
}

/// One optional host `u64`.
#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalU64 {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped integer value.
    pub value: u64,
}

#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
impl HostOptionalU64 {
    /// Return one absent optional `u64`.
    pub(crate) const fn none() -> Self {
        Self {
            has_value: false,
            value: 0,
        }
    }
}

#[cfg(not(feature = "generator"))]
impl NativeAbiCodec for HostOptionalU64 {
    type Value = Option<u64>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if self.has_value {
            Ok(Some(self.value))
        } else {
            Ok(None)
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value,
            },
            None => Self::none(),
        }
    }
}

/// One optional host `i8`.
#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalI8 {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped integer value.
    pub value: i8,
}

#[cfg(not(feature = "generator"))]
#[cfg_attr(not(test), allow(dead_code))]
impl HostOptionalI8 {
    /// Return one absent optional `i8`.
    pub(crate) const fn none() -> Self {
        Self {
            has_value: false,
            value: 0,
        }
    }
}

#[cfg(not(feature = "generator"))]
impl NativeAbiCodec for HostOptionalI8 {
    type Value = Option<i8>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if self.has_value {
            Ok(Some(self.value))
        } else {
            Ok(None)
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value,
            },
            None => Self::none(),
        }
    }
}

/// One host callback ABI status code.
#[cfg(not(feature = "generator"))]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub(crate) enum HostStatus {
    /// The host callback succeeded.
    Ok = 0,
    /// The host callback is not supported.
    NotSupported = 1,
    /// The host callback rejected one invalid argument.
    InvalidArgument = 2,
    /// The host callback could not resolve one object.
    NotFound = 3,
    /// The host callback was denied permission.
    PermissionDenied = 4,
    /// The host callback output buffer was too small.
    BufferTooSmall = 5,
    /// The host callback failed generically.
    Failed = 6,
    /// The host callback would block in nonblocking mode.
    WouldBlock = 7,
}

#[cfg(not(feature = "generator"))]
#[allow(dead_code)]
impl HostStatus {
    /// Return the raw ABI status code.
    pub(crate) const fn code(self) -> u32 {
        self as u32
    }

    /// Decode one raw ABI status code when it is known.
    pub(crate) const fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Ok),
            1 => Some(Self::NotSupported),
            2 => Some(Self::InvalidArgument),
            3 => Some(Self::NotFound),
            4 => Some(Self::PermissionDenied),
            5 => Some(Self::BufferTooSmall),
            6 => Some(Self::Failed),
            7 => Some(Self::WouldBlock),
            _ => None,
        }
    }
}

/// One authored host platform for generator-only shared bridge specs.
#[cfg(feature = "generator")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAbiPlatform {
    /// The Apple iOS host surface.
    Ios,
    /// The Android host surface.
    Android,
}

/// One shared runtime binding type authored outside per-module ABI surfaces.
#[cfg(feature = "generator")]
#[derive(Clone, Copy, Debug)]
pub enum HostAbiRuntimeBindingType {
    /// One runtime status payload.
    RuntimeStatus,
    /// One document-descriptor slice payload.
    DocumentDescriptorSlice,
    /// One notification event payload.
    NotificationEvent,
    /// One permission event payload.
    PermissionEvent,
    /// One borrowed string reference.
    StringRef,
    /// One borrowed string slice.
    StringSlice,
    /// One location sample payload.
    LocationSample,
    /// One text-input event payload.
    TextInputEvent,
    /// One background event payload.
    BackgroundEvent,
    /// One `uint64_t` scalar.
    U64,
    /// One `bool` scalar.
    Bool,
}

/// One shared runtime binding parameter.
#[cfg(feature = "generator")]
#[derive(Clone, Copy, Debug)]
pub struct HostAbiRuntimeBindingParameter {
    /// The projected parameter type.
    pub ty: HostAbiRuntimeBindingType,
    /// The parameter name.
    pub name: &'static str,
}

/// One shared runtime binding function.
#[cfg(feature = "generator")]
#[derive(Clone, Copy, Debug)]
pub struct HostAbiRuntimeBindingSpec {
    /// The field name in the runtime bindings table.
    pub field_name: &'static str,
    /// The function typedef alias.
    pub type_name: &'static str,
    /// The documentation for this runtime binding.
    pub documentation: &'static str,
    /// The projected result type.
    pub result_type: HostAbiRuntimeBindingType,
    /// The ordered C parameters.
    pub parameters: &'static [HostAbiRuntimeBindingParameter],
}

#[cfg(feature = "generator")]
const IOS_BINDING_LANE_NAMES: &[&str] = &[
    "document",
    "permission",
    "text",
    "background",
    "calendar",
    "contact",
    "intent",
    "location",
    "media",
    "notification",
];

#[cfg(feature = "generator")]
const ANDROID_BINDING_LANE_NAMES: &[&str] = &[
    "document",
    "permission",
    "text",
    "background",
    "bluetooth",
    "calendar",
    "camera",
    "contact",
    "usb",
    "intent",
    "location",
    "media",
    "notification",
    "credentials",
    "crypto",
    "midi",
];

#[cfg(feature = "generator")]
const BRIDGE_LANE_NAMES: &[&str] = &[
    "document",
    "permission",
    "calendar",
    "contact",
    "intent",
    "location",
    "media",
    "notification",
    "text",
    "background",
];

#[cfg(feature = "generator")]
const SESSION_HANDLE_PARAMETER: HostAbiRuntimeBindingParameter = HostAbiRuntimeBindingParameter {
    ty: HostAbiRuntimeBindingType::U64,
    name: "session_handle",
};

#[cfg(feature = "generator")]
const DOCUMENT_RESULT_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::U64,
        name: "request_id",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::DocumentDescriptorSlice,
        name: "documents",
    },
];

#[cfg(feature = "generator")]
const NOTIFICATION_EVENT_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::NotificationEvent,
        name: "event",
    },
];

#[cfg(feature = "generator")]
const INTENT_OPEN_URL_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "url",
    },
];

#[cfg(feature = "generator")]
const INTENT_OPEN_FILE_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "path",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_mime_type",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "mime_type",
    },
];

#[cfg(feature = "generator")]
const INTENT_SHARE_TEXT_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "text",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_mime_type",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "mime_type",
    },
];

#[cfg(feature = "generator")]
const INTENT_SHARE_FILES_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringSlice,
        name: "paths",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_mime_type",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "mime_type",
    },
];

#[cfg(feature = "generator")]
const INTENT_CUSTOM_ACTION_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "source",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "action",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_url",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "url",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringSlice,
        name: "paths",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_text",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "text",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::Bool,
        name: "has_mime_type",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "mime_type",
    },
];

#[cfg(feature = "generator")]
const PERMISSION_RESULT_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::PermissionEvent,
        name: "event",
    },
];

#[cfg(feature = "generator")]
const LOCATION_SAMPLE_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::StringRef,
        name: "watch_id",
    },
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::LocationSample,
        name: "sample",
    },
];

#[cfg(feature = "generator")]
const TEXT_INPUT_STATE_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::TextInputEvent,
        name: "event",
    },
];

#[cfg(feature = "generator")]
const BACKGROUND_EVENT_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::BackgroundEvent,
        name: "event",
    },
];

#[cfg(feature = "generator")]
const HOST_RUNTIME_BINDING_SPECS: &[HostAbiRuntimeBindingSpec] = &[
    HostAbiRuntimeBindingSpec {
        field_name: "notify_document_result",
        type_name: "NotifyDocumentResultFunction",
        documentation: "The runtime function that receives one document result.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: DOCUMENT_RESULT_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_notification_event",
        type_name: "NotifyNotificationEventFunction",
        documentation: "The runtime function that receives one notification event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: NOTIFICATION_EVENT_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_intent_open_url",
        type_name: "NotifyIntentOpenUrlFunction",
        documentation: "The runtime function that receives one intent open-url event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: INTENT_OPEN_URL_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_intent_open_file",
        type_name: "NotifyIntentOpenFileFunction",
        documentation: "The runtime function that receives one intent open-file event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: INTENT_OPEN_FILE_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_intent_share_text",
        type_name: "NotifyIntentShareTextFunction",
        documentation: "The runtime function that receives one intent share-text event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: INTENT_SHARE_TEXT_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_intent_share_files",
        type_name: "NotifyIntentShareFilesFunction",
        documentation: "The runtime function that receives one intent share-files event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: INTENT_SHARE_FILES_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_intent_custom_action",
        type_name: "NotifyIntentCustomActionFunction",
        documentation: "The runtime function that receives one intent custom-action event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: INTENT_CUSTOM_ACTION_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_permission_result",
        type_name: "NotifyPermissionResultFunction",
        documentation: "The runtime function that receives one permission result.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: PERMISSION_RESULT_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_location_sample",
        type_name: "NotifyLocationSampleFunction",
        documentation: "The runtime function that receives one location sample.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: LOCATION_SAMPLE_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_text_input_state",
        type_name: "NotifyTextInputStateFunction",
        documentation: "The runtime function that receives one text-input state event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: TEXT_INPUT_STATE_PARAMETERS,
    },
    HostAbiRuntimeBindingSpec {
        field_name: "notify_background_event",
        type_name: "NotifyBackgroundEventFunction",
        documentation: "The runtime function that receives one background event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: BACKGROUND_EVENT_PARAMETERS,
    },
];

/// Return the ordered binding lanes for one authored host platform.
#[cfg(feature = "generator")]
pub fn host_abi_binding_lane_names(platform: HostAbiPlatform) -> &'static [&'static str] {
    match platform {
        HostAbiPlatform::Ios => IOS_BINDING_LANE_NAMES,
        HostAbiPlatform::Android => ANDROID_BINDING_LANE_NAMES,
    }
}

/// Return the ordered runtime-bridge lanes for one authored host platform.
#[cfg(feature = "generator")]
pub fn host_abi_bridge_lane_names(_platform: HostAbiPlatform) -> &'static [&'static str] {
    BRIDGE_LANE_NAMES
}

/// Return the authored shared runtime binding functions.
#[cfg(feature = "generator")]
pub fn host_abi_runtime_binding_specs() -> &'static [HostAbiRuntimeBindingSpec] {
    HOST_RUNTIME_BINDING_SPECS
}
