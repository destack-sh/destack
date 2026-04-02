/// One generated host platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostPlatform {
    /// The Apple iOS host projection.
    Ios,
    /// The Android host projection.
    Android,
}

impl HostPlatform {
    /// Lower one authored host module platform into one generator platform.
    pub(crate) const fn from_abi(
        platform: destack_runtime::host::abi::describe::HostAbiModulePlatform,
    ) -> Self {
        match platform {
            destack_runtime::host::abi::describe::HostAbiModulePlatform::Ios => Self::Ios,
            destack_runtime::host::abi::describe::HostAbiModulePlatform::Android => Self::Android,
        }
    }
}

impl HostPlatform {
    /// Return the canonical path segment.
    pub(crate) const fn segment(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }

    /// Return the platform label for documentation.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Ios => "iOS",
            Self::Android => "Android",
        }
    }

    /// Return the identifier prefix.
    pub(crate) const fn prefix(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }

    /// Return the Rust type-name stem.
    pub(crate) const fn rust_name(self) -> &'static str {
        match self {
            Self::Ios => "Ios",
            Self::Android => "Android",
        }
    }
}

/// One callback-backed host lane for one platform surface.
#[derive(Clone)]
pub(crate) struct CallbackLane {
    /// The field name in the generated bindings table.
    pub field_name: &'static str,
    /// The subject used in generated documentation.
    pub subject: String,
    /// The callback type name.
    pub callback_type: String,
    /// The Rust module path segment.
    pub module_name: &'static str,
    /// The callback submodule segment.
    pub submodule_name: &'static str,
}

/// One runtime binding type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeBindingType {
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

impl RuntimeBindingType {
    /// Return the internal C bridge type name.
    pub(crate) const fn native_name(self) -> &'static str {
        match self {
            Self::RuntimeStatus => "DestackRustRuntimeStatus",
            Self::DocumentDescriptorSlice => "DestackRustDocumentDescriptorSlice",
            Self::IntentEvent => "DestackRustIntentEvent",
            Self::NotificationEvent => "DestackRustNotificationEvent",
            Self::PermissionEvent => "DestackRustPermissionEvent",
            Self::StringRef => "NativeStringRef",
            Self::StringSlice => "NativeStringSlice",
            Self::LocationSample => "DestackRustLocationSample",
            Self::TextInputEvent => "DestackRustTextInputEvent",
            Self::BackgroundEvent => "DestackRustBackgroundEvent",
            Self::U64 => "uint64_t",
            Self::Bool => "bool",
        }
    }

    /// Return the Apple public projection name.
    pub(crate) const fn apple_public_name(self) -> &'static str {
        match self {
            Self::RuntimeStatus => "DestackRustRuntimeStatus",
            Self::DocumentDescriptorSlice => "DestackRustDocumentDescriptorSlice",
            Self::IntentEvent => "DestackRustIntentEvent",
            Self::NotificationEvent => "DestackRustNotificationEvent",
            Self::PermissionEvent => "DestackRustPermissionEvent",
            Self::StringRef => "DestackRustStringRef",
            Self::StringSlice => "DestackRustStringSlice",
            Self::LocationSample => "DestackRustLocationSample",
            Self::TextInputEvent => "DestackRustTextInputEvent",
            Self::BackgroundEvent => "DestackRustBackgroundEvent",
            Self::U64 => "uint64_t",
            Self::Bool => "bool",
        }
    }

    /// Return the Android bridge projection name.
    pub(crate) const fn android_name(self) -> &'static str {
        match self {
            Self::RuntimeStatus => "RuntimeStatus",
            Self::DocumentDescriptorSlice => "HostDocumentDescriptorSlice",
            Self::IntentEvent => "HostIntentEvent",
            Self::NotificationEvent => "HostNotificationEvent",
            Self::PermissionEvent => "HostPermissionEvent",
            Self::StringRef => "NativeStringRef",
            Self::StringSlice => "NativeStringSlice",
            Self::LocationSample => "LocationSample",
            Self::TextInputEvent => "HostTextInputEvent",
            Self::BackgroundEvent => "HostBackgroundEvent",
            Self::U64 => "uint64_t",
            Self::Bool => "bool",
        }
    }
}

/// One runtime binding parameter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeBindingParameter {
    /// The projected parameter type.
    pub ty: RuntimeBindingType,
    /// The parameter name.
    pub name: &'static str,
}

/// One runtime binding function.
#[derive(Clone, Debug)]
pub(crate) struct RuntimeBindingSpec {
    /// The field name in the runtime bindings table.
    pub field_name: &'static str,
    /// The function typedef alias.
    pub type_name: &'static str,
    /// The documentation for this runtime binding.
    pub documentation: &'static str,
    /// The projected result type.
    pub result_type: RuntimeBindingType,
    /// The ordered C parameters.
    pub parameters: Vec<RuntimeBindingParameter>,
}
