use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host notification priority payload.
        #[value(crate::platform::os::abi_generated::NotificationPriorityValue)]
        enum HostNotificationPriority: u32 {
            /// One low-priority notification.
            Low = 1,
            /// One normal-priority notification.
            Normal = 2,
            /// One high-priority notification.
            High = 3,
        }

        /// One host notification calendar trigger payload.
        #[value(crate::platform::os::abi_generated::NotificationCalendarTriggerValue)]
        struct HostNotificationCalendarTrigger {
            /// The trigger year.
            year: u16,
            /// The trigger month, 1 to 12.
            month: u8,
            /// The trigger day of month, 1 to 31.
            day: u8,
            /// The trigger hour, 0 to 23.
            hour: u8,
            /// The trigger minute, 0 to 59.
            minute: u8,
            /// The trigger second, 0 to 59.
            second: u8,
            /// The trigger timezone identifier.
            time_zone: string_ref,
            /// Whether this trigger repeats.
            repeats: bool,
        }

        /// One host notification calendar-date trigger payload.
        #[value(crate::platform::os::abi_generated::NotificationCalendarDateTriggerValue)]
        struct HostNotificationCalendarDateTrigger {
            /// The trigger variant discriminator.
            kind: string_ref,
            /// The calendar trigger payload.
            calendar: HostNotificationCalendarTrigger,
        }

        /// One host notification immediate trigger payload.
        #[value(crate::platform::os::abi_generated::NotificationImmediateTriggerValue)]
        struct HostNotificationImmediateTrigger {
            /// The trigger variant discriminator.
            kind: string_ref,
        }

        /// One host notification time-interval trigger payload.
        #[value(crate::platform::os::abi_generated::NotificationTimeIntervalTriggerValue)]
        struct HostNotificationTimeIntervalTrigger {
            /// The trigger variant discriminator.
            kind: string_ref,
            /// The time-interval trigger delay in nanoseconds.
            interval_ns: u64,
        }

        /// One host notification trigger payload.
        #[value(crate::platform::os::abi_generated::NotificationTriggerValue)]
        enum HostNotificationTrigger {
            /// One calendar-date trigger.
            NotificationCalendarDateTrigger(HostNotificationCalendarDateTrigger),
            /// One immediate trigger.
            NotificationImmediateTrigger(HostNotificationImmediateTrigger),
            /// One time-interval trigger.
            NotificationTimeIntervalTrigger(HostNotificationTimeIntervalTrigger),
        }

        /// One host notification request payload.
        #[value(crate::platform::os::abi_generated::NotificationRequestValue)]
        struct HostNotificationRequest {
            /// The primary notification title.
            title: string_ref,
            /// The subtitle when present.
            subtitle: option(string_ref),
            /// The primary notification body text.
            body: string_ref,
            /// The host notification tag.
            tag: string_ref,
            /// The channel identifier when present.
            channel_id: option(string_ref),
            /// The priority class.
            priority: HostNotificationPriority,
            /// The badge count when present.
            badge_count: option(u32),
            /// The sound identifier when present.
            sound: option(string_ref),
            /// The category identifier when present.
            category_id: option(string_ref),
            /// The thread identifier when present.
            thread_id: option(string_ref),
            /// The delivery trigger selector.
            trigger: HostNotificationTrigger,
            /// The action identifier when present.
            action_id: option(string_ref),
        }

        /// One host notification event metadata payload.
        #[value(crate::platform::os::abi_generated::NotificationEventMetadataValue)]
        struct HostNotificationEventMetadata {
            /// The monotonic event timestamp in nanoseconds.
            timestamp_ns: u64,
            /// The monotonic sequence number for this event stream.
            sequence: u64,
            /// The host notification identifier.
            id: string_ref,
            /// The notification request payload.
            request: HostNotificationRequest,
        }

        /// One host notification interacted payload.
        #[value(crate::platform::os::abi_generated::NotificationInteractedPayloadValue)]
        struct HostNotificationInteractedPayload {
            /// The action identifier when present.
            action_id: option(string_ref),
            /// The text-input response when present.
            action_response_text: option(string_ref),
        }

        /// One host notification delivered event payload.
        #[value(crate::platform::os::abi_generated::NotificationDeliveredEventValue)]
        struct HostNotificationDeliveredEvent {
            /// The notification interaction discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostNotificationEventMetadata,
        }

        /// One host notification dismissed event payload.
        #[value(crate::platform::os::abi_generated::NotificationDismissedEventValue)]
        struct HostNotificationDismissedEvent {
            /// The notification interaction discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostNotificationEventMetadata,
        }

        /// One host notification interacted event payload.
        #[value(crate::platform::os::abi_generated::NotificationInteractedEventValue)]
        struct HostNotificationInteractedEvent {
            /// The notification interaction discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostNotificationEventMetadata,
            /// The interaction payload.
            payload: HostNotificationInteractedPayload,
        }

        /// One host notification event payload.
        #[value(crate::platform::os::abi_generated::NotificationEventValue)]
        enum HostNotificationEvent {
            /// The notification was delivered.
            NotificationDeliveredEvent(HostNotificationDeliveredEvent),
            /// The notification was dismissed.
            NotificationDismissedEvent(HostNotificationDismissedEvent),
            /// The notification was interacted with by the user.
            NotificationInteractedEvent(HostNotificationInteractedEvent),
        }
    }
}
