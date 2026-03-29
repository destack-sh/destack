use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host background scheduler status payload.
        enum HostBackgroundStatus: u32 {
            /// Background scheduling is unavailable.
            Unavailable = 1,
            /// Background scheduling is restricted.
            Restricted = 2,
            /// Background scheduling is available.
            Available = 3,
        }

        /// One host background trigger kind payload.
        enum HostBackgroundTriggerKind: u32 {
            /// One app-refresh style task.
            AppRefresh = 1,
            /// One processing style task.
            Processing = 2,
        }

        /// One host background task-result payload.
        enum HostBackgroundTaskResult: u32 {
            /// The task completed successfully.
            Success = 1,
            /// The task should be retried later.
            Retry = 2,
            /// The task failed and should not be retried automatically.
            Failure = 3,
        }

        /// One host background network-requirement payload.
        enum HostBackgroundNetworkRequirement: u32 {
            /// No network route is required.
            None = 1,
            /// One network route is required.
            Connected = 2,
            /// One unmetered network route is required.
            Unmetered = 3,
        }

        /// One host background conflict-policy payload.
        enum HostBackgroundConflictPolicy: u32 {
            /// Replace one existing registration.
            Replace = 1,
            /// Keep one existing registration.
            Keep = 2,
        }

        /// One host background schedule-kind payload.
        enum HostBackgroundTaskScheduleKind: u32 {
            /// One single future execution.
            Once = 1,
            /// One recurring execution stream.
            Recurring = 2,
        }

        /// One host background task-schedule payload.
        struct HostBackgroundTaskSchedule {
            /// The declared schedule class.
            kind: HostBackgroundTaskScheduleKind,
            /// Whether the earliest execution target is present.
            has_earliest_begin_unix_ns: bool,
            /// The earliest execution target in UTC nanoseconds.
            earliest_begin_unix_ns: u64,
            /// Whether the repeat interval is present.
            has_repeat_interval_ns: bool,
            /// The repeat interval in nanoseconds for recurring schedules.
            repeat_interval_ns: u64,
        }

        /// One host background task-options payload.
        struct HostBackgroundTaskOptions {
            /// The stable task identifier.
            identifier: string_ref,
            /// The trigger class.
            trigger: HostBackgroundTriggerKind,
            /// The requested schedule payload.
            schedule: HostBackgroundTaskSchedule,
            /// The requested network requirement.
            network: HostBackgroundNetworkRequirement,
            /// Whether charging power is required.
            requires_charging: bool,
            /// Whether idle mode is required.
            requires_idle: bool,
            /// The registration conflict policy.
            conflict_policy: HostBackgroundConflictPolicy,
        }

        /// One host background status response payload.
        struct HostBackgroundStatusResponse {
            /// The request status code.
            status: host_status,
            /// Whether one scheduler status is present.
            has_scheduler_status: bool,
            /// The scheduler status when present.
            scheduler_status: HostBackgroundStatus,
        }

        /// One host background task-descriptor payload.
        struct HostBackgroundTaskDescriptor {
            /// The stable task identifier.
            identifier: string_ref,
            /// The trigger class.
            trigger: HostBackgroundTriggerKind,
            /// The effective schedule payload.
            schedule: HostBackgroundTaskSchedule,
            /// The effective network requirement.
            network: HostBackgroundNetworkRequirement,
            /// Whether charging power is required.
            requires_charging: bool,
            /// Whether idle mode is required.
            requires_idle: bool,
            /// The registration conflict policy.
            conflict_policy: HostBackgroundConflictPolicy,
        }

        /// One host background task-list response payload.
        struct HostBackgroundListResponse {
            /// The request status code.
            status: host_status,
            /// The returned task descriptors.
            descriptors: slice(HostBackgroundTaskDescriptor),
        }

        /// One host background unregister request payload.
        struct HostBackgroundUnregisterRequest {
            /// The stable task identifier.
            identifier: string_ref,
        }

        /// One host background trigger-test request payload.
        struct HostBackgroundTriggerTestRequest {
            /// The stable task identifier.
            identifier: string_ref,
        }

        /// One host background trigger-test response payload.
        struct HostBackgroundTriggerTestResponse {
            /// The request status code.
            status: host_status,
            /// Whether the host triggered one task execution.
            is_triggered: bool,
        }

        /// One host background complete request payload.
        struct HostBackgroundCompleteRequest {
            /// The stable execution identifier.
            execution_id: string_ref,
            /// The completion result.
            result: HostBackgroundTaskResult,
        }

        /// One host background event metadata payload.
        struct HostBackgroundEventMetadata {
            /// The monotonic event timestamp in nanoseconds.
            timestamp_ns: u64,
            /// The monotonic sequence number for this event stream.
            sequence: u64,
            /// The stable task identifier.
            identifier: string_ref,
            /// The stable execution identifier for this scheduled execution.
            execution_id: string_ref,
            /// The host execution deadline in UTC nanoseconds.
            deadline_unix_ns: u64,
        }

        /// One host background event kind payload.
        enum HostBackgroundEventKind: u32 {
            /// One task-ready event.
            TaskReady = 1,
            /// One task-expired event.
            TaskExpired = 2,
        }

        /// One host background event payload.
        struct HostBackgroundEvent {
            /// The event kind.
            kind: HostBackgroundEventKind,
            /// The shared event metadata.
            metadata: HostBackgroundEventMetadata,
        }
    }
}
