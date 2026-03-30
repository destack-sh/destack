use super::HostModule;

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

/// One aggregate runtime-ingress parameter type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeIngressParameterType {
    /// One host session handle.
    SessionHandle,
    /// One intent event payload.
    IntentEvent,
}

impl RuntimeIngressParameterType {
    /// Return the Kotlin type name.
    pub(crate) const fn kotlin_name(self) -> &'static str {
        match self {
            Self::SessionHandle => "HostSessionHandle",
            Self::IntentEvent => "RuntimeHostIntentEvent",
        }
    }

    /// Return the Swift type name.
    pub(crate) const fn swift_name(self) -> &'static str {
        match self {
            Self::SessionHandle => "HostSessionHandle",
            Self::IntentEvent => "RuntimeHostIntentEvent",
        }
    }
}

/// One aggregate runtime-ingress method parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeIngressParameter {
    /// The parameter name.
    pub name: &'static str,
    /// The parameter type.
    pub ty: RuntimeIngressParameterType,
}

/// One aggregate runtime-ingress expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeIngressExpr {
    /// One direct parameter.
    Parameter(&'static str),
    /// Whether one optional parameter is present.
    ParameterIsSome(&'static str),
    /// One field projected from one parameter.
    Field {
        /// The parameter name.
        base: &'static str,
        /// The field name.
        field: &'static str,
    },
    /// Whether one optional field is present.
    OptionalIsSome {
        /// The parameter name.
        base: &'static str,
        /// The field name.
        field: &'static str,
    },
}

/// One aggregate runtime-ingress binding call.
#[derive(Clone, Debug)]
pub(crate) struct RuntimeIngressBindingCall {
    /// The resolved runtime binding.
    pub binding: RuntimeBindingSpec,
    /// The lowered binding arguments, excluding the implicit session handle.
    pub arguments: Vec<RuntimeIngressExpr>,
}

/// One aggregate runtime-ingress dispatch case.
#[derive(Clone, Debug)]
pub(crate) struct RuntimeIngressDispatchCase {
    /// The stable variant name.
    pub variant_name: &'static str,
    /// The bound payload locals for this case.
    pub bindings: Vec<&'static str>,
    /// The lowered binding call for this case.
    pub call: RuntimeIngressBindingCall,
}

/// One generated aggregate runtime-ingress lowering.
#[derive(Clone, Debug)]
pub(crate) enum RuntimeIngressLowering {
    /// One direct lowering from one authored ingress function.
    ModuleIngress,
    /// One runtime binding dispatch over one payload enum.
    EnumDispatch {
        /// The enum parameter name.
        enum_parameter: &'static str,
        /// The enum field name.
        enum_field: &'static str,
        /// The ordered dispatch cases.
        cases: Vec<RuntimeIngressDispatchCase>,
    },
}

/// One generated aggregate runtime-ingress method.
#[derive(Clone)]
pub(crate) struct RuntimeIngress {
    /// The owning host module.
    module: HostModule,
    /// The generated method name.
    method_name: &'static str,
    /// The method documentation.
    documentation: &'static str,
    /// The public method parameters.
    parameters: Vec<RuntimeIngressParameter>,
    /// The aggregate lowering shape.
    lowering: RuntimeIngressLowering,
}

impl RuntimeIngress {
    /// Build one generated aggregate runtime-ingress method from one host module.
    pub(crate) fn from_module(
        module: &HostModule,
        runtime_bindings: &[RuntimeBindingSpec],
    ) -> Option<Self> {
        if module.has_ingress() {
            let ingress = module
                .single_ingress()
                .expect("missing authored ingress while lowering runtime ingress");

            return Some(Self {
                module: module.clone(),
                method_name: ingress.name(),
                documentation: ingress.documentation(),
                parameters: Vec::new(),
                lowering: RuntimeIngressLowering::ModuleIngress,
            });
        }

        match module.name() {
            "intent" => Some(Self::intent(module, runtime_bindings)),
            _ => None,
        }
    }

    /// Return the owning host module.
    pub(crate) fn module(&self) -> &HostModule {
        &self.module
    }

    /// Return the generated method name.
    pub(crate) const fn method_name(&self) -> &'static str {
        self.method_name
    }

    /// Return the public method parameters.
    pub(crate) fn parameters(&self) -> &[RuntimeIngressParameter] {
        &self.parameters
    }

    /// Return the aggregate runtime-ingress documentation.
    pub(crate) const fn documentation(&self) -> &'static str {
        self.documentation
    }

    /// Return the aggregate lowering shape.
    pub(crate) fn lowering(&self) -> &RuntimeIngressLowering {
        &self.lowering
    }

    /// Return the canonical module name.
    pub(crate) fn module_name(&self) -> &'static str {
        self.module.name()
    }

    /// Build the shared intent runtime ingress.
    fn intent(module: &HostModule, runtime_bindings: &[RuntimeBindingSpec]) -> Self {
        Self {
            module: module.clone(),
            method_name: "notifyIntentEvent",
            documentation: "Deliver one intent event into one runtime session.",
            parameters: vec![
                RuntimeIngressParameter {
                    name: "sessionHandle",
                    ty: RuntimeIngressParameterType::SessionHandle,
                },
                RuntimeIngressParameter {
                    name: "event",
                    ty: RuntimeIngressParameterType::IntentEvent,
                },
            ],
            lowering: RuntimeIngressLowering::EnumDispatch {
                enum_parameter: "event",
                enum_field: "payload",
                cases: vec![
                    RuntimeIngressDispatchCase {
                        variant_name: "open_url",
                        bindings: vec!["url"],
                        call: RuntimeIngressBindingCall {
                            binding: runtime_binding(runtime_bindings, "notify_intent_open_url"),
                            arguments: vec![
                                RuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Parameter("url"),
                            ],
                        },
                    },
                    RuntimeIngressDispatchCase {
                        variant_name: "open_file",
                        bindings: vec!["path", "contentType"],
                        call: RuntimeIngressBindingCall {
                            binding: runtime_binding(runtime_bindings, "notify_intent_open_file"),
                            arguments: vec![
                                RuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Parameter("path"),
                                RuntimeIngressExpr::ParameterIsSome("contentType"),
                                RuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                    RuntimeIngressDispatchCase {
                        variant_name: "share_text",
                        bindings: vec!["text", "contentType"],
                        call: RuntimeIngressBindingCall {
                            binding: runtime_binding(runtime_bindings, "notify_intent_share_text"),
                            arguments: vec![
                                RuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Parameter("text"),
                                RuntimeIngressExpr::ParameterIsSome("contentType"),
                                RuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                    RuntimeIngressDispatchCase {
                        variant_name: "share_files",
                        bindings: vec!["paths", "contentType"],
                        call: RuntimeIngressBindingCall {
                            binding: runtime_binding(runtime_bindings, "notify_intent_share_files"),
                            arguments: vec![
                                RuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Parameter("paths"),
                                RuntimeIngressExpr::ParameterIsSome("contentType"),
                                RuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                    RuntimeIngressDispatchCase {
                        variant_name: "custom_action",
                        bindings: vec!["action", "url", "paths", "text", "contentType"],
                        call: RuntimeIngressBindingCall {
                            binding: runtime_binding(
                                runtime_bindings,
                                "notify_intent_custom_action",
                            ),
                            arguments: vec![
                                RuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                RuntimeIngressExpr::Parameter("action"),
                                RuntimeIngressExpr::ParameterIsSome("url"),
                                RuntimeIngressExpr::Parameter("url"),
                                RuntimeIngressExpr::Parameter("paths"),
                                RuntimeIngressExpr::ParameterIsSome("text"),
                                RuntimeIngressExpr::Parameter("text"),
                                RuntimeIngressExpr::ParameterIsSome("contentType"),
                                RuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                ],
            },
        }
    }
}

/// Resolve one runtime binding by stable field name.
fn runtime_binding(
    runtime_bindings: &[RuntimeBindingSpec],
    field_name: &'static str,
) -> RuntimeBindingSpec {
    runtime_bindings
        .iter()
        .find(|binding| binding.field_name == field_name)
        .cloned()
        .unwrap_or_else(|| panic!("missing runtime binding {}", field_name))
}
