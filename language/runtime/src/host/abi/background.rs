use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    BackgroundConflictPolicyValue, BackgroundEventMetadataValue, BackgroundEventValue,
    BackgroundNetworkRequirementValue, BackgroundStatusValue, BackgroundTaskDescriptorValue,
    BackgroundTaskExpiredEventValue, BackgroundTaskOptionsValue, BackgroundTaskReadyEventValue,
    BackgroundTaskResultValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
    BackgroundTriggerKindValue,
};
use crate::platform::{NativeAbiCodec, NativeArray};
use crate::runtime::{BindingCallContext, NativeStringRef};

/// One host background scheduler status payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundStatus {
    /// Background scheduling is unavailable.
    Unavailable = 1,
    /// Background scheduling is restricted.
    Restricted = 2,
    /// Background scheduling is available.
    Available = 3,
}

/// One host background trigger kind payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundTriggerKind {
    /// One app-refresh style task.
    AppRefresh = 1,
    /// One processing style task.
    Processing = 2,
}

/// One host background task-result payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundTaskResult {
    /// The task completed successfully.
    Success = 1,
    /// The task should be retried later.
    Retry = 2,
    /// The task failed and should not be retried automatically.
    Failure = 3,
}

/// One host background network-requirement payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundNetworkRequirement {
    /// No network route is required.
    None = 1,
    /// One network route is required.
    Connected = 2,
    /// One unmetered network route is required.
    Unmetered = 3,
}

/// One host background conflict-policy payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundConflictPolicy {
    /// Replace one existing registration.
    Replace = 1,
    /// Keep one existing registration.
    Keep = 2,
}

/// One host background schedule-kind payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundTaskScheduleKind {
    /// One single future execution.
    Once = 1,
    /// One recurring execution stream.
    Recurring = 2,
}

/// One host background task-schedule payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostBackgroundTaskSchedule {
    /// The declared schedule class.
    pub kind: HostBackgroundTaskScheduleKind,
    /// Whether the earliest execution target is present.
    pub has_earliest_begin_unix_ns: bool,
    /// The earliest execution target in UTC nanoseconds.
    pub earliest_begin_unix_ns: u64,
    /// Whether the repeat interval is present.
    pub has_repeat_interval_ns: bool,
    /// The repeat interval in nanoseconds for recurring schedules.
    pub repeat_interval_ns: u64,
}

/// One host background task-options payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostBackgroundTaskOptions {
    /// The stable task identifier.
    pub identifier: NativeStringRef,
    /// The trigger class.
    pub trigger: HostBackgroundTriggerKind,
    /// The requested schedule payload.
    pub schedule: HostBackgroundTaskSchedule,
    /// The requested network requirement.
    pub network: HostBackgroundNetworkRequirement,
    /// Whether charging power is required.
    pub requires_charging: bool,
    /// Whether idle mode is required.
    pub requires_idle: bool,
    /// The registration conflict policy.
    pub conflict_policy: HostBackgroundConflictPolicy,
}

/// One host background task-descriptor payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostBackgroundTaskDescriptor {
    /// The stable task identifier.
    pub identifier: NativeStringRef,
    /// The trigger class.
    pub trigger: HostBackgroundTriggerKind,
    /// The effective schedule payload.
    pub schedule: HostBackgroundTaskSchedule,
    /// The effective network requirement.
    pub network: HostBackgroundNetworkRequirement,
    /// Whether charging power is required.
    pub requires_charging: bool,
    /// Whether idle mode is required.
    pub requires_idle: bool,
    /// The registration conflict policy.
    pub conflict_policy: HostBackgroundConflictPolicy,
}

impl NativeAbiCodec for HostBackgroundTaskDescriptor {
    type Value = BackgroundTaskDescriptorValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(BackgroundTaskDescriptorValue {
            identifier: decode_string(self.identifier)?,
            trigger: decode_trigger(self.trigger),
            schedule: decode_schedule(self.schedule),
            network: decode_network_requirement(self.network),
            requires_charging: self.requires_charging,
            requires_idle: self.requires_idle,
            conflict_policy: decode_conflict_policy(self.conflict_policy),
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            identifier: <NativeStringRef as NativeAbiCodec>::from_value(binding, value.identifier),
            trigger: encode_trigger(value.trigger),
            schedule: encode_schedule(value.schedule),
            network: encode_network_requirement(value.network),
            requires_charging: value.requires_charging,
            requires_idle: value.requires_idle,
            conflict_policy: encode_conflict_policy(value.conflict_policy),
        }
    }
}

/// One host background event metadata payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostBackgroundEventMetadata {
    /// The monotonic event timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// The monotonic sequence number for this event stream.
    pub sequence: u64,
    /// The stable task identifier.
    pub identifier: NativeStringRef,
    /// The stable execution identifier for this scheduled execution.
    pub execution_id: NativeStringRef,
    /// The host execution deadline in UTC nanoseconds.
    pub deadline_unix_ns: u64,
}

/// One host background event kind payload.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum HostBackgroundEventKind {
    /// One task-ready event.
    TaskReady = 1,
    /// One task-expired event.
    TaskExpired = 2,
}

/// One host background event payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostBackgroundEvent {
    /// The event kind.
    pub kind: HostBackgroundEventKind,
    /// The shared event metadata.
    pub metadata: HostBackgroundEventMetadata,
}

/// One owned host background task-options payload.
#[derive(Debug)]
pub(crate) struct HostBackgroundTaskOptionsPayload {
    /// The owned identifier storage.
    identifier_storage: String,
    /// The borrowed ABI payload.
    abi: HostBackgroundTaskOptions,
}

impl HostBackgroundTaskOptionsPayload {
    /// Build one owned host background task-options payload.
    pub(crate) fn new(options: &BackgroundTaskOptionsValue) -> Self {
        let identifier_storage = options.identifier.clone();
        let abi = HostBackgroundTaskOptions {
            identifier: NativeStringRef::from(identifier_storage.as_str()),
            trigger: encode_trigger(options.trigger),
            schedule: encode_schedule(options.schedule),
            network: encode_network_requirement(options.network),
            requires_charging: options.requires_charging,
            requires_idle: options.requires_idle,
            conflict_policy: encode_conflict_policy(options.conflict_policy),
        };

        Self {
            identifier_storage,
            abi,
        }
    }

    /// Return the ABI task-options payload.
    pub(crate) fn abi(&self) -> HostBackgroundTaskOptions {
        let _ = &self.identifier_storage;

        self.abi
    }
}

/// Decode one host background status into one runtime value.
pub(crate) fn decode_status(status: HostBackgroundStatus) -> BackgroundStatusValue {
    match status {
        HostBackgroundStatus::Unavailable => BackgroundStatusValue::Unavailable,
        HostBackgroundStatus::Restricted => BackgroundStatusValue::Restricted,
        HostBackgroundStatus::Available => BackgroundStatusValue::Available,
    }
}

/// Decode one host background descriptor array into runtime values.
pub(crate) unsafe fn decode_descriptors(
    descriptors: NativeArray<HostBackgroundTaskDescriptor>,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    unsafe {
        <NativeArray<HostBackgroundTaskDescriptor> as NativeAbiCodec>::into_value(descriptors)
    }
}

/// Decode one host background descriptor into one runtime value.
pub(crate) fn decode_descriptor(
    descriptor: HostBackgroundTaskDescriptor,
) -> RuntimeResult<BackgroundTaskDescriptorValue> {
    unsafe { <HostBackgroundTaskDescriptor as NativeAbiCodec>::into_value(descriptor) }
}

/// Encode one runtime background result into one host payload.
pub(crate) fn encode_result(result: BackgroundTaskResultValue) -> HostBackgroundTaskResult {
    match result {
        BackgroundTaskResultValue::Success => HostBackgroundTaskResult::Success,
        BackgroundTaskResultValue::Retry => HostBackgroundTaskResult::Retry,
        BackgroundTaskResultValue::Failure => HostBackgroundTaskResult::Failure,
    }
}

/// Decode one host background event into one runtime value.
pub(crate) fn decode_event(event: HostBackgroundEvent) -> RuntimeResult<BackgroundEventValue> {
    let metadata = decode_event_metadata(event.metadata)?;

    Ok(match event.kind {
        HostBackgroundEventKind::TaskReady => {
            BackgroundEventValue::BackgroundTaskReadyEvent(BackgroundTaskReadyEventValue {
                kind: "taskReady".to_string(),
                metadata,
            })
        }
        HostBackgroundEventKind::TaskExpired => {
            BackgroundEventValue::BackgroundTaskExpiredEvent(BackgroundTaskExpiredEventValue {
                kind: "taskExpired".to_string(),
                metadata,
            })
        }
    })
}

/// Encode one runtime background trigger into one host payload.
fn encode_trigger(trigger: BackgroundTriggerKindValue) -> HostBackgroundTriggerKind {
    match trigger {
        BackgroundTriggerKindValue::AppRefresh => HostBackgroundTriggerKind::AppRefresh,
        BackgroundTriggerKindValue::Processing => HostBackgroundTriggerKind::Processing,
    }
}

/// Decode one host background trigger into one runtime value.
fn decode_trigger(trigger: HostBackgroundTriggerKind) -> BackgroundTriggerKindValue {
    match trigger {
        HostBackgroundTriggerKind::AppRefresh => BackgroundTriggerKindValue::AppRefresh,
        HostBackgroundTriggerKind::Processing => BackgroundTriggerKindValue::Processing,
    }
}

/// Encode one runtime background schedule into one host payload.
fn encode_schedule(schedule: BackgroundTaskScheduleValue) -> HostBackgroundTaskSchedule {
    HostBackgroundTaskSchedule {
        kind: encode_schedule_kind(schedule.kind),
        has_earliest_begin_unix_ns: schedule.earliest_begin_unix_ns.is_some(),
        earliest_begin_unix_ns: schedule.earliest_begin_unix_ns.unwrap_or(0),
        has_repeat_interval_ns: schedule.repeat_interval_ns.is_some(),
        repeat_interval_ns: schedule.repeat_interval_ns.unwrap_or(0),
    }
}

/// Decode one host background schedule into one runtime value.
fn decode_schedule(schedule: HostBackgroundTaskSchedule) -> BackgroundTaskScheduleValue {
    BackgroundTaskScheduleValue {
        kind: decode_schedule_kind(schedule.kind),
        earliest_begin_unix_ns: schedule
            .has_earliest_begin_unix_ns
            .then_some(schedule.earliest_begin_unix_ns),
        repeat_interval_ns: schedule
            .has_repeat_interval_ns
            .then_some(schedule.repeat_interval_ns),
    }
}

/// Encode one runtime background schedule kind into one host payload.
fn encode_schedule_kind(kind: BackgroundTaskScheduleKindValue) -> HostBackgroundTaskScheduleKind {
    match kind {
        BackgroundTaskScheduleKindValue::Once => HostBackgroundTaskScheduleKind::Once,
        BackgroundTaskScheduleKindValue::Recurring => HostBackgroundTaskScheduleKind::Recurring,
    }
}

/// Decode one host background schedule kind into one runtime value.
fn decode_schedule_kind(kind: HostBackgroundTaskScheduleKind) -> BackgroundTaskScheduleKindValue {
    match kind {
        HostBackgroundTaskScheduleKind::Once => BackgroundTaskScheduleKindValue::Once,
        HostBackgroundTaskScheduleKind::Recurring => BackgroundTaskScheduleKindValue::Recurring,
    }
}

/// Encode one runtime background network requirement into one host payload.
fn encode_network_requirement(
    network: BackgroundNetworkRequirementValue,
) -> HostBackgroundNetworkRequirement {
    match network {
        BackgroundNetworkRequirementValue::None => HostBackgroundNetworkRequirement::None,
        BackgroundNetworkRequirementValue::Connected => HostBackgroundNetworkRequirement::Connected,
        BackgroundNetworkRequirementValue::Unmetered => HostBackgroundNetworkRequirement::Unmetered,
    }
}

/// Decode one host background network requirement into one runtime value.
fn decode_network_requirement(
    network: HostBackgroundNetworkRequirement,
) -> BackgroundNetworkRequirementValue {
    match network {
        HostBackgroundNetworkRequirement::None => BackgroundNetworkRequirementValue::None,
        HostBackgroundNetworkRequirement::Connected => BackgroundNetworkRequirementValue::Connected,
        HostBackgroundNetworkRequirement::Unmetered => BackgroundNetworkRequirementValue::Unmetered,
    }
}

/// Encode one runtime background conflict policy into one host payload.
fn encode_conflict_policy(policy: BackgroundConflictPolicyValue) -> HostBackgroundConflictPolicy {
    match policy {
        BackgroundConflictPolicyValue::Replace => HostBackgroundConflictPolicy::Replace,
        BackgroundConflictPolicyValue::Keep => HostBackgroundConflictPolicy::Keep,
    }
}

/// Decode one host background conflict policy into one runtime value.
fn decode_conflict_policy(policy: HostBackgroundConflictPolicy) -> BackgroundConflictPolicyValue {
    match policy {
        HostBackgroundConflictPolicy::Replace => BackgroundConflictPolicyValue::Replace,
        HostBackgroundConflictPolicy::Keep => BackgroundConflictPolicyValue::Keep,
    }
}

/// Decode one host background event metadata payload.
fn decode_event_metadata(
    metadata: HostBackgroundEventMetadata,
) -> RuntimeResult<BackgroundEventMetadataValue> {
    Ok(BackgroundEventMetadataValue {
        timestamp_ns: metadata.timestamp_ns,
        sequence: metadata.sequence,
        identifier: decode_string(metadata.identifier)?,
        execution_id: decode_string(metadata.execution_id)?,
        deadline_unix_ns: metadata.deadline_unix_ns,
    })
}

/// Decode one host string reference into one owned string.
fn decode_string(value: NativeStringRef) -> RuntimeResult<String> {
    unsafe { <NativeStringRef as NativeAbiCodec>::into_value(value) }
}
