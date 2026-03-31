use crate::host::abi::describe::{
    HostAbiRuntimeIngress, HostAbiRuntimeIngressBindingCall, HostAbiRuntimeIngressDispatchCase,
    HostAbiRuntimeIngressExpr, HostAbiRuntimeIngressLowering, HostAbiRuntimeIngressParameter,
    HostAbiRuntimeIngressParameterType, host_abi_module,
};

host_abi_module! {
    fn host_abi_module() -> "intent" {
        platforms: [ios, android];
        runtime_ingress: Some(HostAbiRuntimeIngress {
            method_name: "notifyIntentEvent",
            documentation: "Deliver one intent event into one runtime session.",
            parameters: vec![
                HostAbiRuntimeIngressParameter {
                    name: "sessionHandle",
                    ty: HostAbiRuntimeIngressParameterType::SessionHandle,
                },
                HostAbiRuntimeIngressParameter {
                    name: "event",
                    ty: HostAbiRuntimeIngressParameterType::IntentEvent,
                },
            ],
            lowering: HostAbiRuntimeIngressLowering::EnumDispatch {
                enum_parameter: "event",
                enum_field: "payload",
                cases: vec![
                    HostAbiRuntimeIngressDispatchCase {
                        variant_name: "open_url",
                        bindings: vec!["url"],
                        call: HostAbiRuntimeIngressBindingCall {
                            binding_field_name: "notify_intent_open_url",
                            arguments: vec![
                                HostAbiRuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Parameter("url"),
                            ],
                        },
                    },
                    HostAbiRuntimeIngressDispatchCase {
                        variant_name: "open_file",
                        bindings: vec!["path", "contentType"],
                        call: HostAbiRuntimeIngressBindingCall {
                            binding_field_name: "notify_intent_open_file",
                            arguments: vec![
                                HostAbiRuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Parameter("path"),
                                HostAbiRuntimeIngressExpr::ParameterIsSome("contentType"),
                                HostAbiRuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                    HostAbiRuntimeIngressDispatchCase {
                        variant_name: "share_text",
                        bindings: vec!["text", "contentType"],
                        call: HostAbiRuntimeIngressBindingCall {
                            binding_field_name: "notify_intent_share_text",
                            arguments: vec![
                                HostAbiRuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Parameter("text"),
                                HostAbiRuntimeIngressExpr::ParameterIsSome("contentType"),
                                HostAbiRuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                    HostAbiRuntimeIngressDispatchCase {
                        variant_name: "share_files",
                        bindings: vec!["paths", "contentType"],
                        call: HostAbiRuntimeIngressBindingCall {
                            binding_field_name: "notify_intent_share_files",
                            arguments: vec![
                                HostAbiRuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Parameter("paths"),
                                HostAbiRuntimeIngressExpr::ParameterIsSome("contentType"),
                                HostAbiRuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                    HostAbiRuntimeIngressDispatchCase {
                        variant_name: "custom_action",
                        bindings: vec!["action", "url", "paths", "text", "contentType"],
                        call: HostAbiRuntimeIngressBindingCall {
                            binding_field_name: "notify_intent_custom_action",
                            arguments: vec![
                                HostAbiRuntimeIngressExpr::OptionalIsSome {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Field {
                                    base: "event",
                                    field: "source",
                                },
                                HostAbiRuntimeIngressExpr::Parameter("action"),
                                HostAbiRuntimeIngressExpr::ParameterIsSome("url"),
                                HostAbiRuntimeIngressExpr::Parameter("url"),
                                HostAbiRuntimeIngressExpr::Parameter("paths"),
                                HostAbiRuntimeIngressExpr::ParameterIsSome("text"),
                                HostAbiRuntimeIngressExpr::Parameter("text"),
                                HostAbiRuntimeIngressExpr::ParameterIsSome("contentType"),
                                HostAbiRuntimeIngressExpr::Parameter("contentType"),
                            ],
                        },
                    },
                ],
            },
        });
        runtime_host: generated [ios, android];
        types: vec![];
        requests {
            /// Query whether the host can open one URL route.
            fn can_open_url(
                session_handle: session_handle,
                url: string_ref,
                is_supported: output(bool),
            ) -> host_status;

            /// Open one outbound URL route through the host.
            fn open_url(
                session_handle: session_handle,
                url: string_ref,
            ) -> host_status;

            /// Open one outbound path route through the host.
            fn open_path(
                session_handle: session_handle,
                path: string_ref,
            ) -> host_status;

            /// Share one outbound text payload through the host.
            fn share_text(
                session_handle: session_handle,
                text: string_ref,
                has_mime_type: bool,
                mime_type: string_ref,
            ) -> host_status;

            /// Share one outbound path list through the host.
            fn share_paths(
                session_handle: session_handle,
                paths: string_slice,
                has_mime_type: bool,
                mime_type: string_ref,
            ) -> host_status;
        }
        ingress {}
    }
}
