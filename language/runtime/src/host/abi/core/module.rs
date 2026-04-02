use std::mem::MaybeUninit;

#[cfg(not(feature = "generator"))]
use crate::diagnostic::RuntimeResult;
#[cfg(not(feature = "generator"))]
use crate::platform::NativeAbiCodec;
#[cfg(not(feature = "generator"))]
use crate::runtime::BindingCallContext;

/// Process-global host-session handle passed through the host ABI.
#[cfg(not(feature = "generator"))]
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) type HostSessionHandle = u64;

/// One optional host ABI payload.
#[derive(Copy, Debug)]
#[repr(C)]
pub struct HostAbiOptional<T: Copy> {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped value when present.
    pub value: MaybeUninit<T>,
}

impl<T: Copy> Clone for HostAbiOptional<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Copy> HostAbiOptional<T> {
    /// Return one absent optional payload.
    pub(crate) fn none() -> Self {
        Self {
            has_value: false,
            value: MaybeUninit::uninit(),
        }
    }
}

#[cfg(not(feature = "generator"))]
impl<T> NativeAbiCodec for HostAbiOptional<T>
where
    T: NativeAbiCodec + Copy,
{
    type Value = Option<T::Value>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if !self.has_value {
            return Ok(None);
        }

        Ok(Some(unsafe { T::into_value(self.value.assume_init())? }))
    }

    fn from_value(binding: &BindingCallContext, value: Option<T::Value>) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value: MaybeUninit::new(T::from_value(binding, value)),
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
    /// One intent event payload.
    IntentEvent,
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
const INTENT_EVENT_PARAMETERS: &[HostAbiRuntimeBindingParameter] = &[
    SESSION_HANDLE_PARAMETER,
    HostAbiRuntimeBindingParameter {
        ty: HostAbiRuntimeBindingType::IntentEvent,
        name: "event",
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
        field_name: "notify_intent_event",
        type_name: "NotifyIntentEventFunction",
        documentation: "The runtime function that receives one intent event.",
        result_type: HostAbiRuntimeBindingType::RuntimeStatus,
        parameters: INTENT_EVENT_PARAMETERS,
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
