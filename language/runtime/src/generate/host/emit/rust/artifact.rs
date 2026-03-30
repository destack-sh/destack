use std::collections::BTreeSet;

use super::docs::{push_indented_rust_doc_comment, push_rust_doc_comment};
use super::name::pascal_case;
use crate::host::model::{HostArtifact, HostModule, HostPlatform};
use crate::platform::model::WorkspaceLayout;
use destack_runtime::host::abi::describe::{
    HostAbiEnumRepresentation, HostAbiFunction, HostAbiModule, HostAbiNamedType,
    HostAbiNamedTypeDefinition, HostAbiRustAvailability, HostAbiRustCapabilityAction,
    HostAbiRustCapabilityProbe, HostAbiRustStatus, HostAbiSelectorControl, HostAbiType,
};

/// Render the generated Rust iOS ABI files for one module.
pub(crate) fn render_ios_files(layout: &WorkspaceLayout, module: &HostModule) -> Vec<HostArtifact> {
    render_platform_files(layout, module.abi(), HostPlatform::Ios)
}

/// Render the generated Rust ABI runtime files for one module.
pub(crate) fn render_bridge_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    let contents = match module.name() {
        "background" => Some(render_background_bridge_file()),
        "notification" => Some(render_notification_bridge_file()),
        _ => None,
    };

    if let Some(contents) = contents {
        return vec![HostArtifact {
            path: layout
                .language_root
                .join(format!("runtime/src/host/abi/{}/runtime.rs", module.name())),
            contents,
        }];
    }

    build_rust_bridge_file_specs(module)
        .into_iter()
        .map(|spec| HostArtifact {
            path: layout.language_root.join(format!(
                "runtime/src/host/abi/{}/{}",
                module.name(),
                spec.file_name
            )),
            contents: render_rust_bridge_file_spec(&spec),
        })
        .collect()
}

/// Render the generated Rust bridge file for background helpers.
fn render_background_bridge_file() -> String {
    let imports = [
        "use crate::diagnostic::RuntimeResult;",
        "use crate::platform::NativeAbiCodec;",
        "#[cfg(any(target_os = \"android\", target_os = \"ios\"))] use crate::platform::NativeArray;",
        "use crate::platform::abi::NativeStringRef;",
        "use crate::platform::os::abi_generated::{BackgroundConflictPolicyValue, BackgroundEventMetadataValue, BackgroundEventValue, BackgroundNetworkRequirementValue, BackgroundTaskDescriptorValue, BackgroundTaskExpiredEventValue, BackgroundTaskReadyEventValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue, BackgroundTriggerKindValue};",
        "#[cfg(any(target_os = \"android\", target_os = \"ios\"))] use crate::platform::os::abi_generated::{BackgroundStatusValue, BackgroundTaskOptionsValue, BackgroundTaskResultValue};",
        "use crate::runtime::BindingCallContext;",
        "use crate::host::abi::background::{HostBackgroundConflictPolicy, HostBackgroundEvent, HostBackgroundEventKind, HostBackgroundEventMetadata, HostBackgroundNetworkRequirement, HostBackgroundTaskDescriptor, HostBackgroundTaskSchedule, HostBackgroundTaskScheduleKind, HostBackgroundTriggerKind};",
        "#[cfg(any(target_os = \"android\", target_os = \"ios\"))] use crate::host::abi::background::{HostBackgroundStatus, HostBackgroundTaskOptions, HostBackgroundTaskResult};",
    ];
    let payload = RustBridgePayloadSpec {
        documentation: "One owned host background task-options payload.",
        struct_name: "HostBackgroundTaskOptionsPayload",
        storage_fields: vec![RustBridgeStorageField {
            documentation: "The owned identifier storage.",
            name: "_identifier_storage",
            ty: "String",
        }],
        abi_documentation: "The borrowed ABI payload.",
        abi_type: "HostBackgroundTaskOptions",
        constructor_documentation: "Build one owned host background task-options payload.",
        constructor_name: "new",
        constructor_parameters: vec![RustBridgeParameter {
            name: "options",
            ty: "&BackgroundTaskOptionsValue",
        }],
        local_bindings: vec![RustBridgeLocal {
            name: "identifier_storage",
            value: RustBridgeExpr::Path("options.identifier.clone()"),
        }],
        abi_initializer: RustBridgeExpr::StructInit {
            ty: "HostBackgroundTaskOptions",
            fields: vec![
                RustBridgeFieldInitializer {
                    name: "identifier",
                    value: RustBridgeExpr::Path(
                        "NativeStringRef::from(identifier_storage.as_str())",
                    ),
                },
                RustBridgeFieldInitializer {
                    name: "trigger",
                    value: RustBridgeExpr::Path("encode_trigger(options.trigger)"),
                },
                RustBridgeFieldInitializer {
                    name: "schedule",
                    value: RustBridgeExpr::Path("encode_schedule(options.schedule)"),
                },
                RustBridgeFieldInitializer {
                    name: "network",
                    value: RustBridgeExpr::Path("encode_network_requirement(options.network)"),
                },
                RustBridgeFieldInitializer {
                    name: "requires_charging",
                    value: RustBridgeExpr::Path("options.requires_charging"),
                },
                RustBridgeFieldInitializer {
                    name: "requires_idle",
                    value: RustBridgeExpr::Path("options.requires_idle"),
                },
                RustBridgeFieldInitializer {
                    name: "conflict_policy",
                    value: RustBridgeExpr::Path("encode_conflict_policy(options.conflict_policy)"),
                },
            ],
        },
        accessor_documentation: "Return the ABI task-options payload.",
    };
    let mut output = String::new();

    render_rust_bridge_file_header(&mut output, &imports);
    render_background_descriptor_codec_impl(&mut output);
    output.push('\n');
    render_cfg_rust_bridge_payload(
        &mut output,
        "any(target_os = \"android\", target_os = \"ios\")",
        &payload,
    );
    output.push('\n');
    render_cfg_rust_bridge_function(
        &mut output,
        "any(target_os = \"android\", target_os = \"ios\")",
        &RustBridgeFunctionSpec {
            documentation: "Decode one host background status into one runtime value.",
            qualifiers: vec!["pub(crate)"],
            name: "decode_status",
            parameters: vec![RustBridgeParameter {
                name: "status",
                ty: "HostBackgroundStatus",
            }],
            return_type: "BackgroundStatusValue",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "status",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundStatus::Unavailable",
                        value: RustBridgeExpr::Path("BackgroundStatusValue::Unavailable"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundStatus::Restricted",
                        value: RustBridgeExpr::Path("BackgroundStatusValue::Restricted"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundStatus::Available",
                        value: RustBridgeExpr::Path("BackgroundStatusValue::Available"),
                    },
                ],
            })],
        },
    );
    output.push('\n');
    render_cfg_rust_bridge_function(
        &mut output,
        "any(target_os = \"android\", target_os = \"ios\")",
        &RustBridgeFunctionSpec {
            documentation: "Decode one host background descriptor array into runtime values.",
            qualifiers: vec!["pub(crate)", "unsafe"],
            name: "decode_descriptors",
            parameters: vec![RustBridgeParameter {
                name: "descriptors",
                ty: "NativeArray<HostBackgroundTaskDescriptor>",
            }],
            return_type: "RuntimeResult<Vec<BackgroundTaskDescriptorValue>>",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "unsafe { <NativeArray<HostBackgroundTaskDescriptor> as NativeAbiCodec>::into_value(descriptors) }",
            ))],
        },
    );
    output.push('\n');
    render_cfg_rust_bridge_function(
        &mut output,
        "any(target_os = \"android\", target_os = \"ios\")",
        &RustBridgeFunctionSpec {
            documentation: "Decode one host background descriptor into one runtime value.",
            qualifiers: vec!["pub(crate)"],
            name: "decode_descriptor",
            parameters: vec![RustBridgeParameter {
                name: "descriptor",
                ty: "HostBackgroundTaskDescriptor",
            }],
            return_type: "RuntimeResult<BackgroundTaskDescriptorValue>",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "unsafe { <HostBackgroundTaskDescriptor as NativeAbiCodec>::into_value(descriptor) }",
            ))],
        },
    );
    output.push('\n');
    render_cfg_rust_bridge_function(
        &mut output,
        "any(target_os = \"android\", target_os = \"ios\")",
        &RustBridgeFunctionSpec {
            documentation: "Encode one runtime background result into one host payload.",
            qualifiers: vec!["pub(crate)"],
            name: "encode_result",
            parameters: vec![RustBridgeParameter {
                name: "result",
                ty: "BackgroundTaskResultValue",
            }],
            return_type: "HostBackgroundTaskResult",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "result",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "BackgroundTaskResultValue::Success",
                        value: RustBridgeExpr::Path("HostBackgroundTaskResult::Success"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundTaskResultValue::Retry",
                        value: RustBridgeExpr::Path("HostBackgroundTaskResult::Retry"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundTaskResultValue::Failure",
                        value: RustBridgeExpr::Path("HostBackgroundTaskResult::Failure"),
                    },
                ],
            })],
        },
    );
    output.push('\n');
    render_rust_bridge_function(
        &mut output,
        &RustBridgeFunctionSpec {
            documentation: "Decode one host background event into one runtime value.",
            qualifiers: vec!["pub(crate)"],
            name: "decode_event",
            parameters: vec![RustBridgeParameter {
                name: "event",
                ty: "HostBackgroundEvent",
            }],
            return_type: "RuntimeResult<BackgroundEventValue>",
            body: vec![
                RustBridgeStatement::Let {
                    name: "metadata",
                    value: RustBridgeExpr::Path("decode_event_metadata(event.metadata)?"),
                },
                RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "event.kind",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostBackgroundEventKind::TaskReady",
                            value: RustBridgeExpr::Path(
                                "Ok(BackgroundEventValue::BackgroundTaskReadyEvent(BackgroundTaskReadyEventValue { kind: \"taskReady\".to_string(), metadata }))",
                            ),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostBackgroundEventKind::TaskExpired",
                            value: RustBridgeExpr::Path(
                                "Ok(BackgroundEventValue::BackgroundTaskExpiredEvent(BackgroundTaskExpiredEventValue { kind: \"taskExpired\".to_string(), metadata }))",
                            ),
                        },
                    ],
                }),
            ],
        },
    );
    output.push('\n');
    for function in [
        RustBridgeFunctionSpec {
            documentation: "Encode one runtime background trigger into one host payload.",
            qualifiers: Vec::new(),
            name: "encode_trigger",
            parameters: vec![RustBridgeParameter {
                name: "trigger",
                ty: "BackgroundTriggerKindValue",
            }],
            return_type: "HostBackgroundTriggerKind",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "trigger",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "BackgroundTriggerKindValue::AppRefresh",
                        value: RustBridgeExpr::Path("HostBackgroundTriggerKind::AppRefresh"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundTriggerKindValue::Processing",
                        value: RustBridgeExpr::Path("HostBackgroundTriggerKind::Processing"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host background trigger into one runtime value.",
            qualifiers: Vec::new(),
            name: "decode_trigger",
            parameters: vec![RustBridgeParameter {
                name: "trigger",
                ty: "HostBackgroundTriggerKind",
            }],
            return_type: "BackgroundTriggerKindValue",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "trigger",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundTriggerKind::AppRefresh",
                        value: RustBridgeExpr::Path("BackgroundTriggerKindValue::AppRefresh"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundTriggerKind::Processing",
                        value: RustBridgeExpr::Path("BackgroundTriggerKindValue::Processing"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Encode one runtime background schedule into one host payload.",
            qualifiers: Vec::new(),
            name: "encode_schedule",
            parameters: vec![RustBridgeParameter {
                name: "schedule",
                ty: "BackgroundTaskScheduleValue",
            }],
            return_type: "HostBackgroundTaskSchedule",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "HostBackgroundTaskSchedule { kind: encode_schedule_kind(schedule.kind), has_earliest_begin_unix_ns: schedule.earliest_begin_unix_ns.is_some(), earliest_begin_unix_ns: schedule.earliest_begin_unix_ns.unwrap_or(0), has_repeat_interval_ns: schedule.repeat_interval_ns.is_some(), repeat_interval_ns: schedule.repeat_interval_ns.unwrap_or(0) }",
            ))],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host background schedule into one runtime value.",
            qualifiers: Vec::new(),
            name: "decode_schedule",
            parameters: vec![RustBridgeParameter {
                name: "schedule",
                ty: "HostBackgroundTaskSchedule",
            }],
            return_type: "BackgroundTaskScheduleValue",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "BackgroundTaskScheduleValue { kind: decode_schedule_kind(schedule.kind), earliest_begin_unix_ns: schedule.has_earliest_begin_unix_ns.then_some(schedule.earliest_begin_unix_ns), repeat_interval_ns: schedule.has_repeat_interval_ns.then_some(schedule.repeat_interval_ns) }",
            ))],
        },
        RustBridgeFunctionSpec {
            documentation: "Encode one runtime background schedule kind into one host payload.",
            qualifiers: Vec::new(),
            name: "encode_schedule_kind",
            parameters: vec![RustBridgeParameter {
                name: "kind",
                ty: "BackgroundTaskScheduleKindValue",
            }],
            return_type: "HostBackgroundTaskScheduleKind",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "kind",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "BackgroundTaskScheduleKindValue::Once",
                        value: RustBridgeExpr::Path("HostBackgroundTaskScheduleKind::Once"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundTaskScheduleKindValue::Recurring",
                        value: RustBridgeExpr::Path("HostBackgroundTaskScheduleKind::Recurring"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host background schedule kind into one runtime value.",
            qualifiers: Vec::new(),
            name: "decode_schedule_kind",
            parameters: vec![RustBridgeParameter {
                name: "kind",
                ty: "HostBackgroundTaskScheduleKind",
            }],
            return_type: "BackgroundTaskScheduleKindValue",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "kind",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundTaskScheduleKind::Once",
                        value: RustBridgeExpr::Path("BackgroundTaskScheduleKindValue::Once"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundTaskScheduleKind::Recurring",
                        value: RustBridgeExpr::Path("BackgroundTaskScheduleKindValue::Recurring"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Encode one runtime background network requirement into one host payload.",
            qualifiers: Vec::new(),
            name: "encode_network_requirement",
            parameters: vec![RustBridgeParameter {
                name: "network",
                ty: "BackgroundNetworkRequirementValue",
            }],
            return_type: "HostBackgroundNetworkRequirement",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "network",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "BackgroundNetworkRequirementValue::None",
                        value: RustBridgeExpr::Path("HostBackgroundNetworkRequirement::None"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundNetworkRequirementValue::Connected",
                        value: RustBridgeExpr::Path("HostBackgroundNetworkRequirement::Connected"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundNetworkRequirementValue::Unmetered",
                        value: RustBridgeExpr::Path("HostBackgroundNetworkRequirement::Unmetered"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host background network requirement into one runtime value.",
            qualifiers: Vec::new(),
            name: "decode_network_requirement",
            parameters: vec![RustBridgeParameter {
                name: "network",
                ty: "HostBackgroundNetworkRequirement",
            }],
            return_type: "BackgroundNetworkRequirementValue",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "network",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundNetworkRequirement::None",
                        value: RustBridgeExpr::Path("BackgroundNetworkRequirementValue::None"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundNetworkRequirement::Connected",
                        value: RustBridgeExpr::Path("BackgroundNetworkRequirementValue::Connected"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundNetworkRequirement::Unmetered",
                        value: RustBridgeExpr::Path("BackgroundNetworkRequirementValue::Unmetered"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Encode one runtime background conflict policy into one host payload.",
            qualifiers: Vec::new(),
            name: "encode_conflict_policy",
            parameters: vec![RustBridgeParameter {
                name: "policy",
                ty: "BackgroundConflictPolicyValue",
            }],
            return_type: "HostBackgroundConflictPolicy",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "policy",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "BackgroundConflictPolicyValue::Replace",
                        value: RustBridgeExpr::Path("HostBackgroundConflictPolicy::Replace"),
                    },
                    RustBridgeMatchArm {
                        pattern: "BackgroundConflictPolicyValue::Keep",
                        value: RustBridgeExpr::Path("HostBackgroundConflictPolicy::Keep"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host background conflict policy into one runtime value.",
            qualifiers: Vec::new(),
            name: "decode_conflict_policy",
            parameters: vec![RustBridgeParameter {
                name: "policy",
                ty: "HostBackgroundConflictPolicy",
            }],
            return_type: "BackgroundConflictPolicyValue",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                scrutinee: "policy",
                arms: vec![
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundConflictPolicy::Replace",
                        value: RustBridgeExpr::Path("BackgroundConflictPolicyValue::Replace"),
                    },
                    RustBridgeMatchArm {
                        pattern: "HostBackgroundConflictPolicy::Keep",
                        value: RustBridgeExpr::Path("BackgroundConflictPolicyValue::Keep"),
                    },
                ],
            })],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host background event metadata payload.",
            qualifiers: Vec::new(),
            name: "decode_event_metadata",
            parameters: vec![RustBridgeParameter {
                name: "metadata",
                ty: "HostBackgroundEventMetadata",
            }],
            return_type: "RuntimeResult<BackgroundEventMetadataValue>",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "Ok(BackgroundEventMetadataValue { timestamp_ns: metadata.timestamp_ns, sequence: metadata.sequence, identifier: decode_string(metadata.identifier)?, execution_id: decode_string(metadata.execution_id)?, deadline_unix_ns: metadata.deadline_unix_ns })",
            ))],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one host string reference into one owned string.",
            qualifiers: Vec::new(),
            name: "decode_string",
            parameters: vec![RustBridgeParameter {
                name: "value",
                ty: "NativeStringRef",
            }],
            return_type: "RuntimeResult<String>",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "unsafe { <NativeStringRef as NativeAbiCodec>::into_value(value) }",
            ))],
        },
    ] {
        render_rust_bridge_function(&mut output, &function);
        output.push('\n');
    }

    output
}

/// Render the generated Rust bridge file for notification helpers.
fn render_notification_bridge_file() -> String {
    let imports = [
        "use crate::diagnostic::RuntimeResult;",
        "use crate::host::core::error::invalid_argument_value;",
        "#[cfg(any(target_os = \"android\", target_os = \"ios\"))] use crate::host::core::error::not_supported;",
        "#[cfg(any(target_os = \"android\", target_os = \"ios\"))] use crate::platform::NativeAbiCodec;",
        "use crate::platform::abi::NativeStringRef;",
        "use crate::platform::os::abi_generated::{NotificationDeliveredEventValue, NotificationDismissedEventValue, NotificationEventMetadataValue, NotificationEventValue, NotificationImmediateTriggerValue, NotificationInteractedEventValue, NotificationInteractedPayloadValue, NotificationPriority, NotificationRequestValue, NotificationTriggerValue};",
        "#[cfg(any(target_os = \"android\", target_os = \"ios\"))] use crate::runtime::BindingCallContext;",
        "use crate::host::abi::notification::{HostNotificationEvent, HostNotificationEventKind, HostNotificationRequest};",
    ];
    let mut output = String::new();

    render_rust_bridge_file_header(&mut output, &imports);
    render_cfg_rust_bridge_function(
        &mut output,
        "any(target_os = \"android\", target_os = \"ios\")",
        &RustBridgeFunctionSpec {
            documentation: "Encode one mobile notification request payload for the host ABI.",
            qualifiers: vec!["pub(crate)"],
            name: "encode_notification_request",
            parameters: vec![
                RustBridgeParameter {
                    name: "binding",
                    ty: "&BindingCallContext",
                },
                RustBridgeParameter {
                    name: "request",
                    ty: "&NotificationRequestValue",
                },
                RustBridgeParameter {
                    name: "operation",
                    ty: "&'static str",
                },
            ],
            return_type: "RuntimeResult<HostNotificationRequest>",
            body: vec![
                RustBridgeStatement::If {
                    condition: RustBridgeExpr::Path("request.tag.is_empty()"),
                    then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Err(invalid_argument_value(\"request.tag\", \"mobile notification requests require one non-empty tag\"))",
                    ))],
                },
                RustBridgeStatement::If {
                    condition: RustBridgeExpr::Path(
                        "request.subtitle.is_some() || request.channel_id.is_some() || request.badge_count.is_some() || request.sound.is_some() || request.category_id.is_some() || request.thread_id.is_some() || request.action_id.is_some()",
                    ),
                    then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Err(not_supported(operation))",
                    ))],
                },
                RustBridgeStatement::If {
                    condition: RustBridgeExpr::Path(
                        "!matches!(request.trigger, NotificationTriggerValue::NotificationImmediateTrigger(_))",
                    ),
                    then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Err(not_supported(operation))",
                    ))],
                },
                RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(HostNotificationRequest { identifier: NativeStringRef::from_value(binding, request.tag.clone()), title: NativeStringRef::from_value(binding, request.title.clone()), body: NativeStringRef::from_value(binding, request.body.clone()) })",
                )),
            ],
        },
    );
    output.push('\n');
    for function in [
        RustBridgeFunctionSpec {
            documentation: "Decode one mobile notification event payload from the host ABI.",
            qualifiers: vec!["pub(crate)"],
            name: "decode_notification_event",
            parameters: vec![RustBridgeParameter {
                name: "event",
                ty: "HostNotificationEvent",
            }],
            return_type: "RuntimeResult<NotificationEventValue>",
            body: vec![
                RustBridgeStatement::Let {
                    name: "request",
                    value: RustBridgeExpr::Path(
                        "decode_notification_request(event.request, event.has_action_identifier, event.action_identifier)?",
                    ),
                },
                RustBridgeStatement::Let {
                    name: "metadata",
                    value: RustBridgeExpr::Path(
                        "NotificationEventMetadataValue { timestamp_ns: event.timestamp_ns, sequence: event.sequence, id: request.tag.clone(), request }",
                    ),
                },
                RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "event.kind",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostNotificationEventKind::Delivered",
                            value: RustBridgeExpr::Path(
                                "Ok(NotificationEventValue::NotificationDeliveredEvent(NotificationDeliveredEventValue { kind: \"delivered\".to_string(), metadata }))",
                            ),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostNotificationEventKind::Activated",
                            value: RustBridgeExpr::Path(
                                "{ let action_id = decode_optional_string(event.has_action_identifier, event.action_identifier, \"HostNotificationEvent.action_identifier\")?; Ok(NotificationEventValue::NotificationInteractedEvent(NotificationInteractedEventValue { kind: \"interacted\".to_string(), metadata, payload: NotificationInteractedPayloadValue { action_id, action_response_text: None } })) }",
                            ),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostNotificationEventKind::Dismissed",
                            value: RustBridgeExpr::Path(
                                "Ok(NotificationEventValue::NotificationDismissedEvent(NotificationDismissedEventValue { kind: \"dismissed\".to_string(), metadata }))",
                            ),
                        },
                    ],
                }),
            ],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one mobile notification request payload from the host ABI.",
            qualifiers: Vec::new(),
            name: "decode_notification_request",
            parameters: vec![
                RustBridgeParameter {
                    name: "request",
                    ty: "HostNotificationRequest",
                },
                RustBridgeParameter {
                    name: "has_action_identifier",
                    ty: "bool",
                },
                RustBridgeParameter {
                    name: "action_identifier",
                    ty: "NativeStringRef",
                },
            ],
            return_type: "RuntimeResult<NotificationRequestValue>",
            body: vec![
                RustBridgeStatement::Let {
                    name: "identifier",
                    value: RustBridgeExpr::Path(
                        "decode_required_string(request.identifier, \"HostNotificationRequest.identifier\")?",
                    ),
                },
                RustBridgeStatement::Let {
                    name: "title",
                    value: RustBridgeExpr::Path(
                        "decode_required_string(request.title, \"HostNotificationRequest.title\")?",
                    ),
                },
                RustBridgeStatement::Let {
                    name: "body",
                    value: RustBridgeExpr::Path(
                        "decode_required_string(request.body, \"HostNotificationRequest.body\")?",
                    ),
                },
                RustBridgeStatement::Let {
                    name: "action_id",
                    value: RustBridgeExpr::Path(
                        "decode_optional_string(has_action_identifier, action_identifier, \"HostNotificationEvent.action_identifier\")?",
                    ),
                },
                RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(NotificationRequestValue { title, subtitle: None, body, tag: identifier, channel_id: None, priority: NotificationPriority::Normal, badge_count: None, sound: None, category_id: None, thread_id: None, trigger: NotificationTriggerValue::NotificationImmediateTrigger(NotificationImmediateTriggerValue { kind: \"immediate\".to_string() }), action_id })",
                )),
            ],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one required notification string from the host ABI.",
            qualifiers: Vec::new(),
            name: "decode_required_string",
            parameters: vec![
                RustBridgeParameter {
                    name: "value",
                    ty: "NativeStringRef",
                },
                RustBridgeParameter {
                    name: "argument",
                    ty: "&'static str",
                },
            ],
            return_type: "RuntimeResult<String>",
            body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                "unsafe { value.as_str() }.map(str::to_string).map_err(|_| invalid_argument_value(argument, format!(\"invalid {argument} string\")))",
            ))],
        },
        RustBridgeFunctionSpec {
            documentation: "Decode one optional notification string from the host ABI.",
            qualifiers: Vec::new(),
            name: "decode_optional_string",
            parameters: vec![
                RustBridgeParameter {
                    name: "is_present",
                    ty: "bool",
                },
                RustBridgeParameter {
                    name: "value",
                    ty: "NativeStringRef",
                },
                RustBridgeParameter {
                    name: "argument",
                    ty: "&'static str",
                },
            ],
            return_type: "RuntimeResult<Option<String>>",
            body: vec![
                RustBridgeStatement::If {
                    condition: RustBridgeExpr::Path("!is_present"),
                    then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Ok(None)",
                    ))],
                },
                RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "decode_required_string(value, argument).map(Some)",
                )),
            ],
        },
    ] {
        render_rust_bridge_function(&mut output, &function);
        output.push('\n');
    }

    output
}

/// Render the generated Rust Android ABI files for one module.
pub(crate) fn render_android_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    render_platform_files(layout, module.abi(), HostPlatform::Android)
}

/// Render the generated Rust Android ingress file for one module.
pub(crate) fn render_android_ingress_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    render_platform_ingress_files(layout, module.abi(), HostPlatform::Android)
}

/// Render the generated Rust iOS ingress file for one module.
pub(crate) fn render_ios_ingress_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    render_platform_ingress_files(layout, module.abi(), HostPlatform::Ios)
}

/// Render the generated Rust ABI files for one platform and module.
fn render_platform_files(
    layout: &WorkspaceLayout,
    module: &HostAbiModule,
    platform: HostPlatform,
) -> Vec<HostArtifact> {
    let platform_segment = platform.segment();
    let mut files = Vec::new();

    if module_uses_generated_types(module, platform) {
        files.push(HostArtifact {
            path: layout.language_root.join(format!(
                "runtime/src/host/{platform_segment}/abi/{}/types.generated.rs",
                module.name
            )),
            contents: render_types(module),
        });
    }

    files.extend([
        HostArtifact {
            path: layout.language_root.join(format!(
                "runtime/src/host/{platform_segment}/abi/{}/callbacks.generated.rs",
                module.name
            )),
            contents: render_callbacks(module, platform),
        },
        HostArtifact {
            path: layout.language_root.join(format!(
                "runtime/src/host/{platform_segment}/abi/{}/ffi.generated.rs",
                module.name
            )),
            contents: render_ffi(module, platform),
        },
    ]);

    files
}

/// One generated Rust bridge file.
#[derive(Clone, Debug)]
struct RustBridgeFileSpec {
    /// The output file name.
    file_name: &'static str,
    /// The module imports.
    imports: BTreeSet<&'static str>,
    /// The nested module declarations.
    nested_modules: Vec<&'static str>,
    /// The nested-item reexports.
    reexports: Vec<&'static str>,
    /// The helper functions.
    helpers: Vec<&'static str>,
    /// The typed helper functions.
    helper_functions: Vec<RustBridgeFunctionSpec>,
    /// The payload structs.
    payloads: Vec<RustBridgePayloadSpec>,
}

/// One generated Rust helper function.
#[derive(Clone, Debug)]
struct RustBridgeFunctionSpec {
    /// The function documentation.
    documentation: &'static str,
    /// The function qualifiers.
    qualifiers: Vec<&'static str>,
    /// The function name.
    name: &'static str,
    /// The function parameters.
    parameters: Vec<RustBridgeParameter>,
    /// The function return type.
    return_type: &'static str,
    /// The function body.
    body: Vec<RustBridgeStatement>,
}

/// One generated Rust bridge payload.
#[derive(Clone, Debug)]
struct RustBridgePayloadSpec {
    /// The payload documentation.
    documentation: &'static str,
    /// The payload type name.
    struct_name: &'static str,
    /// The owned storage fields.
    storage_fields: Vec<RustBridgeStorageField>,
    /// The ABI payload field documentation.
    abi_documentation: &'static str,
    /// The ABI payload type.
    abi_type: &'static str,
    /// The constructor documentation.
    constructor_documentation: &'static str,
    /// The constructor name.
    constructor_name: &'static str,
    /// The constructor parameters.
    constructor_parameters: Vec<RustBridgeParameter>,
    /// The local constructor bindings.
    local_bindings: Vec<RustBridgeLocal>,
    /// The ABI initializer expression.
    abi_initializer: RustBridgeExpr,
    /// The ABI accessor documentation.
    accessor_documentation: &'static str,
}

/// One generated Rust payload storage field.
#[derive(Clone, Debug)]
struct RustBridgeStorageField {
    /// The field documentation.
    documentation: &'static str,
    /// The field name.
    name: &'static str,
    /// The field type.
    ty: &'static str,
}

/// One generated Rust function parameter.
#[derive(Clone, Debug)]
struct RustBridgeParameter {
    /// The parameter name.
    name: &'static str,
    /// The parameter type.
    ty: &'static str,
}

/// One generated Rust local binding.
#[derive(Clone, Debug)]
struct RustBridgeLocal {
    /// The local binding name.
    name: &'static str,
    /// The binding expression.
    value: RustBridgeExpr,
}

/// One generated Rust statement.
#[derive(Clone, Debug)]
enum RustBridgeStatement {
    /// One local binding.
    Let {
        /// The bound name.
        name: &'static str,
        /// The bound value.
        value: RustBridgeExpr,
    },
    /// One return statement.
    Return(RustBridgeExpr),
    /// One if statement.
    If {
        /// The condition expression.
        condition: RustBridgeExpr,
        /// The then branch.
        then_body: Vec<RustBridgeStatement>,
    },
}

/// One generated Rust bridge expression.
#[derive(Clone, Debug)]
enum RustBridgeExpr {
    /// One raw path expression.
    Path(&'static str),
    /// One cloned path expression.
    ClonePath(&'static str),
    /// One borrowed path expression.
    BorrowPath(&'static str),
    /// One function call.
    FunctionCall {
        /// The function path.
        function: &'static str,
        /// The call arguments.
        arguments: Vec<RustBridgeExpr>,
    },
    /// One method call.
    MethodCall {
        /// The receiver expression.
        receiver: Box<RustBridgeExpr>,
        /// The method name.
        method: &'static str,
        /// The call arguments.
        arguments: Vec<RustBridgeExpr>,
    },
    /// One struct initializer.
    StructInit {
        /// The struct type.
        ty: &'static str,
        /// The field initializers.
        fields: Vec<RustBridgeFieldInitializer>,
    },
    /// One iterator collect over mapped refs.
    IterMapCollectVec {
        /// The iterated source binding.
        source: &'static str,
        /// The mapping function.
        mapper: &'static str,
    },
    /// One `is_some` expression.
    IsSome(&'static str),
    /// One `map(...).unwrap_or_default()` expression.
    OptionMapOrDefault {
        /// The option source binding.
        source: &'static str,
        /// The mapper function.
        mapper: &'static str,
    },
    /// One match expression.
    Match {
        /// The matched expression.
        scrutinee: &'static str,
        /// The match arms.
        arms: Vec<RustBridgeMatchArm>,
    },
}

/// One generated Rust match arm.
#[derive(Clone, Debug)]
struct RustBridgeMatchArm {
    /// The arm pattern.
    pattern: &'static str,
    /// The arm value.
    value: RustBridgeExpr,
}

/// One generated Rust struct field initializer.
#[derive(Clone, Debug)]
struct RustBridgeFieldInitializer {
    /// The field name.
    name: &'static str,
    /// The field value.
    value: RustBridgeExpr,
}

/// Build the generated Rust bridge files for one module.
fn build_rust_bridge_file_specs(module: &HostModule) -> Vec<RustBridgeFileSpec> {
    match module.name() {
        "calendar" => vec![build_calendar_bridge_file_spec()],
        "contact" => vec![build_contact_bridge_file_spec()],
        "document" => vec![build_document_bridge_file_spec()],
        "media" => vec![build_media_bridge_file_spec()],
        "permission" => vec![build_permission_bridge_file_spec()],
        "text" => vec![build_text_bridge_file_spec()],
        _ => Vec::new(),
    }
}

/// Build the generated Rust bridge file for calendar helpers.
fn build_calendar_bridge_file_spec() -> RustBridgeFileSpec {
    let mut imports = BTreeSet::new();
    imports.insert("crate::diagnostic::{RuntimeError, RuntimeResult};");
    imports.insert("crate::host::abi::calendar::{HostCalendarAccess, HostCalendarAttendee, HostCalendarAvailability, HostCalendarDescriptor, HostCalendarEvent, HostCalendarEventCreateResponse, HostCalendarEventDraft, HostCalendarEventListResponse, HostCalendarEventQuery, HostCalendarEventReadResponse, HostCalendarListResponse, HostCalendarParticipantStatus, HostCalendarRecurrenceFrequency, HostCalendarRecurrenceRule, HostCalendarRecurrenceWeekday, HostCalendarReminder, HostCalendarReminderKind};");
    imports.insert("crate::host::core::callback::decode_callback_host_required_string_ref;");
    imports.insert("crate::platform::PlatformError;");
    imports.insert("crate::platform::abi::NativeStringRef;");
    imports.insert("crate::platform::os::abi_generated::{CalendarAbsoluteReminderValue, CalendarAccess, CalendarAttendeeValue, CalendarAvailability, CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue, CalendarParticipantStatus, CalendarRecurrenceFrequency, CalendarRecurrenceRuleValue, CalendarRecurrenceWeekdayValue, CalendarRelativeReminderValue, CalendarReminderValue};");
    imports.insert("crate::runtime::BindingCallContext;");

    RustBridgeFileSpec {
        file_name: "runtime.rs",
        imports,
        nested_modules: Vec::new(),
        reexports: Vec::new(),
        helpers: Vec::new(),
        helper_functions: vec![
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar event-query payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_calendar_event_query",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "query",
                        ty: "&CalendarEventQueryValue",
                    },
                ],
                return_type: "HostCalendarEventQuery",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostCalendarEventQuery { calendar_ids: binding.store_string_slice(query.calendar_ids.iter().map(NativeStringRef::from).collect()), start_unix_ns: query.start_unix_ns, end_unix_ns: query.end_unix_ns, has_limit: query.limit.is_some(), limit: query.limit.unwrap_or_default(), include_canceled: query.include_canceled, include_declined: query.include_declined, include_recurrence_instances: query.include_recurrence_instances }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar event-draft payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_calendar_event_draft",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "draft",
                        ty: "&CalendarEventDraftValue",
                    },
                ],
                return_type: "HostCalendarEventDraft",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostCalendarEventDraft { calendar_id: binding.store_string(&draft.calendar_id), title: binding.store_string(&draft.title), has_notes: draft.notes.is_some(), notes: binding.store_string(draft.notes.as_deref().unwrap_or(\"\")), has_location: draft.location.is_some(), location: binding.store_string(draft.location.as_deref().unwrap_or(\"\")), start_unix_ns: draft.start_unix_ns, end_unix_ns: draft.end_unix_ns, all_day: draft.all_day, has_time_zone: draft.time_zone.is_some(), time_zone: binding.store_string(draft.time_zone.as_deref().unwrap_or(\"\")), availability: encode_calendar_availability(draft.availability), has_url: draft.url.is_some(), url: binding.store_string(draft.url.as_deref().unwrap_or(\"\")), has_recurrence_rule: draft.recurrence_rule.is_some(), recurrence_rule: draft.recurrence_rule.as_ref().map(|rule| encode_calendar_recurrence_rule(binding, rule)).unwrap_or_else(|| empty_calendar_recurrence_rule(binding)), has_attendees: draft.attendees.is_some(), attendees: binding.store_slice(draft.attendees.as_ref().map(|attendees| attendees.iter().map(|attendee| encode_calendar_attendee(binding, attendee)).collect()).unwrap_or_default()), has_reminders: draft.reminders.is_some(), reminders: binding.store_slice(draft.reminders.as_ref().map(|reminders| reminders.iter().map(encode_calendar_reminder).collect()).unwrap_or_default()) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar-list response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_calendar_list_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostCalendarListResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<Vec<CalendarDescriptorValue>>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "calendars",
                        value: RustBridgeExpr::Path("unsafe { response.calendars.as_slice()? }"),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "calendars.iter().copied().map(|calendar| decode_calendar_descriptor(calendar, operation)).collect()",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar event-list response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_calendar_event_list_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostCalendarEventListResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<Vec<CalendarEventValue>>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "events",
                        value: RustBridgeExpr::Path("unsafe { response.events.as_slice()? }"),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "events.iter().copied().map(|event| decode_calendar_event(event, operation)).collect()",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar event-read response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_calendar_event_read_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostCalendarEventReadResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<CalendarEventValue>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_event"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.event\", format!(\"{operation} returned one missing response.event\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "decode_calendar_event(response.event, operation)",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar event-create response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_calendar_event_create_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostCalendarEventCreateResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<String>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_id"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.id\", format!(\"{operation} returned one missing response.id\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "decode_callback_host_required_string_ref(operation, \"HostCalendarEventCreateResponse.id\", response.id)",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar access payload for the host ABI.",
                qualifiers: Vec::new(),
                name: "encode_calendar_access",
                parameters: vec![RustBridgeParameter {
                    name: "access",
                    ty: "CalendarAccess",
                }],
                return_type: "HostCalendarAccess",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "access",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "CalendarAccess::Read",
                            value: RustBridgeExpr::Path("HostCalendarAccess::Read"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarAccess::Write",
                            value: RustBridgeExpr::Path("HostCalendarAccess::Write"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar access payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_access",
                parameters: vec![RustBridgeParameter {
                    name: "access",
                    ty: "HostCalendarAccess",
                }],
                return_type: "CalendarAccess",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "access",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAccess::Read",
                            value: RustBridgeExpr::Path("CalendarAccess::Read"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAccess::Write",
                            value: RustBridgeExpr::Path("CalendarAccess::Write"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar availability payload for the host ABI.",
                qualifiers: Vec::new(),
                name: "encode_calendar_availability",
                parameters: vec![RustBridgeParameter {
                    name: "availability",
                    ty: "CalendarAvailability",
                }],
                return_type: "HostCalendarAvailability",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "availability",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "CalendarAvailability::Busy",
                            value: RustBridgeExpr::Path("HostCalendarAvailability::Busy"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarAvailability::Free",
                            value: RustBridgeExpr::Path("HostCalendarAvailability::Free"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarAvailability::Tentative",
                            value: RustBridgeExpr::Path("HostCalendarAvailability::Tentative"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarAvailability::OutOfOffice",
                            value: RustBridgeExpr::Path("HostCalendarAvailability::OutOfOffice"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarAvailability::Unavailable",
                            value: RustBridgeExpr::Path("HostCalendarAvailability::Unavailable"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarAvailability::Unknown",
                            value: RustBridgeExpr::Path("HostCalendarAvailability::Unknown"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar availability payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_availability",
                parameters: vec![RustBridgeParameter {
                    name: "availability",
                    ty: "HostCalendarAvailability",
                }],
                return_type: "CalendarAvailability",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "availability",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAvailability::Busy",
                            value: RustBridgeExpr::Path("CalendarAvailability::Busy"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAvailability::Free",
                            value: RustBridgeExpr::Path("CalendarAvailability::Free"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAvailability::Tentative",
                            value: RustBridgeExpr::Path("CalendarAvailability::Tentative"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAvailability::OutOfOffice",
                            value: RustBridgeExpr::Path("CalendarAvailability::OutOfOffice"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAvailability::Unavailable",
                            value: RustBridgeExpr::Path("CalendarAvailability::Unavailable"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarAvailability::Unknown",
                            value: RustBridgeExpr::Path("CalendarAvailability::Unknown"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one participant-status payload for the host ABI.",
                qualifiers: Vec::new(),
                name: "encode_calendar_participant_status",
                parameters: vec![RustBridgeParameter {
                    name: "status",
                    ty: "CalendarParticipantStatus",
                }],
                return_type: "HostCalendarParticipantStatus",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "status",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Unknown",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Unknown"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Pending",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Pending"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Accepted",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Accepted"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Tentative",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Tentative"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Declined",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Declined"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Delegated",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Delegated"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::Completed",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::Completed"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarParticipantStatus::InProcess",
                            value: RustBridgeExpr::Path("HostCalendarParticipantStatus::InProcess"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host participant-status payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_participant_status",
                parameters: vec![RustBridgeParameter {
                    name: "status",
                    ty: "HostCalendarParticipantStatus",
                }],
                return_type: "CalendarParticipantStatus",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "status",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Unknown",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Unknown"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Pending",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Pending"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Accepted",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Accepted"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Tentative",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Tentative"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Declined",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Declined"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Delegated",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Delegated"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::Completed",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::Completed"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarParticipantStatus::InProcess",
                            value: RustBridgeExpr::Path("CalendarParticipantStatus::InProcess"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one recurrence-frequency payload for the host ABI.",
                qualifiers: Vec::new(),
                name: "encode_calendar_recurrence_frequency",
                parameters: vec![RustBridgeParameter {
                    name: "frequency",
                    ty: "CalendarRecurrenceFrequency",
                }],
                return_type: "HostCalendarRecurrenceFrequency",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "frequency",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "CalendarRecurrenceFrequency::Daily",
                            value: RustBridgeExpr::Path("HostCalendarRecurrenceFrequency::Daily"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarRecurrenceFrequency::Weekly",
                            value: RustBridgeExpr::Path("HostCalendarRecurrenceFrequency::Weekly"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarRecurrenceFrequency::Monthly",
                            value: RustBridgeExpr::Path("HostCalendarRecurrenceFrequency::Monthly"),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarRecurrenceFrequency::Yearly",
                            value: RustBridgeExpr::Path("HostCalendarRecurrenceFrequency::Yearly"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host recurrence-frequency payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_recurrence_frequency",
                parameters: vec![RustBridgeParameter {
                    name: "frequency",
                    ty: "HostCalendarRecurrenceFrequency",
                }],
                return_type: "CalendarRecurrenceFrequency",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "frequency",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostCalendarRecurrenceFrequency::Daily",
                            value: RustBridgeExpr::Path("CalendarRecurrenceFrequency::Daily"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarRecurrenceFrequency::Weekly",
                            value: RustBridgeExpr::Path("CalendarRecurrenceFrequency::Weekly"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarRecurrenceFrequency::Monthly",
                            value: RustBridgeExpr::Path("CalendarRecurrenceFrequency::Monthly"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarRecurrenceFrequency::Yearly",
                            value: RustBridgeExpr::Path("CalendarRecurrenceFrequency::Yearly"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar recurrence-weekday payload.",
                qualifiers: Vec::new(),
                name: "encode_calendar_recurrence_weekday",
                parameters: vec![RustBridgeParameter {
                    name: "weekday",
                    ty: "CalendarRecurrenceWeekdayValue",
                }],
                return_type: "HostCalendarRecurrenceWeekday",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostCalendarRecurrenceWeekday { day: weekday.day, has_week_number: weekday.week_number.is_some(), week_number: weekday.week_number.unwrap_or_default() }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar recurrence-weekday payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_recurrence_weekday",
                parameters: vec![RustBridgeParameter {
                    name: "weekday",
                    ty: "HostCalendarRecurrenceWeekday",
                }],
                return_type: "CalendarRecurrenceWeekdayValue",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "CalendarRecurrenceWeekdayValue { day: weekday.day, week_number: weekday.has_week_number.then_some(weekday.week_number) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar recurrence-rule payload.",
                qualifiers: Vec::new(),
                name: "encode_calendar_recurrence_rule",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "rule",
                        ty: "&CalendarRecurrenceRuleValue",
                    },
                ],
                return_type: "HostCalendarRecurrenceRule",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostCalendarRecurrenceRule { frequency: encode_calendar_recurrence_frequency(rule.frequency), interval: rule.interval, has_count: rule.count.is_some(), count: rule.count.unwrap_or_default(), has_until_unix_ns: rule.until_unix_ns.is_some(), until_unix_ns: rule.until_unix_ns.unwrap_or_default(), by_week_days: binding.store_slice(rule.by_week_days.clone()), by_weekday_ordinals: binding.store_slice(rule.by_weekday_ordinals.iter().copied().map(encode_calendar_recurrence_weekday).collect()), by_month_days: binding.store_slice(rule.by_month_days.clone()), by_months: binding.store_slice(rule.by_months.clone()), by_year_days: binding.store_slice(rule.by_year_days.clone()), by_week_numbers: binding.store_slice(rule.by_week_numbers.clone()), by_set_positions: binding.store_slice(rule.by_set_positions.clone()) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar recurrence-rule payload.",
                qualifiers: vec!["unsafe"],
                name: "decode_calendar_recurrence_rule",
                parameters: vec![RustBridgeParameter {
                    name: "rule",
                    ty: "HostCalendarRecurrenceRule",
                }],
                return_type: "RuntimeResult<CalendarRecurrenceRuleValue>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "by_week_days",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_week_days.as_slice()? }.to_vec()",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "by_weekday_ordinals",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_weekday_ordinals.as_slice()? }.iter().copied().map(decode_calendar_recurrence_weekday).collect()",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "by_month_days",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_month_days.as_slice()? }.to_vec()",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "by_months",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_months.as_slice()? }.to_vec()",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "by_year_days",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_year_days.as_slice()? }.to_vec()",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "by_week_numbers",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_week_numbers.as_slice()? }.to_vec()",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "by_set_positions",
                        value: RustBridgeExpr::Path(
                            "unsafe { rule.by_set_positions.as_slice()? }.to_vec()",
                        ),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Ok(CalendarRecurrenceRuleValue { frequency: decode_calendar_recurrence_frequency(rule.frequency), interval: rule.interval, count: rule.has_count.then_some(rule.count), until_unix_ns: rule.has_until_unix_ns.then_some(rule.until_unix_ns), by_week_days, by_weekday_ordinals, by_month_days, by_months, by_year_days, by_week_numbers, by_set_positions })",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Return one empty calendar recurrence-rule payload.",
                qualifiers: Vec::new(),
                name: "empty_calendar_recurrence_rule",
                parameters: vec![RustBridgeParameter {
                    name: "binding",
                    ty: "&BindingCallContext",
                }],
                return_type: "HostCalendarRecurrenceRule",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostCalendarRecurrenceRule { frequency: HostCalendarRecurrenceFrequency::Daily, interval: 0, has_count: false, count: 0, has_until_unix_ns: false, until_unix_ns: 0, by_week_days: binding.store_slice(Vec::new()), by_weekday_ordinals: binding.store_slice(Vec::new()), by_month_days: binding.store_slice(Vec::new()), by_months: binding.store_slice(Vec::new()), by_year_days: binding.store_slice(Vec::new()), by_week_numbers: binding.store_slice(Vec::new()), by_set_positions: binding.store_slice(Vec::new()) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar attendee payload.",
                qualifiers: Vec::new(),
                name: "encode_calendar_attendee",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "attendee",
                        ty: "&CalendarAttendeeValue",
                    },
                ],
                return_type: "HostCalendarAttendee",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostCalendarAttendee { has_id: attendee.id.is_some(), id: binding.store_string(attendee.id.as_deref().unwrap_or(\"\")), has_name: attendee.name.is_some(), name: binding.store_string(attendee.name.as_deref().unwrap_or(\"\")), has_email: attendee.email.is_some(), email: binding.store_string(attendee.email.as_deref().unwrap_or(\"\")), optional: attendee.optional, organizer: attendee.organizer, response_status: encode_calendar_participant_status(attendee.response_status) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar attendee payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_attendee",
                parameters: vec![
                    RustBridgeParameter {
                        name: "attendee",
                        ty: "HostCalendarAttendee",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<CalendarAttendeeValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(CalendarAttendeeValue { id: decode_optional_string_ref(operation, \"HostCalendarAttendee.id\", attendee.has_id, attendee.id)?, name: decode_optional_string_ref(operation, \"HostCalendarAttendee.name\", attendee.has_name, attendee.name)?, email: decode_optional_string_ref(operation, \"HostCalendarAttendee.email\", attendee.has_email, attendee.email)?, optional: attendee.optional, organizer: attendee.organizer, response_status: decode_calendar_participant_status(attendee.response_status) })",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one calendar reminder payload.",
                qualifiers: Vec::new(),
                name: "encode_calendar_reminder",
                parameters: vec![RustBridgeParameter {
                    name: "reminder",
                    ty: "&CalendarReminderValue",
                }],
                return_type: "HostCalendarReminder",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "reminder",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "CalendarReminderValue::CalendarAbsoluteReminder(value)",
                            value: RustBridgeExpr::Path(
                                "HostCalendarReminder { kind: HostCalendarReminderKind::Absolute, absolute_unix_ns: value.absolute_unix_ns, minutes_before_start: 0 }",
                            ),
                        },
                        RustBridgeMatchArm {
                            pattern: "CalendarReminderValue::CalendarRelativeReminder(value)",
                            value: RustBridgeExpr::Path(
                                "HostCalendarReminder { kind: HostCalendarReminderKind::Relative, absolute_unix_ns: 0, minutes_before_start: value.minutes_before_start }",
                            ),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar reminder payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_reminder",
                parameters: vec![RustBridgeParameter {
                    name: "reminder",
                    ty: "HostCalendarReminder",
                }],
                return_type: "CalendarReminderValue",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "reminder.kind",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostCalendarReminderKind::Absolute",
                            value: RustBridgeExpr::Path(
                                "CalendarReminderValue::CalendarAbsoluteReminder(CalendarAbsoluteReminderValue { kind: \"absolute\".to_string(), absolute_unix_ns: reminder.absolute_unix_ns })",
                            ),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostCalendarReminderKind::Relative",
                            value: RustBridgeExpr::Path(
                                "CalendarReminderValue::CalendarRelativeReminder(CalendarRelativeReminderValue { kind: \"relative\".to_string(), minutes_before_start: reminder.minutes_before_start })",
                            ),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar descriptor payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_descriptor",
                parameters: vec![
                    RustBridgeParameter {
                        name: "descriptor",
                        ty: "HostCalendarDescriptor",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<CalendarDescriptorValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(CalendarDescriptorValue { id: decode_callback_host_required_string_ref(operation, \"HostCalendarDescriptor.id\", descriptor.id)?, title: decode_callback_host_required_string_ref(operation, \"HostCalendarDescriptor.title\", descriptor.title)?, source: decode_callback_host_required_string_ref(operation, \"HostCalendarDescriptor.source\", descriptor.source)?, owner: decode_optional_string_ref(operation, \"HostCalendarDescriptor.owner\", descriptor.has_owner, descriptor.owner)?, color_argb: descriptor.color_argb, primary: descriptor.primary, access: decode_calendar_access(descriptor.access) })",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host calendar event payload.",
                qualifiers: Vec::new(),
                name: "decode_calendar_event",
                parameters: vec![
                    RustBridgeParameter {
                        name: "event",
                        ty: "HostCalendarEvent",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<CalendarEventValue>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "recurrence_rule",
                        value: RustBridgeExpr::Path(
                            "if event.has_recurrence_rule { Some(unsafe { decode_calendar_recurrence_rule(event.recurrence_rule)? }) } else { None }",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "attendees",
                        value: RustBridgeExpr::Path(
                            "if event.has_attendees { Some(unsafe { event.attendees.as_slice()? }.iter().copied().map(|attendee| decode_calendar_attendee(attendee, operation)).collect::<RuntimeResult<Vec<_>>>()?) } else { None }",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "reminders",
                        value: RustBridgeExpr::Path(
                            "if event.has_reminders { Some(unsafe { event.reminders.as_slice()? }.iter().copied().map(decode_calendar_reminder).collect()) } else { None }",
                        ),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Ok(CalendarEventValue { id: decode_callback_host_required_string_ref(operation, \"HostCalendarEvent.id\", event.id)?, calendar_id: decode_callback_host_required_string_ref(operation, \"HostCalendarEvent.calendar_id\", event.calendar_id)?, title: decode_callback_host_required_string_ref(operation, \"HostCalendarEvent.title\", event.title)?, notes: decode_optional_string_ref(operation, \"HostCalendarEvent.notes\", event.has_notes, event.notes)?, location: decode_optional_string_ref(operation, \"HostCalendarEvent.location\", event.has_location, event.location)?, start_unix_ns: event.start_unix_ns, end_unix_ns: event.end_unix_ns, all_day: event.all_day, canceled: event.canceled, time_zone: decode_optional_string_ref(operation, \"HostCalendarEvent.time_zone\", event.has_time_zone, event.time_zone)?, availability: decode_calendar_availability(event.availability), url: decode_optional_string_ref(operation, \"HostCalendarEvent.url\", event.has_url, event.url)?, organizer_name: decode_optional_string_ref(operation, \"HostCalendarEvent.organizer_name\", event.has_organizer_name, event.organizer_name)?, organizer_email: decode_optional_string_ref(operation, \"HostCalendarEvent.organizer_email\", event.has_organizer_email, event.organizer_email)?, recurring: event.recurring, recurrence_master_id: decode_optional_string_ref(operation, \"HostCalendarEvent.recurrence_master_id\", event.has_recurrence_master_id, event.recurrence_master_id)?, recurrence_id_unix_ns: event.has_recurrence_id_unix_ns.then_some(event.recurrence_id_unix_ns), recurrence_rule, attendees, reminders })",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one optional host string reference.",
                qualifiers: Vec::new(),
                name: "decode_optional_string_ref",
                parameters: vec![
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                    RustBridgeParameter {
                        name: "field_name",
                        ty: "&'static str",
                    },
                    RustBridgeParameter {
                        name: "is_present",
                        ty: "bool",
                    },
                    RustBridgeParameter {
                        name: "value",
                        ty: "crate::platform::abi::NativeStringRef",
                    },
                ],
                return_type: "RuntimeResult<Option<String>>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!is_present"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Ok(None)",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "decode_callback_host_required_string_ref(operation, field_name, value).map(Some)",
                    )),
                ],
            },
        ],
        payloads: Vec::new(),
    }
}

/// Build the generated Rust bridge file for contact helpers.
fn build_contact_bridge_file_spec() -> RustBridgeFileSpec {
    let mut imports = BTreeSet::new();
    imports.insert("crate::diagnostic::{RuntimeError, RuntimeResult};");
    imports.insert("crate::host::abi::contact::{HostContact, HostContactAddress, HostContactCreateResponse, HostContactDraft, HostContactEmail, HostContactName, HostContactOrganization, HostContactPage, HostContactPageResponse, HostContactPhone, HostContactQuery, HostContactResponse};");
    imports.insert("crate::host::core::callback::decode_callback_host_required_string_ref;");
    imports.insert("crate::platform::PlatformError;");
    imports.insert("crate::platform::os::abi_generated::{ContactAddressValue, ContactDraftValue, ContactEmailValue, ContactNameValue, ContactOrganizationValue, ContactPageValue, ContactPhoneValue, ContactQueryValue, ContactValue};");
    imports.insert("crate::runtime::BindingCallContext;");

    RustBridgeFileSpec {
        file_name: "runtime.rs",
        imports,
        nested_modules: Vec::new(),
        reexports: Vec::new(),
        helpers: Vec::new(),
        helper_functions: vec![
            RustBridgeFunctionSpec {
                documentation: "Encode one contact query payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_contact_query",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "query",
                        ty: "&ContactQueryValue",
                    },
                ],
                return_type: "HostContactQuery",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactQuery { has_cursor: query.cursor.is_some(), cursor: binding.store_string(query.cursor.as_deref().unwrap_or(\"\")), has_limit: query.limit.is_some(), limit: query.limit.unwrap_or_default(), include_phones: query.include_phones, include_emails: query.include_emails, include_addresses: query.include_addresses, include_organization: query.include_organization, include_notes: query.include_notes }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one contact draft payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_contact_draft",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "draft",
                        ty: "&ContactDraftValue",
                    },
                ],
                return_type: "HostContactDraft",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactDraft { name: encode_contact_name(binding, &draft.name), phones: binding.store_slice(draft.phones.iter().map(|phone| encode_contact_phone(binding, phone)).collect()), emails: binding.store_slice(draft.emails.iter().map(|email| encode_contact_email(binding, email)).collect()), addresses: binding.store_slice(draft.addresses.iter().map(|address| encode_contact_address(binding, address)).collect()), organization: encode_contact_organization(binding, &draft.organization), note: binding.store_string(&draft.note) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-page response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_contact_page_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostContactPageResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactPageValue>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_page"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.page\", format!(\"{operation} returned one missing response.page\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "unsafe { decode_contact_page(response.page, operation) }",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-read response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_contact_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostContactResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactValue>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_contact"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.contact\", format!(\"{operation} returned one missing response.contact\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "unsafe { decode_contact(response.contact, operation) }",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-create response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_contact_create_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostContactCreateResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<String>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_id"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.id\", format!(\"{operation} returned one missing response.id\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "decode_callback_host_required_string_ref(operation, \"HostContactCreateResponse.id\", response.id)",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one contact-name payload.",
                qualifiers: Vec::new(),
                name: "encode_contact_name",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "name",
                        ty: "&ContactNameValue",
                    },
                ],
                return_type: "HostContactName",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactName { given_name: binding.store_string(&name.given_name), middle_name: binding.store_string(&name.middle_name), family_name: binding.store_string(&name.family_name), prefix: binding.store_string(&name.prefix), suffix: binding.store_string(&name.suffix), nickname: binding.store_string(&name.nickname), phonetic_given_name: binding.store_string(&name.phonetic_given_name), phonetic_family_name: binding.store_string(&name.phonetic_family_name) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one contact-phone payload.",
                qualifiers: Vec::new(),
                name: "encode_contact_phone",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "phone",
                        ty: "&ContactPhoneValue",
                    },
                ],
                return_type: "HostContactPhone",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactPhone { label: binding.store_string(&phone.label), number: binding.store_string(&phone.number), normalized_number: binding.store_string(&phone.normalized_number), primary: phone.primary }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one contact-email payload.",
                qualifiers: Vec::new(),
                name: "encode_contact_email",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "email",
                        ty: "&ContactEmailValue",
                    },
                ],
                return_type: "HostContactEmail",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactEmail { label: binding.store_string(&email.label), address: binding.store_string(&email.address), primary: email.primary }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one contact-address payload.",
                qualifiers: Vec::new(),
                name: "encode_contact_address",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "address",
                        ty: "&ContactAddressValue",
                    },
                ],
                return_type: "HostContactAddress",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactAddress { label: binding.store_string(&address.label), street: binding.store_string(&address.street), city: binding.store_string(&address.city), region: binding.store_string(&address.region), postal_code: binding.store_string(&address.postal_code), country: binding.store_string(&address.country), country_code: binding.store_string(&address.country_code) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one contact-organization payload.",
                qualifiers: Vec::new(),
                name: "encode_contact_organization",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "organization",
                        ty: "&ContactOrganizationValue",
                    },
                ],
                return_type: "HostContactOrganization",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostContactOrganization { company: binding.store_string(&organization.company), department: binding.store_string(&organization.department), title: binding.store_string(&organization.title) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact page.",
                qualifiers: vec!["unsafe"],
                name: "decode_contact_page",
                parameters: vec![
                    RustBridgeParameter {
                        name: "page",
                        ty: "HostContactPage",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactPageValue>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "contacts",
                        value: RustBridgeExpr::Path("unsafe { page.contacts.as_slice()? }"),
                    },
                    RustBridgeStatement::Let {
                        name: "contacts",
                        value: RustBridgeExpr::Path(
                            "contacts.iter().copied().map(|contact| unsafe { decode_contact(contact, operation) }).collect::<RuntimeResult<Vec<_>>>()?",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "next_cursor",
                        value: RustBridgeExpr::Path(
                            "decode_callback_host_required_string_ref(operation, \"HostContactPage.next_cursor\", page.next_cursor)?",
                        ),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Ok(ContactPageValue { contacts, next_cursor, has_more: page.has_more })",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact payload.",
                qualifiers: vec!["unsafe"],
                name: "decode_contact",
                parameters: vec![
                    RustBridgeParameter {
                        name: "contact",
                        ty: "HostContact",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactValue>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "phones",
                        value: RustBridgeExpr::Path("unsafe { contact.phones.as_slice()? }"),
                    },
                    RustBridgeStatement::Let {
                        name: "phones",
                        value: RustBridgeExpr::Path(
                            "phones.iter().copied().map(|phone| decode_contact_phone(phone, operation)).collect::<RuntimeResult<Vec<_>>>()?",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "emails",
                        value: RustBridgeExpr::Path("unsafe { contact.emails.as_slice()? }"),
                    },
                    RustBridgeStatement::Let {
                        name: "emails",
                        value: RustBridgeExpr::Path(
                            "emails.iter().copied().map(|email| decode_contact_email(email, operation)).collect::<RuntimeResult<Vec<_>>>()?",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "addresses",
                        value: RustBridgeExpr::Path("unsafe { contact.addresses.as_slice()? }"),
                    },
                    RustBridgeStatement::Let {
                        name: "addresses",
                        value: RustBridgeExpr::Path(
                            "addresses.iter().copied().map(|address| decode_contact_address(address, operation)).collect::<RuntimeResult<Vec<_>>>()?",
                        ),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Ok(ContactValue { id: decode_callback_host_required_string_ref(operation, \"HostContact.id\", contact.id)?, name: decode_contact_name(contact.name, operation)?, phones, emails, addresses, organization: decode_contact_organization(contact.organization, operation)?, note: decode_callback_host_required_string_ref(operation, \"HostContact.note\", contact.note)? })",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-name payload.",
                qualifiers: Vec::new(),
                name: "decode_contact_name",
                parameters: vec![
                    RustBridgeParameter {
                        name: "name",
                        ty: "HostContactName",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactNameValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(ContactNameValue { given_name: decode_callback_host_required_string_ref(operation, \"HostContactName.given_name\", name.given_name)?, middle_name: decode_callback_host_required_string_ref(operation, \"HostContactName.middle_name\", name.middle_name)?, family_name: decode_callback_host_required_string_ref(operation, \"HostContactName.family_name\", name.family_name)?, prefix: decode_callback_host_required_string_ref(operation, \"HostContactName.prefix\", name.prefix)?, suffix: decode_callback_host_required_string_ref(operation, \"HostContactName.suffix\", name.suffix)?, nickname: decode_callback_host_required_string_ref(operation, \"HostContactName.nickname\", name.nickname)?, phonetic_given_name: decode_callback_host_required_string_ref(operation, \"HostContactName.phonetic_given_name\", name.phonetic_given_name)?, phonetic_family_name: decode_callback_host_required_string_ref(operation, \"HostContactName.phonetic_family_name\", name.phonetic_family_name)? })",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-phone payload.",
                qualifiers: Vec::new(),
                name: "decode_contact_phone",
                parameters: vec![
                    RustBridgeParameter {
                        name: "phone",
                        ty: "HostContactPhone",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactPhoneValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(ContactPhoneValue { label: decode_callback_host_required_string_ref(operation, \"HostContactPhone.label\", phone.label)?, number: decode_callback_host_required_string_ref(operation, \"HostContactPhone.number\", phone.number)?, normalized_number: decode_callback_host_required_string_ref(operation, \"HostContactPhone.normalized_number\", phone.normalized_number)?, primary: phone.primary })",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-email payload.",
                qualifiers: Vec::new(),
                name: "decode_contact_email",
                parameters: vec![
                    RustBridgeParameter {
                        name: "email",
                        ty: "HostContactEmail",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactEmailValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(ContactEmailValue { label: decode_callback_host_required_string_ref(operation, \"HostContactEmail.label\", email.label)?, address: decode_callback_host_required_string_ref(operation, \"HostContactEmail.address\", email.address)?, primary: email.primary })",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-address payload.",
                qualifiers: Vec::new(),
                name: "decode_contact_address",
                parameters: vec![
                    RustBridgeParameter {
                        name: "address",
                        ty: "HostContactAddress",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactAddressValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(ContactAddressValue { label: decode_callback_host_required_string_ref(operation, \"HostContactAddress.label\", address.label)?, street: decode_callback_host_required_string_ref(operation, \"HostContactAddress.street\", address.street)?, city: decode_callback_host_required_string_ref(operation, \"HostContactAddress.city\", address.city)?, region: decode_callback_host_required_string_ref(operation, \"HostContactAddress.region\", address.region)?, postal_code: decode_callback_host_required_string_ref(operation, \"HostContactAddress.postal_code\", address.postal_code)?, country: decode_callback_host_required_string_ref(operation, \"HostContactAddress.country\", address.country)?, country_code: decode_callback_host_required_string_ref(operation, \"HostContactAddress.country_code\", address.country_code)? })",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host contact-organization payload.",
                qualifiers: Vec::new(),
                name: "decode_contact_organization",
                parameters: vec![
                    RustBridgeParameter {
                        name: "organization",
                        ty: "HostContactOrganization",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<ContactOrganizationValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(ContactOrganizationValue { company: decode_callback_host_required_string_ref(operation, \"HostContactOrganization.company\", organization.company)?, department: decode_callback_host_required_string_ref(operation, \"HostContactOrganization.department\", organization.department)?, title: decode_callback_host_required_string_ref(operation, \"HostContactOrganization.title\", organization.title)? })",
                ))],
            },
        ],
        payloads: Vec::new(),
    }
}

/// Build the generated Rust bridge file for document payloads.
fn build_document_bridge_file_spec() -> RustBridgeFileSpec {
    let mut imports = BTreeSet::new();
    imports.insert("crate::platform::abi::{NativeStringRef, NativeStringSlice};");
    imports.insert("crate::platform::os::abi_generated::DocumentPickOptionsValue;");
    imports.insert("crate::host::abi::document::HostDocumentRequest;");

    RustBridgeFileSpec {
        file_name: "runtime.rs",
        imports,
        nested_modules: Vec::new(),
        reexports: Vec::new(),
        helpers: Vec::new(),
        helper_functions: Vec::new(),
        payloads: vec![RustBridgePayloadSpec {
            documentation: "One owned host document request payload.",
            struct_name: "HostDocumentRequestPayload",
            storage_fields: vec![
                RustBridgeStorageField {
                    documentation: "The owned MIME-type backing storage.",
                    name: "_mime_type_storage",
                    ty: "Vec<String>",
                },
                RustBridgeStorageField {
                    documentation: "The borrowed MIME-type refs.",
                    name: "_mime_type_refs",
                    ty: "Vec<NativeStringRef>",
                },
                RustBridgeStorageField {
                    documentation: "The owned extension backing storage.",
                    name: "_extension_storage",
                    ty: "Vec<String>",
                },
                RustBridgeStorageField {
                    documentation: "The borrowed extension refs.",
                    name: "_extension_refs",
                    ty: "Vec<NativeStringRef>",
                },
            ],
            abi_documentation: "The borrowed ABI request view.",
            abi_type: "HostDocumentRequest",
            constructor_documentation: "Build one owned host document request payload.",
            constructor_name: "new",
            constructor_parameters: vec![
                RustBridgeParameter {
                    name: "request_id",
                    ty: "u64",
                },
                RustBridgeParameter {
                    name: "options",
                    ty: "&DocumentPickOptionsValue",
                },
            ],
            local_bindings: vec![
                RustBridgeLocal {
                    name: "mime_type_storage",
                    value: RustBridgeExpr::ClonePath("options.mime_types"),
                },
                RustBridgeLocal {
                    name: "mime_type_refs",
                    value: RustBridgeExpr::IterMapCollectVec {
                        source: "mime_type_storage",
                        mapper: "NativeStringRef::from",
                    },
                },
                RustBridgeLocal {
                    name: "extension_storage",
                    value: RustBridgeExpr::ClonePath("options.extensions"),
                },
                RustBridgeLocal {
                    name: "extension_refs",
                    value: RustBridgeExpr::IterMapCollectVec {
                        source: "extension_storage",
                        mapper: "NativeStringRef::from",
                    },
                },
            ],
            abi_initializer: RustBridgeExpr::StructInit {
                ty: "HostDocumentRequest",
                fields: vec![
                    RustBridgeFieldInitializer {
                        name: "request_id",
                        value: RustBridgeExpr::Path("request_id"),
                    },
                    RustBridgeFieldInitializer {
                        name: "mime_types",
                        value: RustBridgeExpr::FunctionCall {
                            function: "NativeStringSlice::from_slice",
                            arguments: vec![RustBridgeExpr::BorrowPath("mime_type_refs")],
                        },
                    },
                    RustBridgeFieldInitializer {
                        name: "extensions",
                        value: RustBridgeExpr::FunctionCall {
                            function: "NativeStringSlice::from_slice",
                            arguments: vec![RustBridgeExpr::BorrowPath("extension_refs")],
                        },
                    },
                    RustBridgeFieldInitializer {
                        name: "allows_multiple_selection",
                        value: RustBridgeExpr::Path("options.multiple"),
                    },
                    RustBridgeFieldInitializer {
                        name: "allows_directory_selection",
                        value: RustBridgeExpr::Path("options.allow_directories"),
                    },
                    RustBridgeFieldInitializer {
                        name: "copies_to_sandbox",
                        value: RustBridgeExpr::Path("options.copy_to_sandbox"),
                    },
                ],
            },
            accessor_documentation: "Return the ABI request view.",
        }],
    }
}

/// Build the generated Rust bridge file for media helpers.
fn build_media_bridge_file_spec() -> RustBridgeFileSpec {
    let mut imports = BTreeSet::new();
    imports.insert("crate::diagnostic::{RuntimeError, RuntimeResult};");
    imports.insert("crate::host::abi::media::{HostMediaAssetDescriptor, HostMediaAssetKind, HostMediaDeleteRequest, HostMediaDeleteResponse, HostMediaImportPathRequest, HostMediaImportPathResponse, HostMediaListRequest, HostMediaListResponse, HostMediaListResult, HostMediaReadResponse};");
    imports.insert("crate::host::core::callback::decode_callback_host_required_string_ref;");
    imports.insert("crate::platform::PlatformError;");
    imports.insert("crate::platform::abi::NativeStringRef;");
    imports.insert("crate::platform::os::MediaAssetKind;");
    imports.insert(
        "crate::platform::os::abi_generated::{MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue};",
    );
    imports.insert("crate::runtime::BindingCallContext;");

    RustBridgeFileSpec {
        file_name: "runtime.rs",
        imports,
        nested_modules: Vec::new(),
        reexports: Vec::new(),
        helpers: Vec::new(),
        helper_functions: vec![
            RustBridgeFunctionSpec {
                documentation: "Encode one media-list request payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_media_list_request",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "query",
                        ty: "&MediaQueryValue",
                    },
                ],
                return_type: "HostMediaListRequest",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostMediaListRequest { has_cursor: query.cursor.is_some(), cursor: binding.store_string(query.cursor.as_deref().unwrap_or(\"\")), has_limit: query.limit.is_some(), limit: query.limit.unwrap_or_default(), kinds: binding.store_slice(query.kinds.iter().copied().map(encode_media_asset_kind).collect()), include_hidden: query.include_hidden }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one media-import request payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_media_import_path_request",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "path",
                        ty: "&str",
                    },
                    RustBridgeParameter {
                        name: "kind",
                        ty: "MediaAssetKind",
                    },
                ],
                return_type: "HostMediaImportPathRequest",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostMediaImportPathRequest { path: binding.store_string(path), kind: encode_media_asset_kind(kind) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one media-delete request payload for the host ABI.",
                qualifiers: vec!["pub(crate)"],
                name: "encode_media_delete_request",
                parameters: vec![
                    RustBridgeParameter {
                        name: "binding",
                        ty: "&BindingCallContext",
                    },
                    RustBridgeParameter {
                        name: "identifiers",
                        ty: "&[String]",
                    },
                ],
                return_type: "HostMediaDeleteRequest",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "HostMediaDeleteRequest { identifiers: binding.store_string_slice(identifiers.iter().map(NativeStringRef::from).collect()) }",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media-list response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_media_list_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostMediaListResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<MediaPageValue>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_page"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.page\", format!(\"{operation} returned one missing response.page\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "unsafe { decode_media_list_result(response.page, operation) }",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media-read response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_media_read_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostMediaReadResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<MediaAssetDescriptorValue>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_descriptor"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.descriptor\", format!(\"{operation} returned one missing response.descriptor\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "decode_media_asset_descriptor(response.descriptor, operation)",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media-import response payload.",
                qualifiers: vec!["pub(crate)", "unsafe"],
                name: "decode_media_import_path_response",
                parameters: vec![
                    RustBridgeParameter {
                        name: "response",
                        ty: "HostMediaImportPathResponse",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<String>",
                body: vec![
                    RustBridgeStatement::If {
                        condition: RustBridgeExpr::Path("!response.has_identifier"),
                        then_body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                            "Err(RuntimeError::from(PlatformError::invalid_argument_value(\"response.identifier\", format!(\"{operation} returned one missing response.identifier\"))).boxed())",
                        ))],
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "decode_callback_host_required_string_ref(operation, \"HostMediaImportPathResponse.identifier\", response.identifier)",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media-delete response payload.",
                qualifiers: vec!["pub(crate)"],
                name: "decode_media_delete_response",
                parameters: vec![RustBridgeParameter {
                    name: "response",
                    ty: "HostMediaDeleteResponse",
                }],
                return_type: "u32",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "response.deleted_count",
                ))],
            },
            RustBridgeFunctionSpec {
                documentation: "Encode one runtime media asset kind as one host media asset kind.",
                qualifiers: Vec::new(),
                name: "encode_media_asset_kind",
                parameters: vec![RustBridgeParameter {
                    name: "kind",
                    ty: "MediaAssetKind",
                }],
                return_type: "HostMediaAssetKind",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "kind",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "MediaAssetKind::Image",
                            value: RustBridgeExpr::Path("HostMediaAssetKind::Image"),
                        },
                        RustBridgeMatchArm {
                            pattern: "MediaAssetKind::Video",
                            value: RustBridgeExpr::Path("HostMediaAssetKind::Video"),
                        },
                        RustBridgeMatchArm {
                            pattern: "MediaAssetKind::Audio",
                            value: RustBridgeExpr::Path("HostMediaAssetKind::Audio"),
                        },
                        RustBridgeMatchArm {
                            pattern: "MediaAssetKind::Other",
                            value: RustBridgeExpr::Path("HostMediaAssetKind::Other"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media asset kind into one runtime media asset kind.",
                qualifiers: Vec::new(),
                name: "decode_media_asset_kind",
                parameters: vec![RustBridgeParameter {
                    name: "kind",
                    ty: "HostMediaAssetKind",
                }],
                return_type: "MediaAssetKind",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Match {
                    scrutinee: "kind",
                    arms: vec![
                        RustBridgeMatchArm {
                            pattern: "HostMediaAssetKind::Image",
                            value: RustBridgeExpr::Path("MediaAssetKind::Image"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostMediaAssetKind::Video",
                            value: RustBridgeExpr::Path("MediaAssetKind::Video"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostMediaAssetKind::Audio",
                            value: RustBridgeExpr::Path("MediaAssetKind::Audio"),
                        },
                        RustBridgeMatchArm {
                            pattern: "HostMediaAssetKind::Other",
                            value: RustBridgeExpr::Path("MediaAssetKind::Other"),
                        },
                    ],
                })],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media-list result payload.",
                qualifiers: vec!["unsafe"],
                name: "decode_media_list_result",
                parameters: vec![
                    RustBridgeParameter {
                        name: "page",
                        ty: "HostMediaListResult",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<MediaPageValue>",
                body: vec![
                    RustBridgeStatement::Let {
                        name: "assets",
                        value: RustBridgeExpr::Path("unsafe { page.assets.as_slice()? }"),
                    },
                    RustBridgeStatement::Let {
                        name: "assets",
                        value: RustBridgeExpr::Path(
                            "assets.iter().copied().map(|asset| decode_media_asset_descriptor(asset, operation)).collect::<RuntimeResult<Vec<_>>>()?",
                        ),
                    },
                    RustBridgeStatement::Let {
                        name: "next_cursor",
                        value: RustBridgeExpr::Path(
                            "decode_callback_host_required_string_ref(operation, \"HostMediaListResult.next_cursor\", page.next_cursor)?",
                        ),
                    },
                    RustBridgeStatement::Return(RustBridgeExpr::Path(
                        "Ok(MediaPageValue { assets, next_cursor, has_more: page.has_more })",
                    )),
                ],
            },
            RustBridgeFunctionSpec {
                documentation: "Decode one host media asset-descriptor payload.",
                qualifiers: Vec::new(),
                name: "decode_media_asset_descriptor",
                parameters: vec![
                    RustBridgeParameter {
                        name: "descriptor",
                        ty: "HostMediaAssetDescriptor",
                    },
                    RustBridgeParameter {
                        name: "operation",
                        ty: "&'static str",
                    },
                ],
                return_type: "RuntimeResult<MediaAssetDescriptorValue>",
                body: vec![RustBridgeStatement::Return(RustBridgeExpr::Path(
                    "Ok(MediaAssetDescriptorValue { id: decode_callback_host_required_string_ref(operation, \"HostMediaAssetDescriptor.identifier\", descriptor.identifier)?, uri: decode_callback_host_required_string_ref(operation, \"HostMediaAssetDescriptor.uri\", descriptor.uri)?, filename: decode_callback_host_required_string_ref(operation, \"HostMediaAssetDescriptor.filename\", descriptor.filename)?, mime_type: decode_callback_host_required_string_ref(operation, \"HostMediaAssetDescriptor.mime_type\", descriptor.mime_type)?, kind: decode_media_asset_kind(descriptor.kind), width: descriptor.width, height: descriptor.height, duration_ms: descriptor.duration_ms, size_bytes: descriptor.size_bytes, created_unix_ns: descriptor.created_unix_ns, modified_unix_ns: descriptor.modified_unix_ns })",
                ))],
            },
        ],
        payloads: Vec::new(),
    }
}

/// Build the generated Rust bridge file for permission payloads.
fn build_permission_bridge_file_spec() -> RustBridgeFileSpec {
    let mut imports = BTreeSet::new();
    imports.insert("crate::platform::abi::NativeStringRef;");
    imports.insert("crate::platform::os::Permission;");
    imports.insert("crate::host::abi::permission::HostPermissionRequest;");

    RustBridgeFileSpec {
        file_name: "runtime.rs",
        imports,
        nested_modules: Vec::new(),
        reexports: Vec::new(),
        helpers: vec![
            r#"/// Return the canonical host permission name.
pub(crate) const fn permission_name(permission: Permission) -> &'static str {
    match permission {
        Permission::Location => "location",
        Permission::LocationBackground => "locationBackground",
        Permission::Camera => "camera",
        Permission::Microphone => "microphone",
        Permission::Bluetooth => "bluetooth",
        Permission::Notifications => "notifications",
        Permission::ContactsRead => "contactsRead",
        Permission::ContactsWrite => "contactsWrite",
        Permission::MediaRead => "mediaRead",
        Permission::MediaWrite => "mediaWrite",
        Permission::Motion => "motion",
        Permission::ClipboardRead => "clipboardRead",
        Permission::CalendarRead => "calendarRead",
        Permission::CalendarWrite => "calendarWrite",
    }
}"#,
        ],
        helper_functions: Vec::new(),
        payloads: vec![RustBridgePayloadSpec {
            documentation: "One owned host permission request payload.",
            struct_name: "HostPermissionRequestPayload",
            storage_fields: vec![RustBridgeStorageField {
                documentation: "The owned permission-name backing storage.",
                name: "_permission_storage",
                ty: "String",
            }],
            abi_documentation: "The borrowed ABI request view.",
            abi_type: "HostPermissionRequest",
            constructor_documentation: "Build one owned single-permission request payload.",
            constructor_name: "single",
            constructor_parameters: vec![
                RustBridgeParameter {
                    name: "request_id",
                    ty: "u64",
                },
                RustBridgeParameter {
                    name: "permission",
                    ty: "Permission",
                },
            ],
            local_bindings: vec![RustBridgeLocal {
                name: "permission_storage",
                value: RustBridgeExpr::MethodCall {
                    receiver: Box::new(RustBridgeExpr::FunctionCall {
                        function: "permission_name",
                        arguments: vec![RustBridgeExpr::Path("permission")],
                    }),
                    method: "to_string",
                    arguments: Vec::new(),
                },
            }],
            abi_initializer: RustBridgeExpr::StructInit {
                ty: "HostPermissionRequest",
                fields: vec![
                    RustBridgeFieldInitializer {
                        name: "request_id",
                        value: RustBridgeExpr::Path("request_id"),
                    },
                    RustBridgeFieldInitializer {
                        name: "permission",
                        value: RustBridgeExpr::FunctionCall {
                            function: "NativeStringRef::from",
                            arguments: vec![RustBridgeExpr::BorrowPath("permission_storage")],
                        },
                    },
                ],
            },
            accessor_documentation: "Return the ABI request view.",
        }],
    }
}

/// Build the generated Rust bridge file for text payloads.
fn build_text_bridge_file_spec() -> RustBridgeFileSpec {
    let mut imports = BTreeSet::new();
    imports.insert("crate::platform::NativeAbiCodec;");
    imports.insert(
        "crate::platform::abi::{NativeStringRef, NativeStringRef as PlatformNativeStringRef};",
    );
    imports.insert(
        "crate::platform::input::{\
InputTextInputType, InputTextRange, InputTextRectangle, InputTextSessionConfig, \
InputTextSessionStateValue, InputTextTransform2D\
};",
    );
    imports.insert(
        "crate::host::abi::text::{\
HostTextInputConfiguration, HostTextInputGeometry, HostTextInputGeometryRequest, \
HostTextInputOpenRequest, HostTextInputRange, HostTextInputRectangle, HostTextInputState, \
HostTextInputStateRequest, HostTextInputTransform2D, HostTextInputType\
};",
    );

    RustBridgeFileSpec {
        file_name: "runtime.rs",
        imports,
        nested_modules: Vec::new(),
        reexports: Vec::new(),
        helpers: vec![
            r#"/// Convert one runtime text-input type into one host payload.
fn host_text_input_type(input_type: InputTextInputType) -> HostTextInputType {
    match input_type {
        InputTextInputType::Text => HostTextInputType::Text,
        InputTextInputType::Number => HostTextInputType::Number,
        InputTextInputType::Email => HostTextInputType::Email,
        InputTextInputType::Url => HostTextInputType::Url,
        InputTextInputType::Password => HostTextInputType::Password,
        InputTextInputType::Phone => HostTextInputType::Phone,
        InputTextInputType::Search => HostTextInputType::Search,
    }
}"#,
            r#"/// Convert one runtime text state into one host payload.
fn host_text_state(text: &str, state: &InputTextSessionStateValue) -> HostTextInputState {
    let composing = if let Some(composing) = state.composing {
        host_text_range(composing)
    } else {
        HostTextInputRange::default()
    };

    HostTextInputState {
        text: NativeStringRef::from(text),
        selection: host_text_range(state.selection),
        has_composing: state.composing.is_some(),
        composing,
    }
}"#,
            r#"/// Convert one runtime text range into one host payload.
fn host_text_range(range: InputTextRange) -> HostTextInputRange {
    HostTextInputRange {
        start_offset: range.start_offset,
        end_offset: range.end_offset,
    }
}"#,
            r#"/// Convert one runtime text rectangle into one host payload.
fn host_text_rectangle(rectangle: InputTextRectangle) -> HostTextInputRectangle {
    HostTextInputRectangle {
        x: rectangle.x,
        y: rectangle.y,
        width: rectangle.width,
        height: rectangle.height,
    }
}"#,
            r#"/// Convert one runtime text transform into one host payload.
fn host_text_transform(transform: InputTextTransform2D) -> HostTextInputTransform2D {
    HostTextInputTransform2D {
        xx: transform.xx,
        xy: transform.xy,
        yx: transform.yx,
        yy: transform.yy,
        tx: transform.tx,
        ty: transform.ty,
    }
}"#,
            r#"/// Decode one host text state payload into one runtime value.
pub(crate) unsafe fn decode_text_input_state(
    state: HostTextInputState,
) -> InputTextSessionStateValue {
    let text = unsafe { <PlatformNativeStringRef as NativeAbiCodec>::into_value(state.text) }
        .unwrap_or_default();

    InputTextSessionStateValue {
        text,
        selection: InputTextRange {
            start_offset: state.selection.start_offset,
            end_offset: state.selection.end_offset,
        },
        composing: if state.has_composing {
            Some(InputTextRange {
                start_offset: state.composing.start_offset,
                end_offset: state.composing.end_offset,
            })
        } else {
            None
        },
    }
}"#,
        ],
        helper_functions: Vec::new(),
        payloads: vec![
            RustBridgePayloadSpec {
                documentation: "One owned host text open request payload.",
                struct_name: "HostTextOpenRequestPayload",
                storage_fields: vec![RustBridgeStorageField {
                    documentation: "The owned state text storage.",
                    name: "_text_storage",
                    ty: "String",
                }],
                abi_documentation: "The borrowed ABI payload.",
                abi_type: "HostTextInputOpenRequest",
                constructor_documentation: "Build one owned host text open request payload.",
                constructor_name: "new",
                constructor_parameters: vec![
                    RustBridgeParameter {
                        name: "session_id",
                        ty: "u64",
                    },
                    RustBridgeParameter {
                        name: "config",
                        ty: "InputTextSessionConfig",
                    },
                    RustBridgeParameter {
                        name: "state",
                        ty: "&InputTextSessionStateValue",
                    },
                ],
                local_bindings: vec![RustBridgeLocal {
                    name: "text_storage",
                    value: RustBridgeExpr::ClonePath("state.text"),
                }],
                abi_initializer: RustBridgeExpr::StructInit {
                    ty: "HostTextInputOpenRequest",
                    fields: vec![
                        RustBridgeFieldInitializer {
                            name: "configuration",
                            value: RustBridgeExpr::StructInit {
                                ty: "HostTextInputConfiguration",
                                fields: vec![
                                    RustBridgeFieldInitializer {
                                        name: "session_id",
                                        value: RustBridgeExpr::Path("session_id"),
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "input_type",
                                        value: RustBridgeExpr::FunctionCall {
                                            function: "host_text_input_type",
                                            arguments: vec![RustBridgeExpr::Path(
                                                "config.input_type",
                                            )],
                                        },
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "is_multiline",
                                        value: RustBridgeExpr::Path("config.is_multiline"),
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "is_secure",
                                        value: RustBridgeExpr::Path("config.is_secure"),
                                    },
                                ],
                            },
                        },
                        RustBridgeFieldInitializer {
                            name: "state",
                            value: RustBridgeExpr::FunctionCall {
                                function: "host_text_state",
                                arguments: vec![
                                    RustBridgeExpr::BorrowPath("text_storage"),
                                    RustBridgeExpr::Path("state"),
                                ],
                            },
                        },
                    ],
                },
                accessor_documentation: "Return the ABI request view.",
            },
            RustBridgePayloadSpec {
                documentation: "One owned host text-state update payload.",
                struct_name: "HostTextStateRequestPayload",
                storage_fields: vec![RustBridgeStorageField {
                    documentation: "The owned state text storage.",
                    name: "_text_storage",
                    ty: "String",
                }],
                abi_documentation: "The borrowed ABI payload.",
                abi_type: "HostTextInputStateRequest",
                constructor_documentation: "Build one owned host text-state update payload.",
                constructor_name: "new",
                constructor_parameters: vec![
                    RustBridgeParameter {
                        name: "session_id",
                        ty: "u64",
                    },
                    RustBridgeParameter {
                        name: "state",
                        ty: "&InputTextSessionStateValue",
                    },
                ],
                local_bindings: vec![RustBridgeLocal {
                    name: "text_storage",
                    value: RustBridgeExpr::ClonePath("state.text"),
                }],
                abi_initializer: RustBridgeExpr::StructInit {
                    ty: "HostTextInputStateRequest",
                    fields: vec![
                        RustBridgeFieldInitializer {
                            name: "session_id",
                            value: RustBridgeExpr::Path("session_id"),
                        },
                        RustBridgeFieldInitializer {
                            name: "state",
                            value: RustBridgeExpr::FunctionCall {
                                function: "host_text_state",
                                arguments: vec![
                                    RustBridgeExpr::BorrowPath("text_storage"),
                                    RustBridgeExpr::Path("state"),
                                ],
                            },
                        },
                    ],
                },
                accessor_documentation: "Return the ABI request view.",
            },
            RustBridgePayloadSpec {
                documentation: "One owned host text-geometry update payload.",
                struct_name: "HostTextGeometryRequestPayload",
                storage_fields: Vec::new(),
                abi_documentation: "The borrowed ABI payload.",
                abi_type: "HostTextInputGeometryRequest",
                constructor_documentation: "Build one owned host text-geometry update payload.",
                constructor_name: "new",
                constructor_parameters: vec![
                    RustBridgeParameter {
                        name: "session_id",
                        ty: "u64",
                    },
                    RustBridgeParameter {
                        name: "local_to_target_transform",
                        ty: "InputTextTransform2D",
                    },
                    RustBridgeParameter {
                        name: "editor_rectangle",
                        ty: "InputTextRectangle",
                    },
                    RustBridgeParameter {
                        name: "caret_rectangle",
                        ty: "Option<InputTextRectangle>",
                    },
                    RustBridgeParameter {
                        name: "composing_rectangle",
                        ty: "Option<InputTextRectangle>",
                    },
                ],
                local_bindings: Vec::new(),
                abi_initializer: RustBridgeExpr::StructInit {
                    ty: "HostTextInputGeometryRequest",
                    fields: vec![
                        RustBridgeFieldInitializer {
                            name: "session_id",
                            value: RustBridgeExpr::Path("session_id"),
                        },
                        RustBridgeFieldInitializer {
                            name: "geometry",
                            value: RustBridgeExpr::StructInit {
                                ty: "HostTextInputGeometry",
                                fields: vec![
                                    RustBridgeFieldInitializer {
                                        name: "local_to_target_transform",
                                        value: RustBridgeExpr::FunctionCall {
                                            function: "host_text_transform",
                                            arguments: vec![RustBridgeExpr::Path(
                                                "local_to_target_transform",
                                            )],
                                        },
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "editor_rectangle",
                                        value: RustBridgeExpr::FunctionCall {
                                            function: "host_text_rectangle",
                                            arguments: vec![RustBridgeExpr::Path(
                                                "editor_rectangle",
                                            )],
                                        },
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "has_caret_rectangle",
                                        value: RustBridgeExpr::IsSome("caret_rectangle"),
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "caret_rectangle",
                                        value: RustBridgeExpr::OptionMapOrDefault {
                                            source: "caret_rectangle",
                                            mapper: "host_text_rectangle",
                                        },
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "has_composing_rectangle",
                                        value: RustBridgeExpr::IsSome("composing_rectangle"),
                                    },
                                    RustBridgeFieldInitializer {
                                        name: "composing_rectangle",
                                        value: RustBridgeExpr::OptionMapOrDefault {
                                            source: "composing_rectangle",
                                            mapper: "host_text_rectangle",
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                },
                accessor_documentation: "Return the ABI request view.",
            },
        ],
    }
}

/// Render one generated Rust bridge file.
fn render_rust_bridge_file_spec(spec: &RustBridgeFileSpec) -> String {
    let mut output = String::new();

    let imports = spec
        .imports
        .iter()
        .map(|import| format!("use {import}"))
        .collect::<Vec<_>>();
    let import_refs = imports.iter().map(String::as_str).collect::<Vec<_>>();
    render_rust_bridge_file_header(&mut output, &import_refs);

    for nested_module in &spec.nested_modules {
        output.push_str(nested_module);
        output.push('\n');
    }

    for reexport in &spec.reexports {
        output.push_str(reexport);
        output.push('\n');
    }

    if !spec.nested_modules.is_empty() || !spec.reexports.is_empty() {
        output.push('\n');
    }

    for helper in &spec.helpers {
        output.push_str(helper);
        output.push_str("\n\n");
    }

    for helper_function in &spec.helper_functions {
        render_rust_bridge_function(&mut output, helper_function);
        output.push('\n');
    }

    for payload in &spec.payloads {
        render_rust_bridge_payload(&mut output, payload);
        output.push('\n');
    }

    output
}

/// Render one generated Rust bridge file header.
fn render_rust_bridge_file_header(output: &mut String, imports: &[&str]) {
    output.push_str("// generated by generate-bindings: do not edit\n\n");

    for import in imports {
        output.push_str(import);
        output.push('\n');
    }

    if !imports.is_empty() {
        output.push('\n');
    }
}

/// Render one cfg-gated generated Rust bridge helper function.
fn render_cfg_rust_bridge_function(
    output: &mut String,
    cfg: &'static str,
    function: &RustBridgeFunctionSpec,
) {
    push_rust_doc_comment(output, function.documentation);
    output.push_str(&format!("#[cfg({cfg})]\n"));

    let qualifiers = if function.qualifiers.is_empty() {
        String::new()
    } else {
        format!("{} ", function.qualifiers.join(" "))
    };

    output.push_str(&format!("{qualifiers}fn {}(\n", function.name));
    for parameter in &function.parameters {
        output.push_str(&format!("    {}: {},\n", parameter.name, parameter.ty));
    }
    output.push_str(&format!(") -> {} {{\n", function.return_type));

    for (index, statement) in function.body.iter().enumerate() {
        let is_last = index + 1 == function.body.len();
        render_rust_bridge_statement(output, statement, 1, is_last, true);
    }

    output.push_str("}\n");
}

/// Render one cfg-gated generated Rust bridge payload.
fn render_cfg_rust_bridge_payload(
    output: &mut String,
    cfg: &'static str,
    payload: &RustBridgePayloadSpec,
) {
    push_rust_doc_comment(output, payload.documentation);
    output.push_str(&format!("#[cfg({cfg})]\n"));
    output.push_str("#[derive(Debug)]\n");
    output.push_str(&format!("pub(crate) struct {} {{\n", payload.struct_name));

    for field in &payload.storage_fields {
        push_indented_rust_doc_comment(output, "    ", field.documentation);
        output.push_str(&format!("    {}: {},\n", field.name, field.ty));
    }

    push_indented_rust_doc_comment(output, "    ", payload.abi_documentation);
    output.push_str(&format!("    abi: {},\n", payload.abi_type));
    output.push_str("}\n\n");

    output.push_str(&format!("#[cfg({cfg})]\n"));
    output.push_str(&format!("impl {} {{\n", payload.struct_name));
    push_indented_rust_doc_comment(output, "    ", payload.constructor_documentation);
    output.push_str(&format!(
        "    pub(crate) fn {}(\n",
        payload.constructor_name
    ));
    for parameter in &payload.constructor_parameters {
        output.push_str(&format!("        {}: {},\n", parameter.name, parameter.ty));
    }
    output.push_str("    ) -> Self {\n");
    for local in &payload.local_bindings {
        output.push_str(&format!(
            "        let {} = {};\n",
            local.name,
            render_rust_bridge_expr(&local.value, 2)
        ));
    }
    if !payload.local_bindings.is_empty() {
        output.push('\n');
    }
    output.push_str(&format!(
        "        let abi = {};\n\n",
        render_rust_bridge_expr(&payload.abi_initializer, 2)
    ));
    output.push_str("        Self {\n");
    for field in &payload.storage_fields {
        let local_name = field.name.trim_start_matches('_');
        output.push_str(&format!("            {}: {},\n", field.name, local_name));
    }
    output.push_str("            abi,\n");
    output.push_str("        }\n");
    output.push_str("    }\n\n");

    push_indented_rust_doc_comment(output, "    ", payload.accessor_documentation);
    output.push_str(&format!(
        "    pub(crate) fn abi(&self) -> {} {{\n",
        payload.abi_type
    ));
    output.push_str("        self.abi\n");
    output.push_str("    }\n");
    output.push_str("}\n");
}

/// Render the background descriptor `NativeAbiCodec` implementation.
fn render_background_descriptor_codec_impl(output: &mut String) {
    output.push_str("impl NativeAbiCodec for HostBackgroundTaskDescriptor {\n");
    output.push_str("    type Value = BackgroundTaskDescriptorValue;\n\n");
    output.push_str("    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {\n");
    output.push_str("        Ok(BackgroundTaskDescriptorValue {\n");
    output.push_str("            identifier: decode_string(self.identifier)?,\n");
    output.push_str("            trigger: decode_trigger(self.trigger),\n");
    output.push_str("            schedule: decode_schedule(self.schedule),\n");
    output.push_str("            network: decode_network_requirement(self.network),\n");
    output.push_str("            requires_charging: self.requires_charging,\n");
    output.push_str("            requires_idle: self.requires_idle,\n");
    output.push_str("            conflict_policy: decode_conflict_policy(self.conflict_policy),\n");
    output.push_str("        })\n");
    output.push_str("    }\n\n");
    output.push_str(
        "    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {\n",
    );
    output.push_str("        Self {\n");
    output.push_str(
        "            identifier: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.identifier),\n",
    );
    output.push_str("            trigger: encode_trigger(value.trigger),\n");
    output.push_str("            schedule: encode_schedule(value.schedule),\n");
    output.push_str("            network: encode_network_requirement(value.network),\n");
    output.push_str("            requires_charging: value.requires_charging,\n");
    output.push_str("            requires_idle: value.requires_idle,\n");
    output
        .push_str("            conflict_policy: encode_conflict_policy(value.conflict_policy),\n");
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("}\n");
}

/// Render one generated Rust bridge helper function.
fn render_rust_bridge_function(output: &mut String, function: &RustBridgeFunctionSpec) {
    push_rust_doc_comment(output, function.documentation);

    let qualifiers = if function.qualifiers.is_empty() {
        String::new()
    } else {
        format!("{} ", function.qualifiers.join(" "))
    };

    output.push_str(&format!("{qualifiers}fn {}(\n", function.name));
    for parameter in &function.parameters {
        output.push_str(&format!("    {}: {},\n", parameter.name, parameter.ty));
    }
    output.push_str(&format!(") -> {} {{\n", function.return_type));

    for (index, statement) in function.body.iter().enumerate() {
        let is_last = index + 1 == function.body.len();
        render_rust_bridge_statement(output, statement, 1, is_last, true);
    }

    output.push_str("}\n");
}

/// Render one generated Rust bridge payload.
fn render_rust_bridge_payload(output: &mut String, payload: &RustBridgePayloadSpec) {
    push_rust_doc_comment(output, payload.documentation);
    output.push_str("#[derive(Debug)]\n");
    output.push_str(&format!("pub(crate) struct {} {{\n", payload.struct_name));

    for field in &payload.storage_fields {
        push_indented_rust_doc_comment(output, "    ", field.documentation);
        output.push_str(&format!("    {}: {},\n", field.name, field.ty));
    }

    push_indented_rust_doc_comment(output, "    ", payload.abi_documentation);
    output.push_str(&format!("    abi: {},\n", payload.abi_type));
    output.push_str("}\n\n");

    output.push_str(&format!("impl {} {{\n", payload.struct_name));
    push_indented_rust_doc_comment(output, "    ", payload.constructor_documentation);
    output.push_str(&format!(
        "    pub(crate) fn {}(\n",
        payload.constructor_name
    ));
    for parameter in &payload.constructor_parameters {
        output.push_str(&format!("        {}: {},\n", parameter.name, parameter.ty));
    }
    output.push_str("    ) -> Self {\n");
    for local in &payload.local_bindings {
        output.push_str(&format!(
            "        let {} = {};\n",
            local.name,
            render_rust_bridge_expr(&local.value, 2)
        ));
    }
    if !payload.local_bindings.is_empty() {
        output.push('\n');
    }
    output.push_str(&format!(
        "        let abi = {};\n\n",
        render_rust_bridge_expr(&payload.abi_initializer, 2)
    ));
    output.push_str("        Self {\n");
    for field in &payload.storage_fields {
        let local_name = field.name.trim_start_matches('_');
        output.push_str(&format!("            {}: {},\n", field.name, local_name));
    }
    output.push_str("            abi,\n");
    output.push_str("        }\n");
    output.push_str("    }\n\n");

    push_indented_rust_doc_comment(output, "    ", payload.accessor_documentation);
    output.push_str(&format!(
        "    pub(crate) fn abi(&self) -> {} {{\n",
        payload.abi_type
    ));
    output.push_str("        self.abi\n");
    output.push_str("    }\n");
    output.push_str("}\n");
}

/// Render one generated Rust bridge statement.
fn render_rust_bridge_statement(
    output: &mut String,
    statement: &RustBridgeStatement,
    indent: usize,
    is_last: bool,
    allow_implicit_tail: bool,
) {
    let indentation = "    ".repeat(indent);

    match statement {
        RustBridgeStatement::Let { name, value } => {
            output.push_str(&indentation);
            output.push_str(&format!(
                "let {name} = {};\n",
                render_rust_bridge_expr(value, indent)
            ));
        }
        RustBridgeStatement::Return(value) => {
            output.push_str(&indentation);
            if allow_implicit_tail && is_last {
                output.push_str(&render_rust_bridge_expr(value, indent));
            } else {
                output.push_str("return ");
                output.push_str(&render_rust_bridge_expr(value, indent));
                output.push(';');
            }
            output.push('\n');
        }
        RustBridgeStatement::If {
            condition,
            then_body,
        } => {
            output.push_str(&indentation);
            output.push_str(&format!(
                "if {} {{\n",
                render_rust_bridge_expr(condition, indent)
            ));
            for (index, nested_statement) in then_body.iter().enumerate() {
                let is_last_nested = index + 1 == then_body.len();
                render_rust_bridge_statement(
                    output,
                    nested_statement,
                    indent + 1,
                    is_last_nested,
                    false,
                );
            }
            output.push_str(&indentation);
            output.push_str("}\n");
        }
    }
}

/// Render one generated Rust bridge expression.
fn render_rust_bridge_expr(expr: &RustBridgeExpr, indent: usize) -> String {
    match expr {
        RustBridgeExpr::Path(path) => (*path).to_string(),
        RustBridgeExpr::ClonePath(path) => format!("{path}.clone()"),
        RustBridgeExpr::BorrowPath(path) => format!("&{path}"),
        RustBridgeExpr::FunctionCall {
            function,
            arguments,
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| render_rust_bridge_expr(argument, indent))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{function}({arguments})")
        }
        RustBridgeExpr::MethodCall {
            receiver,
            method,
            arguments,
        } => {
            let receiver = render_rust_bridge_expr(receiver, indent);
            let arguments = arguments
                .iter()
                .map(|argument| render_rust_bridge_expr(argument, indent))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{receiver}.{method}({arguments})")
        }
        RustBridgeExpr::StructInit { ty, fields } => {
            let indentation = "    ".repeat(indent);
            let field_indentation = "    ".repeat(indent + 1);
            let mut output = String::new();

            output.push_str(&format!("{ty} {{\n"));
            for field in fields {
                output.push_str(&field_indentation);
                output.push_str(field.name);
                output.push_str(": ");
                output.push_str(&render_rust_bridge_expr(&field.value, indent + 1));
                output.push_str(",\n");
            }
            output.push_str(&indentation);
            output.push('}');

            output
        }
        RustBridgeExpr::IterMapCollectVec { source, mapper } => {
            format!("{source}.iter().map({mapper}).collect::<Vec<_>>()")
        }
        RustBridgeExpr::IsSome(source) => format!("{source}.is_some()"),
        RustBridgeExpr::OptionMapOrDefault { source, mapper } => {
            format!("{source}.map({mapper}).unwrap_or_default()")
        }
        RustBridgeExpr::Match { scrutinee, arms } => {
            let indentation = "    ".repeat(indent);
            let arm_indentation = "    ".repeat(indent + 1);
            let mut output = String::new();

            output.push_str(&format!("match {scrutinee} {{\n"));
            for arm in arms {
                output.push_str(&arm_indentation);
                output.push_str(arm.pattern);
                output.push_str(" => ");
                output.push_str(&render_rust_bridge_expr(&arm.value, indent + 1));
                output.push_str(",\n");
            }
            output.push_str(&indentation);
            output.push('}');

            output
        }
    }
}

/// Render one generated Rust ingress file for one platform and module.
fn render_platform_ingress_files(
    layout: &WorkspaceLayout,
    module: &HostAbiModule,
    platform: HostPlatform,
) -> Vec<HostArtifact> {
    if !module_uses_generated_ingress(module) {
        return Vec::new();
    }

    let platform_segment = platform.segment();
    let contents = render_ingress(module, platform);

    vec![HostArtifact {
        path: layout.language_root.join(format!(
            "runtime/src/host/{platform_segment}/abi/ingress/{}.generated.rs",
            module.name
        )),
        contents,
    }]
}

/// Render one generated Rust callbacks file.
fn render_callbacks(module: &HostAbiModule, platform: HostPlatform) -> String {
    let mut output = String::new();
    let platform_label = platform.label();
    let platform_prefix = platform.prefix();
    let callback_struct_name = callback_struct_name(module, platform);

    output.push_str("// generated by generate-bindings: do not edit\n\n");

    let mut abi_types = referenced_named_types(module);
    abi_types.sort();
    abi_types.dedup();

    if !abi_types.is_empty() {
        output.push_str(&format!(
            "use crate::host::abi::{}::{{{}}};\n",
            module.name,
            abi_types.join(", ")
        ));
    }
    if module_uses_native_array(module) {
        output.push_str("use crate::platform::NativeArray;\n");
    }
    if module_uses_native_slice(module) || module_uses_string_slice(module) {
        let mut native_types = Vec::new();
        if module_uses_native_slice(module) {
            native_types.push("NativeSlice");
        }
        if module_uses_string_slice(module) {
            native_types.push("NativeStringSlice");
        }
        output.push_str(&format!(
            "use crate::platform::abi::{{{}}};\n",
            native_types.join(", ")
        ));
    }
    if module_uses_string_ref(module) {
        output.push_str("use crate::platform::abi::NativeStringRef;\n");
    }
    output.push_str(&format!(
        "use crate::host::{platform_segment}::abi::bindings::invoke_{platform_prefix}_binding_callback;\n\n",
        platform_segment = platform.segment()
    ));

    for request in &module.requests {
        let field_name = rust_request_field_name(module, request);

        push_rust_doc_comment(
            &mut output,
            &format!("The host callback for `{field_name}`."),
        );
        output.push_str(&format!(
            "pub(crate) type {} =\n    unsafe extern \"C\" fn({}) -> u32;\n\n",
            callback_type_name(module, request, platform),
            render_callback_parameter_list(request)
        ));
    }

    push_rust_doc_comment(
        &mut output,
        &format!(
            "Callback table for {platform_label} host {} request interop.",
            module.name
        ),
    );
    output.push_str("#[derive(Clone, Copy, Debug, Default)]\n");
    output.push_str("#[repr(C)]\n");
    output.push_str(&format!("pub(crate) struct {callback_struct_name} {{\n"));

    for request in &module.requests {
        let field_name = rust_request_field_name(module, request);
        push_indented_rust_doc_comment(
            &mut output,
            "    ",
            &format!("The callback for `{field_name}`."),
        );
        output.push_str(&format!(
            "    pub {}: Option<{}>,\n",
            field_name,
            callback_type_name(module, request, platform)
        ));
    }

    output.push_str("}\n\n");

    push_rust_doc_comment(
        &mut output,
        &format!(
            "Resolve and invoke one {platform_label} host {} callback.",
            module.name
        ),
    );
    output.push_str("pub(super) fn ");
    output.push_str(&format!(
        "call_{}_{}_callback<T: Copy>(\n",
        platform_prefix, module.name
    ));
    output.push_str("    session_handle: u64,\n");
    output.push_str(&format!(
        "    resolve: impl FnOnce(&{callback_struct_name}) -> Option<T>,\n"
    ));
    output.push_str("    invoke: impl FnOnce(T) -> u32,\n");
    output.push_str(") -> u32 {\n");
    output.push_str(&format!(
        "    invoke_{}_binding_callback(session_handle, |bindings| resolve(&bindings.{}), invoke)\n",
        platform_prefix, module.name
    ));
    output.push_str("}\n");

    output
}

/// Render one generated Rust types file.
fn render_types(module: &HostAbiModule) -> String {
    let mut output = String::new();

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("pub(crate) use super::callbacks::*;\n\n");

    for named_type in &module.types {
        render_named_type(&mut output, module, named_type);
        output.push('\n');
    }

    output
}

/// Render one named Rust ABI type.
fn render_named_type(output: &mut String, module: &HostAbiModule, named_type: &HostAbiNamedType) {
    push_rust_doc_comment(output, named_type.documentation);

    match &named_type.definition {
        HostAbiNamedTypeDefinition::Struct { fields } => {
            output.push_str("#[derive(Clone, Copy, Debug, Default, PartialEq");
            if named_type_supports_eq(module, named_type) {
                output.push_str(", Eq");
            }
            output.push_str(")]\n");
            output.push_str("#[repr(C)]\n");
            output.push_str(&format!("pub(crate) struct {} {{\n", named_type.name));

            for field in fields {
                push_indented_rust_doc_comment(output, "    ", field.documentation);
                output.push_str(&format!(
                    "    pub {}: {},\n",
                    field.name,
                    render_rust_type(&field.ty)
                ));
            }

            output.push_str("}\n");
        }
        HostAbiNamedTypeDefinition::Enum { repr, variants } => {
            output.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
            output.push_str(&format!("#[repr({})]\n", render_enum_repr(*repr)));
            output.push_str(&format!("pub(crate) enum {} {{\n", named_type.name));

            for variant in variants {
                push_indented_rust_doc_comment(output, "    ", variant.documentation);
                output.push_str(&format!(
                    "    {} = {},\n",
                    variant.name, variant.discriminant
                ));
            }

            output.push_str("}\n");
        }
    }
}

/// Render one generated Rust ingress file.
fn render_ingress(module: &HostAbiModule, platform: HostPlatform) -> String {
    if let Some(ingress) = module.ingress.first() {
        if module_uses_simple_generated_ingress(ingress) {
            return render_simple_ingress(platform, ingress);
        }
    }

    match module.name {
        "background" => render_background_ingress(platform),
        "intent" => render_intent_ingress(platform),
        "notification" => render_notification_ingress(platform),
        "permission" => render_permission_ingress(platform),
        "text" => render_text_ingress(platform),
        _ => String::new(),
    }
}

/// Return whether one module uses generated ingress wrappers.
fn module_uses_generated_ingress(module: &HostAbiModule) -> bool {
    !module.ingress.is_empty() || matches!(module.name, "intent")
}

/// Return whether one ingress fits the straightforward generated Rust wrapper path.
fn module_uses_simple_generated_ingress(ingress: &HostAbiFunction) -> bool {
    if !matches!(ingress.result, HostAbiType::RuntimeStatus) {
        return false;
    }

    ingress
        .parameters
        .iter()
        .all(|parameter| rust_simple_ingress_parameter_supported(&parameter.ty))
}

/// Return whether one ingress parameter fits the straightforward generated wrapper path.
fn rust_simple_ingress_parameter_supported(ty: &HostAbiType) -> bool {
    match ty {
        HostAbiType::U8
        | HostAbiType::U16
        | HostAbiType::I8
        | HostAbiType::I16
        | HostAbiType::U32
        | HostAbiType::I32
        | HostAbiType::U64
        | HostAbiType::HostRequestId
        | HostAbiType::Bool
        | HostAbiType::F64
        | HostAbiType::StringRef
        | HostAbiType::StringSlice
        | HostAbiType::HostSessionHandle => true,
        HostAbiType::NativeSlice(inner) => matches!(inner.as_ref(), HostAbiType::Named(_)),
        HostAbiType::Named(name) => !name.starts_with("Host"),
        HostAbiType::HostStatus
        | HostAbiType::RuntimeStatus
        | HostAbiType::NativeArray(_)
        | HostAbiType::OutputPointer(_) => false,
    }
}

/// Render one straightforward generated Rust ingress wrapper.
fn render_simple_ingress(platform: HostPlatform, ingress: &HostAbiFunction) -> String {
    let ingress_path = platform_ingress_module_path(platform);
    let core_path = platform_ingress_core_path(platform);
    let prefix = rust_platform_prefix(platform);
    let mut output = String::new();
    let mut abi_imports = BTreeSet::new();
    let mut os_imports = BTreeSet::new();
    let mut core_imports = BTreeSet::new();
    let mut needs_invalid_argument_value = false;

    core_imports.insert("runtime_status");

    for parameter in ingress.parameters.iter().skip(1) {
        match &parameter.ty {
            HostAbiType::StringRef => {
                abi_imports.insert("NativeStringRef".to_string());
                core_imports.insert("decode_string");
            }
            HostAbiType::StringSlice => {
                abi_imports.insert("NativeStringSlice".to_string());
                core_imports.insert("decode_string_slice");
            }
            HostAbiType::NativeSlice(inner) => {
                abi_imports.insert("NativeSlice".to_string());
                os_imports.insert(rust_simple_ingress_value_type(inner.as_ref()));
            }
            HostAbiType::Named(name) => {
                needs_invalid_argument_value = true;
                os_imports.insert(rust_simple_ingress_value_type(&HostAbiType::Named(name)));
            }
            _ => {}
        }
    }

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("use crate::diagnostic::RuntimeStatus;\n");

    if needs_invalid_argument_value {
        output.push_str("use crate::host::core::error::invalid_argument_value;\n");
    }

    if !abi_imports.is_empty() {
        output.push_str(&format!(
            "use crate::platform::abi::{{{}}};\n",
            abi_imports.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    if !os_imports.is_empty() {
        output.push_str(&format!(
            "use crate::platform::os::{{{}}};\n",
            os_imports.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    output.push_str(&format!(
        "use {core_path}::{{{}}};\n",
        core_imports.into_iter().collect::<Vec<_>>().join(", ")
    ));
    output.push_str(&format!(
        "use {ingress_path}::{prefix}_{};\n\n",
        ingress.name
    ));

    output.push_str("#[unsafe(no_mangle)]\n");
    output.push_str(&format!(
        "pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_{}(\n",
        ingress.name
    ));
    output.push_str("    runtime_id: u64,\n");

    for (index, parameter) in ingress.parameters.iter().skip(1).enumerate() {
        let trailing = if index + 2 == ingress.parameters.len() {
            ""
        } else {
            ","
        };
        output.push_str(&format!(
            "    {}: {}{trailing}\n",
            parameter.name,
            rust_simple_ingress_parameter_type(&parameter.ty)
        ));
    }

    output.push_str(") -> RuntimeStatus {\n");
    output.push_str("    let result = (|| {\n");

    for parameter in ingress.parameters.iter().skip(1) {
        render_simple_ingress_decode(&mut output, parameter.name, &parameter.ty);
    }

    output.push_str(&format!("        {prefix}_{}(\n", ingress.name));
    output.push_str("            runtime_id");

    for parameter in ingress.parameters.iter().skip(1) {
        output.push_str(",\n            ");
        output.push_str(&rust_simple_ingress_call_argument(
            parameter.name,
            &parameter.ty,
        ));
    }

    output.push_str("\n        )\n");
    output.push_str("    })();\n\n");
    output.push_str("    runtime_status(result)\n");
    output.push_str("}\n");

    output
}

/// Render the decode block for one straightforward ingress parameter.
fn render_simple_ingress_decode(output: &mut String, name: &str, ty: &HostAbiType) {
    match ty {
        HostAbiType::StringRef => {
            output.push_str(&format!(
                "        let {name} = decode_string({name}, \"{name}\")?;\n"
            ));
        }
        HostAbiType::StringSlice => {
            output.push_str(&format!(
                "        let {name} = decode_string_slice({name}, \"{name}\")?;\n"
            ));
        }
        HostAbiType::NativeSlice(_) => {
            output.push_str(&format!(
                "        let {name} = unsafe {{ {name}.as_slice() }}?.to_vec();\n"
            ));
        }
        HostAbiType::Named(_) => {
            output.push_str(&format!(
                "        if {name}.is_null() {{\n            return Err(invalid_argument_value(\n                \"{name}\",\n                \"{name} pointer must not be null\",\n            ));\n        }}\n\n        let {name} = unsafe {{ *{name} }};\n"
            ));
        }
        _ => {}
    }
}

/// Return the extern parameter type for one straightforward ingress parameter.
fn rust_simple_ingress_parameter_type(ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 => "u8".to_string(),
        HostAbiType::U16 => "u16".to_string(),
        HostAbiType::I8 => "i8".to_string(),
        HostAbiType::I16 => "i16".to_string(),
        HostAbiType::U32 => "u32".to_string(),
        HostAbiType::I32 => "i32".to_string(),
        HostAbiType::U64 | HostAbiType::HostRequestId | HostAbiType::HostSessionHandle => {
            "u64".to_string()
        }
        HostAbiType::Bool => "bool".to_string(),
        HostAbiType::F64 => "f64".to_string(),
        HostAbiType::StringRef => "NativeStringRef".to_string(),
        HostAbiType::StringSlice => "NativeStringSlice".to_string(),
        HostAbiType::NativeSlice(inner) => {
            format!(
                "NativeSlice<{}>",
                rust_simple_ingress_value_type(inner.as_ref())
            )
        }
        HostAbiType::Named(_) => {
            format!("*const {}", rust_simple_ingress_value_type(ty))
        }
        other => panic!("unsupported straightforward ingress parameter type: {other:?}"),
    }
}

/// Return the call argument for one straightforward ingress parameter.
fn rust_simple_ingress_call_argument(name: &str, ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::StringRef | HostAbiType::StringSlice => format!("&{name}"),
        _ => name.to_string(),
    }
}

/// Return the runtime value type for one straightforward ingress ABI type.
fn rust_simple_ingress_value_type(ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::Named(name) => {
            let name = name.strip_prefix("Host").unwrap_or(name);
            format!("{name}Value")
        }
        other => panic!("unsupported straightforward ingress value type: {other:?}"),
    }
}

/// Return the platform ingress module path.
fn platform_ingress_module_path(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Android => "crate::host::android::ingress",
        HostPlatform::Ios => "crate::host::ios::ingress",
    }
}

/// Return the ingress core module path.
fn platform_ingress_core_path(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Android => "crate::host::android::abi::ingress::core",
        HostPlatform::Ios => "crate::host::ios::abi::ingress::core",
    }
}

/// Return the Rust runtime ingress export prefix.
fn rust_platform_prefix(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Android => "android",
        HostPlatform::Ios => "ios",
    }
}

/// Return one generated background ingress wrapper.
fn render_background_ingress(platform: HostPlatform) -> String {
    let ingress_path = platform_ingress_module_path(platform);
    let core_path = platform_ingress_core_path(platform);
    let prefix = rust_platform_prefix(platform);

    format!(
        "// generated by generate-bindings: do not edit\n\n\
use crate::diagnostic::RuntimeStatus;\n\
use crate::host::abi::background::HostBackgroundEvent;\n\
use crate::host::abi::background::decode_event;\n\
use crate::host::core::error::invalid_argument_value;\n\
use {core_path}::runtime_status;\n\
use {ingress_path}::{prefix}_notify_background_event;\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_background_event(\n\
    runtime_id: u64,\n\
    event: *const HostBackgroundEvent,\n\
) -> RuntimeStatus {{\n\
    let result = if event.is_null() {{\n\
        Err(invalid_argument_value(\n\
            \"event\",\n\
            \"event pointer must not be null\",\n\
        ))\n\
    }} else {{\n\
        let event = decode_event(unsafe {{ *event }});\n\
        event.and_then(|event| {prefix}_notify_background_event(runtime_id, event))\n\
    }};\n\n\
    runtime_status(result)\n\
}}\n"
    )
}

/// Return one generated intent ingress wrapper.
fn render_intent_ingress(platform: HostPlatform) -> String {
    let ingress_path = platform_ingress_module_path(platform);
    let core_path = platform_ingress_core_path(platform);
    let prefix = rust_platform_prefix(platform);

    format!(
        "// generated by generate-bindings: do not edit\n\n\
use crate::diagnostic::RuntimeStatus;\n\
use crate::platform::abi::{{NativeStringRef, NativeStringSlice}};\n\
use {core_path}::{{decode_optional_string, decode_string, decode_string_slice, runtime_status}};\n\
use {ingress_path}::{{\n\
    {prefix}_notify_intent_custom_action, {prefix}_notify_intent_open_file,\n\
    {prefix}_notify_intent_open_url, {prefix}_notify_intent_share_files,\n\
    {prefix}_notify_intent_share_text,\n\
}};\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_intent_open_url(\n\
    runtime_id: u64,\n\
    has_source: bool,\n\
    source: NativeStringRef,\n\
    url: NativeStringRef,\n\
) -> RuntimeStatus {{\n\
    let result = decode_optional_string(has_source, source, \"source\").and_then(|source| {{\n\
        decode_string(url, \"url\")\n\
            .and_then(|url| {prefix}_notify_intent_open_url(runtime_id, source.as_deref(), &url))\n\
    }});\n\n\
    runtime_status(result)\n\
}}\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_intent_open_file(\n\
    runtime_id: u64,\n\
    has_source: bool,\n\
    source: NativeStringRef,\n\
    path: NativeStringRef,\n\
    has_mime_type: bool,\n\
    mime_type: NativeStringRef,\n\
) -> RuntimeStatus {{\n\
    let result = decode_optional_string(has_source, source, \"source\").and_then(|source| {{\n\
        decode_string(path, \"path\").and_then(|path| {{\n\
            decode_optional_string(has_mime_type, mime_type, \"mime_type\").and_then(|mime_type| {{\n\
                {prefix}_notify_intent_open_file(\n\
                    runtime_id,\n\
                    source.as_deref(),\n\
                    &path,\n\
                    mime_type.as_deref(),\n\
                )\n\
            }})\n\
        }})\n\
    }});\n\n\
    runtime_status(result)\n\
}}\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_intent_share_text(\n\
    runtime_id: u64,\n\
    has_source: bool,\n\
    source: NativeStringRef,\n\
    text: NativeStringRef,\n\
    has_mime_type: bool,\n\
    mime_type: NativeStringRef,\n\
) -> RuntimeStatus {{\n\
    let result = decode_optional_string(has_source, source, \"source\").and_then(|source| {{\n\
        decode_string(text, \"text\").and_then(|text| {{\n\
            decode_optional_string(has_mime_type, mime_type, \"mime_type\").and_then(|mime_type| {{\n\
                {prefix}_notify_intent_share_text(\n\
                    runtime_id,\n\
                    source.as_deref(),\n\
                    &text,\n\
                    mime_type.as_deref(),\n\
                )\n\
            }})\n\
        }})\n\
    }});\n\n\
    runtime_status(result)\n\
}}\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_intent_share_files(\n\
    runtime_id: u64,\n\
    has_source: bool,\n\
    source: NativeStringRef,\n\
    paths: NativeStringSlice,\n\
    has_mime_type: bool,\n\
    mime_type: NativeStringRef,\n\
) -> RuntimeStatus {{\n\
    let result = decode_optional_string(has_source, source, \"source\").and_then(|source| {{\n\
        decode_string_slice(paths, \"paths\").and_then(|paths| {{\n\
            decode_optional_string(has_mime_type, mime_type, \"mime_type\").and_then(|mime_type| {{\n\
                {prefix}_notify_intent_share_files(\n\
                    runtime_id,\n\
                    source.as_deref(),\n\
                    &paths,\n\
                    mime_type.as_deref(),\n\
                )\n\
            }})\n\
        }})\n\
    }});\n\n\
    runtime_status(result)\n\
}}\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_intent_custom_action(\n\
    runtime_id: u64,\n\
    has_source: bool,\n\
    source: NativeStringRef,\n\
    action: NativeStringRef,\n\
    has_url: bool,\n\
    url: NativeStringRef,\n\
    paths: NativeStringSlice,\n\
    has_text: bool,\n\
    text: NativeStringRef,\n\
    has_mime_type: bool,\n\
    mime_type: NativeStringRef,\n\
) -> RuntimeStatus {{\n\
    let result = decode_optional_string(has_source, source, \"source\").and_then(|source| {{\n\
        decode_string(action, \"action\").and_then(|action| {{\n\
            decode_optional_string(has_url, url, \"url\").and_then(|url| {{\n\
                decode_string_slice(paths, \"paths\").and_then(|paths| {{\n\
                    decode_optional_string(has_text, text, \"text\").and_then(|text| {{\n\
                        decode_optional_string(has_mime_type, mime_type, \"mime_type\").and_then(\n\
                            |mime_type| {{\n\
                                {prefix}_notify_intent_custom_action(\n\
                                    runtime_id,\n\
                                    source.as_deref(),\n\
                                    &action,\n\
                                    url.as_deref(),\n\
                                    &paths,\n\
                                    text.as_deref(),\n\
                                    mime_type.as_deref(),\n\
                                )\n\
                            }},\n\
                        )\n\
                    }})\n\
                }})\n\
            }})\n\
        }})\n\
    }});\n\n\
    runtime_status(result)\n\
}}\n"
    )
}

/// Return one generated notification ingress wrapper.
fn render_notification_ingress(platform: HostPlatform) -> String {
    let ingress_path = platform_ingress_module_path(platform);
    let core_path = platform_ingress_core_path(platform);
    let prefix = rust_platform_prefix(platform);

    format!(
        "// generated by generate-bindings: do not edit\n\n\
use crate::diagnostic::RuntimeStatus;\n\
use crate::host::abi::notification::HostNotificationEvent;\n\
use crate::host::abi::notification::decode_notification_event;\n\
use {core_path}::runtime_status;\n\
use {ingress_path}::{prefix}_notify_notification_event;\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_notification_event(\n\
    runtime_id: u64,\n\
    event: HostNotificationEvent,\n\
) -> RuntimeStatus {{\n\
    let result = decode_notification_event(event)\n\
        .and_then(|event| {prefix}_notify_notification_event(runtime_id, event));\n\n\
    runtime_status(result)\n\
}}\n"
    )
}

/// Return one generated permission ingress wrapper.
fn render_permission_ingress(platform: HostPlatform) -> String {
    let ingress_path = platform_ingress_module_path(platform);
    let core_path = platform_ingress_core_path(platform);
    let prefix = rust_platform_prefix(platform);

    format!(
        "// generated by generate-bindings: do not edit\n\n\
use crate::diagnostic::RuntimeStatus;\n\
use crate::host::abi::permission::HostPermissionEvent;\n\
use {core_path}::{{decode_string, runtime_status}};\n\
use {ingress_path}::{prefix}_notify_permission_result;\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_permission_result(\n\
    runtime_id: u64,\n\
    event: HostPermissionEvent,\n\
) -> RuntimeStatus {{\n\
    let result = decode_string(event.permission, \"permission\").and_then(|permission| {{\n\
        {prefix}_notify_permission_result(\n\
            runtime_id,\n\
            event.request_id,\n\
            permission.as_str(),\n\
            event.is_granted,\n\
        )\n\
    }});\n\n\
    runtime_status(result)\n\
}}\n"
    )
}

/// Return one generated text ingress wrapper.
fn render_text_ingress(platform: HostPlatform) -> String {
    let core_path = platform_ingress_core_path(platform);
    let prefix = rust_platform_prefix(platform);

    format!(
        "// generated by generate-bindings: do not edit\n\n\
use crate::diagnostic::RuntimeStatus;\n\
use crate::host::abi::text::HostTextInputState;\n\
use crate::host::abi::text::decode_text_input_state;\n\
use crate::host::core::error::invalid_argument_value;\n\
use crate::platform::input::host::text::notify_text_input_state;\n\
use {core_path}::runtime_status;\n\n\
#[unsafe(no_mangle)]\n\
pub(crate) unsafe extern \"C\" fn destack_host_{prefix}_notify_text_input_state(\n\
    runtime_id: u64,\n\
    session_id: u64,\n\
    state: *const HostTextInputState,\n\
) -> RuntimeStatus {{\n\
    let result = if state.is_null() {{\n\
        Err(invalid_argument_value(\n\
            \"state\",\n\
            \"state pointer must not be null\",\n\
        ))\n\
    }} else {{\n\
        let state = unsafe {{ decode_text_input_state(*state) }};\n\
        notify_text_input_state(runtime_id, session_id, state)\n\
    }};\n\n\
    runtime_status(result)\n\
}}\n"
    )
}

/// Render one generated Rust FFI file.
fn render_ffi(module: &HostAbiModule, platform: HostPlatform) -> String {
    let mut output = String::new();
    let platform_label = platform.label();
    let platform_prefix = platform.prefix();
    let callback_helper_name = format!("call_{}_{}_callback", platform_prefix, module.name);
    let callback_struct_name = callback_struct_name(module, platform);

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str(&format!(
        "use super::callbacks::{{{callback_helper_name}, {callback_struct_name}}};\n"
    ));

    let mut abi_types = referenced_named_types(module);
    abi_types.sort();
    abi_types.dedup();

    if !abi_types.is_empty() {
        output.push_str(&format!(
            "use crate::host::abi::{}::{{{}}};\n",
            module.name,
            abi_types.join(", ")
        ));
    }
    if module_uses_output_pointer(module) {
        output.push_str("use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;\n");
    }
    if module_uses_native_array(module) {
        output.push_str("use crate::platform::NativeArray;\n");
    }
    if module_uses_native_slice(module) || module_uses_string_slice(module) {
        let mut native_types = Vec::new();
        if module_uses_native_slice(module) {
            native_types.push("NativeSlice");
        }
        if module_uses_string_slice(module) {
            native_types.push("NativeStringSlice");
        }
        output.push_str(&format!(
            "use crate::platform::abi::{{{}}};\n",
            native_types.join(", ")
        ));
    }
    if module_uses_string_ref(module) {
        output.push_str("use crate::platform::abi::NativeStringRef;\n");
    }
    if !module.rust_capability_probes.is_empty() {
        output.push_str(&format!(
            "use crate::host::{}::abi::bindings::resolve_{}_binding_callback;\n",
            platform.segment(),
            platform.prefix()
        ));
        output.push_str("use crate::host::{HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK};\n");
    }
    output.push('\n');

    for request in &module.requests {
        let field_name = rust_request_field_name(module, request);
        let callback_handle_name = request
            .parameters
            .first()
            .map(|parameter| parameter.name)
            .unwrap_or("session_handle");

        push_rust_doc_comment(
            &mut output,
            &format!(
                "Forward the `{field_name}` {} request through the {platform_label} host ABI.",
                module.name
            ),
        );
        output.push_str("#[unsafe(no_mangle)]\n");
        output.push_str("pub(crate) unsafe extern \"C\" fn ");
        output.push_str(&format!(
            "destack_host_{}_{}_{}(\n",
            platform.segment(),
            module.name,
            rust_request_export_stem(module, request)
        ));

        for parameter in ffi_parameters(request) {
            output.push_str(&format!(
                "    {}: {},\n",
                parameter.name,
                render_rust_type(&parameter.ty)
            ));
        }
        output.push_str(") -> u32 {\n");

        for parameter in request
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty, HostAbiType::OutputPointer(_)))
        {
            output.push_str(&format!("    if {}.is_null() {{\n", parameter.name));
            output.push_str("        return HOST_STATUS_INVALID_ARGUMENT;\n");
            output.push_str("    }\n\n");
        }

        output.push_str(&format!("    {callback_helper_name}(\n"));
        output.push_str(&format!("        {callback_handle_name},\n"));
        output.push_str(&format!("        |callbacks| callbacks.{},\n", field_name));
        output.push_str("        |callback| unsafe { callback(");
        output.push_str(callback_handle_name);
        for parameter in request.parameters.iter().skip(1) {
            output.push_str(", ");
            output.push_str(parameter.name);
        }
        output.push_str(") },\n");
        output.push_str("    )\n");
        output.push_str("}\n\n");
    }

    for control in &module.selector_controls {
        render_selector_control(&mut output, module, platform, control);
    }

    for probe in &module.rust_capability_probes {
        render_capability_probe(&mut output, module, platform, probe, &callback_struct_name);
    }

    output
}

/// Render one selector-backed control wrapper family.
fn render_selector_control(
    output: &mut String,
    module: &HostAbiModule,
    platform: HostPlatform,
    control: &HostAbiSelectorControl,
) {
    render_selector_control_getter(output, module, platform, control);

    if control.setter_export_stem.is_some() {
        render_selector_control_setter(output, module, platform, control);
    }

    if control.range_export_stem.is_some() {
        render_selector_control_range(output, module, platform, control);
    }
}

/// Render one selector-backed control getter.
fn render_selector_control_getter(
    output: &mut String,
    module: &HostAbiModule,
    platform: HostPlatform,
    control: &HostAbiSelectorControl,
) {
    let parameter_type = render_rust_type(&control.value_ty);
    let target_export = selector_target_request_stem("get", &control.value_ty);

    push_rust_doc_comment(
        output,
        &format!(
            "Forward the `{}` {} control getter through the {} host ABI.",
            control.getter_export_stem,
            module.name,
            platform.label()
        ),
    );
    output.push_str("#[unsafe(no_mangle)]\n");
    output.push_str(&format!(
        "pub(crate) unsafe extern \"C\" fn destack_host_{}_{}_{}(\n",
        platform.segment(),
        module.name,
        control.getter_export_stem
    ));
    output.push_str("    session_handle: u64,\n");
    output.push_str("    stream_id: u64,\n");
    output.push_str(&format!("    value: *mut {parameter_type},\n"));
    output.push_str(") -> u32 {\n");
    output.push_str(&format!(
        "    unsafe {{ destack_host_{}_{}_{}(session_handle, stream_id, {}, value) }}\n",
        platform.segment(),
        module.name,
        target_export,
        control.selector
    ));
    output.push_str("}\n\n");
}

/// Render one selector-backed control setter.
fn render_selector_control_setter(
    output: &mut String,
    module: &HostAbiModule,
    platform: HostPlatform,
    control: &HostAbiSelectorControl,
) {
    let setter_export_stem = control
        .setter_export_stem
        .expect("selector setter export stem must be present");
    let parameter_type = render_rust_type(&control.value_ty);
    let target_export = selector_target_request_stem("set", &control.value_ty);

    push_rust_doc_comment(
        output,
        &format!(
            "Forward the `{setter_export_stem}` {} control setter through the {} host ABI.",
            module.name,
            platform.label()
        ),
    );
    output.push_str("#[unsafe(no_mangle)]\n");
    output.push_str(&format!(
        "pub(crate) unsafe extern \"C\" fn destack_host_{}_{}_{}(\n",
        platform.segment(),
        module.name,
        setter_export_stem
    ));
    output.push_str("    session_handle: u64,\n");
    output.push_str("    stream_id: u64,\n");
    output.push_str(&format!("    value: {parameter_type},\n"));
    output.push_str(") -> u32 {\n");
    output.push_str(&format!(
        "    unsafe {{ destack_host_{}_{}_{}(session_handle, stream_id, {}, value) }}\n",
        platform.segment(),
        module.name,
        target_export,
        control.selector
    ));
    output.push_str("}\n\n");
}

/// Render one selector-backed control range getter.
fn render_selector_control_range(
    output: &mut String,
    module: &HostAbiModule,
    platform: HostPlatform,
    control: &HostAbiSelectorControl,
) {
    let range_export_stem = control
        .range_export_stem
        .expect("selector range export stem must be present");
    let parameter_type = render_rust_type(&control.value_ty);
    let target_export = selector_target_request_stem("range", &control.value_ty);

    push_rust_doc_comment(
        output,
        &format!(
            "Forward the `{range_export_stem}` {} control range query through the {} host ABI.",
            module.name,
            platform.label()
        ),
    );
    output.push_str("#[unsafe(no_mangle)]\n");
    output.push_str(&format!(
        "pub(crate) unsafe extern \"C\" fn destack_host_{}_{}_{}(\n",
        platform.segment(),
        module.name,
        range_export_stem
    ));
    output.push_str("    session_handle: u64,\n");
    output.push_str("    stream_id: u64,\n");
    output.push_str(&format!("    minimum: *mut {parameter_type},\n"));
    output.push_str(&format!("    maximum: *mut {parameter_type},\n"));
    output.push_str(&format!("    step: *mut {parameter_type},\n"));
    output.push_str(") -> u32 {\n");
    output.push_str(&format!(
        "    unsafe {{ destack_host_{}_{}_{}(session_handle, stream_id, {}, minimum, maximum, step) }}\n",
        platform.segment(),
        module.name,
        target_export,
        control.selector
    ));
    output.push_str("}\n\n");
}

/// Render one generated capability probe.
fn render_capability_probe(
    output: &mut String,
    module: &HostAbiModule,
    platform: HostPlatform,
    probe: &HostAbiRustCapabilityProbe,
    callback_struct_name: &str,
) {
    push_rust_doc_comment(output, probe.documentation);
    output.push_str("#[unsafe(no_mangle)]\n");
    output.push_str(&format!(
        "pub(crate) unsafe extern \"C\" fn destack_host_{}_{}_{}(\n",
        platform.segment(),
        module.name,
        probe.export_name
    ));
    output.push_str("    session_handle: u64,\n");

    for parameter in &probe.parameters {
        output.push_str(&format!(
            "    {}: {},\n",
            parameter.name,
            render_rust_type(&parameter.ty)
        ));
    }

    output.push_str(") -> u32 {\n");
    output.push_str(&format!(
        "    let callbacks = match resolve_{}_binding_callback(session_handle, |bindings| Some(bindings.{})) {{\n",
        platform.prefix(),
        module.name
    ));
    output.push_str("        Ok(callbacks) => callbacks,\n");
    output.push_str("        Err(status) => return status,\n");
    output.push_str("    };\n\n");
    output.push_str(&format!(
        "    let callbacks: {callback_struct_name} = callbacks;\n\n"
    ));
    output.push_str("    let is_supported = ");
    output.push_str(&render_capability_availability(&probe.availability));
    output.push_str(";\n");
    output.push_str("    if !is_supported {\n");
    output.push_str(&format!(
        "        return {};\n",
        render_status_constant(probe.unsupported_status)
    ));
    output.push_str("    }\n\n");
    output.push_str(&render_capability_action(&probe.on_supported));
    output.push_str("}\n\n");
}

/// Return the FFI parameters for one request.
fn ffi_parameters(
    request: &HostAbiFunction,
) -> &[destack_runtime::host::abi::describe::HostAbiParameter] {
    &request.parameters
}
/// Return the callback struct name for one module and platform.
fn callback_struct_name(module: &HostAbiModule, platform: HostPlatform) -> String {
    format!(
        "{}Host{}Callbacks",
        platform.rust_name(),
        pascal_case(module.name)
    )
}

/// Return the callback type name for one request.
fn callback_type_name(
    module: &HostAbiModule,
    request: &HostAbiFunction,
    platform: HostPlatform,
) -> String {
    format!(
        "{}Host{}{}Callback",
        platform.rust_name(),
        pascal_case(module.name),
        pascal_case(rust_request_export_stem(module, request))
    )
}

/// Return the Rust request field name for one module request.
fn rust_request_field_name(module: &HostAbiModule, request: &HostAbiFunction) -> String {
    let _ = module;

    request.name.to_string()
}

/// Return the exported request stem for one module request.
fn rust_request_export_stem(module: &HostAbiModule, request: &HostAbiFunction) -> String {
    let _ = module;

    request.name.to_string()
}

/// Render the callback parameter list for one request.
fn render_callback_parameter_list(request: &HostAbiFunction) -> String {
    request
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, render_rust_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render one Rust ABI type.
fn render_rust_type(ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 => "u8".to_string(),
        HostAbiType::U16 => "u16".to_string(),
        HostAbiType::I8 => "i8".to_string(),
        HostAbiType::I16 => "i16".to_string(),
        HostAbiType::U32 => "u32".to_string(),
        HostAbiType::I32 => "i32".to_string(),
        HostAbiType::U64 => "u64".to_string(),
        HostAbiType::HostRequestId => "u64".to_string(),
        HostAbiType::Bool => "bool".to_string(),
        HostAbiType::F64 => "f64".to_string(),
        HostAbiType::StringRef => "NativeStringRef".to_string(),
        HostAbiType::StringSlice => "NativeStringSlice".to_string(),
        HostAbiType::HostSessionHandle => "u64".to_string(),
        HostAbiType::HostStatus => "u32".to_string(),
        HostAbiType::RuntimeStatus => "RuntimeStatus".to_string(),
        HostAbiType::NativeArray(inner) => format!("NativeArray<{}>", render_rust_type(inner)),
        HostAbiType::NativeSlice(inner) => format!("NativeSlice<{}>", render_rust_type(inner)),
        HostAbiType::OutputPointer(inner) => format!("*mut {}", render_rust_type(inner)),
        HostAbiType::Named(name) => (*name).to_string(),
    }
}

/// Render one Rust enum representation.
fn render_enum_repr(repr: HostAbiEnumRepresentation) -> &'static str {
    match repr {
        HostAbiEnumRepresentation::I32 => "i32",
        HostAbiEnumRepresentation::U32 => "u32",
    }
}

/// Return whether one module should emit one runtime types file for one platform.
fn module_uses_generated_types(module: &HostAbiModule, platform: HostPlatform) -> bool {
    !module.types.is_empty()
        && module.platforms.len() == 1
        && HostPlatform::from_abi(module.platforms[0]) == platform
}

/// Return whether one named type supports `Eq`.
fn named_type_supports_eq(module: &HostAbiModule, named_type: &HostAbiNamedType) -> bool {
    match &named_type.definition {
        HostAbiNamedTypeDefinition::Struct { fields } => fields
            .iter()
            .all(|field| type_supports_eq(module, &field.ty)),
        HostAbiNamedTypeDefinition::Enum { .. } => true,
    }
}

/// Return whether one type supports `Eq`.
fn type_supports_eq(module: &HostAbiModule, ty: &HostAbiType) -> bool {
    match ty {
        HostAbiType::U8
        | HostAbiType::U16
        | HostAbiType::I8
        | HostAbiType::I16
        | HostAbiType::U32
        | HostAbiType::I32
        | HostAbiType::U64
        | HostAbiType::HostRequestId
        | HostAbiType::Bool
        | HostAbiType::StringRef
        | HostAbiType::StringSlice
        | HostAbiType::HostSessionHandle
        | HostAbiType::HostStatus
        | HostAbiType::RuntimeStatus => true,
        HostAbiType::F64 => false,
        HostAbiType::NativeArray(inner)
        | HostAbiType::NativeSlice(inner)
        | HostAbiType::OutputPointer(inner) => type_supports_eq(module, inner),
        HostAbiType::Named(name) => module
            .types
            .iter()
            .find(|named_type| named_type.name == *name)
            .is_some_and(|named_type| named_type_supports_eq(module, named_type)),
    }
}

/// Return the low-level selector request stem for one operation and scalar type.
fn selector_target_request_stem(operation: &str, value_ty: &HostAbiType) -> &'static str {
    match (operation, value_ty) {
        ("get", HostAbiType::U64) => "stream_get_u64",
        ("set", HostAbiType::U64) => "stream_set_u64",
        ("range", HostAbiType::U64) => "stream_get_range_u64",
        ("get", HostAbiType::U32) => "stream_get_u32",
        ("set", HostAbiType::U32) => "stream_set_u32",
        ("range", HostAbiType::U32) => "stream_get_range_u32",
        ("get", HostAbiType::F64) => "stream_get_f64",
        ("set", HostAbiType::F64) => "stream_set_f64",
        ("range", HostAbiType::F64) => "stream_get_range_f64",
        _ => panic!("unsupported selector control scalar type"),
    }
}

/// Render one capability availability expression.
fn render_capability_availability(availability: &HostAbiRustAvailability) -> String {
    match availability {
        HostAbiRustAvailability::CallbackPresent(field) => format!("callbacks.{field}.is_some()"),
        HostAbiRustAvailability::ParameterEquals { parameter, value } => {
            format!("{parameter} == {value}")
        }
        HostAbiRustAvailability::All(conditions) => conditions
            .iter()
            .map(render_capability_availability)
            .map(|condition| format!("({condition})"))
            .collect::<Vec<_>>()
            .join(" && "),
        HostAbiRustAvailability::Any(conditions) => conditions
            .iter()
            .map(render_capability_availability)
            .map(|condition| format!("({condition})"))
            .collect::<Vec<_>>()
            .join(" || "),
    }
}

/// Render one capability action body.
fn render_capability_action(action: &HostAbiRustCapabilityAction) -> String {
    match action {
        HostAbiRustCapabilityAction::ReturnStatus(status) => {
            format!("    {}\n", render_status_constant(*status))
        }
        HostAbiRustCapabilityAction::InvokeCallback {
            callback,
            arguments,
        } => {
            let mut output = String::new();

            output.push_str(&format!(
                "    let Some(callback) = callbacks.{callback} else {{\n"
            ));
            output.push_str("        return HOST_STATUS_OK;\n");
            output.push_str("    };\n\n");
            output.push_str("    unsafe { callback(");
            output.push_str(&arguments.join(", "));
            output.push_str(") }\n");

            output
        }
    }
}

/// Render one host status constant.
fn render_status_constant(status: HostAbiRustStatus) -> &'static str {
    match status {
        HostAbiRustStatus::Ok => "HOST_STATUS_OK",
        HostAbiRustStatus::NotSupported => "HOST_STATUS_NOT_SUPPORTED",
    }
}

/// Return whether one module references `NativeArray`.
fn module_uses_native_array(module: &HostAbiModule) -> bool {
    module_requests_use(module, &|ty| matches!(ty, HostAbiType::NativeArray(_)))
}

/// Return whether one module references `NativeStringRef`.
fn module_uses_string_ref(module: &HostAbiModule) -> bool {
    module_requests_use(module, &|ty| matches!(ty, HostAbiType::StringRef))
}

/// Return whether one module references `NativeSlice`.
fn module_uses_native_slice(module: &HostAbiModule) -> bool {
    module_requests_use(module, &|ty| matches!(ty, HostAbiType::NativeSlice(_)))
}

/// Return whether one module references `NativeStringSlice`.
fn module_uses_string_slice(module: &HostAbiModule) -> bool {
    module_requests_use(module, &|ty| matches!(ty, HostAbiType::StringSlice))
}

/// Return whether one module references output pointers.
fn module_uses_output_pointer(module: &HostAbiModule) -> bool {
    module_requests_use(module, &|ty| matches!(ty, HostAbiType::OutputPointer(_)))
}

/// Return whether any request type in one module matches one predicate.
fn module_requests_use(module: &HostAbiModule, predicate: &impl Fn(&HostAbiType) -> bool) -> bool {
    module
        .requests
        .iter()
        .flat_map(|request| request.parameters.iter())
        .any(|parameter| type_uses(&parameter.ty, predicate))
}

/// Return the named ABI types referenced in one module request surface.
fn referenced_named_types(module: &HostAbiModule) -> Vec<&'static str> {
    let mut names = Vec::new();

    for request in &module.requests {
        for parameter in &request.parameters {
            collect_named_types(&parameter.ty, &mut names);
        }
    }

    names
}

/// Collect the named ABI types reachable from one type.
fn collect_named_types(ty: &HostAbiType, names: &mut Vec<&'static str>) {
    match ty {
        HostAbiType::Named(name) => names.push(name),
        HostAbiType::NativeArray(inner)
        | HostAbiType::NativeSlice(inner)
        | HostAbiType::OutputPointer(inner) => collect_named_types(inner, names),
        _ => {}
    }
}

/// Return whether one type or nested type matches one predicate.
fn type_uses(ty: &HostAbiType, predicate: &impl Fn(&HostAbiType) -> bool) -> bool {
    if predicate(ty) {
        return true;
    }

    match ty {
        HostAbiType::NativeArray(inner)
        | HostAbiType::NativeSlice(inner)
        | HostAbiType::OutputPointer(inner) => type_uses(inner, predicate),
        _ => false,
    }
}
