use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host intent event metadata payload.
        #[value(crate::platform::os::abi_generated::IntentEventMetadataValue)]
        struct HostIntentEventMetadata {
            /// The monotonic event timestamp in nanoseconds.
            timestamp_ns: u64,
            /// The monotonic sequence number for this event stream.
            sequence: u64,
            /// The source package, bundle, or process identifier when available.
            source: option(string_ref),
        }

        /// One host open-url intent payload.
        #[value(crate::platform::os::abi_generated::IntentOpenUrlPayloadValue)]
        struct HostIntentOpenUrlPayload {
            /// The URL payload from the host.
            url: string_ref,
        }

        /// One host open-url intent event payload.
        #[value(crate::platform::os::abi_generated::IntentOpenUrlEventValue)]
        struct HostIntentOpenUrlEvent {
            /// The intent event discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostIntentEventMetadata,
            /// The open-url payload.
            payload: HostIntentOpenUrlPayload,
        }

        /// One host open-file intent payload.
        #[value(crate::platform::os::abi_generated::IntentOpenFilePayloadValue)]
        struct HostIntentOpenFilePayload {
            /// The path payload from the host.
            path: os_path,
            /// The normalized content type when available.
            mime_type: option(string_ref),
        }

        /// One host open-file intent event payload.
        #[value(crate::platform::os::abi_generated::IntentOpenFileEventValue)]
        struct HostIntentOpenFileEvent {
            /// The intent event discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostIntentEventMetadata,
            /// The open-file payload.
            payload: HostIntentOpenFilePayload,
        }

        /// One host share-text intent payload.
        #[value(crate::platform::os::abi_generated::IntentShareTextPayloadValue)]
        struct HostIntentShareTextPayload {
            /// The shared text payload from the host.
            text: string_ref,
            /// The normalized content type when available.
            mime_type: option(string_ref),
        }

        /// One host share-text intent event payload.
        #[value(crate::platform::os::abi_generated::IntentShareTextEventValue)]
        struct HostIntentShareTextEvent {
            /// The intent event discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostIntentEventMetadata,
            /// The share-text payload.
            payload: HostIntentShareTextPayload,
        }

        /// One host share-files intent payload.
        #[value(crate::platform::os::abi_generated::IntentShareFilesPayloadValue)]
        struct HostIntentShareFilesPayload {
            /// The shared path payloads from the host.
            paths: slice(os_path),
            /// The normalized content type when available.
            mime_type: option(string_ref),
        }

        /// One host share-files intent event payload.
        #[value(crate::platform::os::abi_generated::IntentShareFilesEventValue)]
        struct HostIntentShareFilesEvent {
            /// The intent event discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostIntentEventMetadata,
            /// The share-files payload.
            payload: HostIntentShareFilesPayload,
        }

        /// One host custom-action intent payload.
        #[value(crate::platform::os::abi_generated::IntentCustomActionPayloadValue)]
        struct HostIntentCustomActionPayload {
            /// The custom action identifier from the host.
            action: string_ref,
            /// The URL payload when available.
            url: option(string_ref),
            /// The file-path payloads when available.
            paths: slice(os_path),
            /// The shared text payload when available.
            text: option(string_ref),
            /// The normalized content type when available.
            mime_type: option(string_ref),
        }

        /// One host custom-action intent event payload.
        #[value(crate::platform::os::abi_generated::IntentCustomActionEventValue)]
        struct HostIntentCustomActionEvent {
            /// The intent event discriminator.
            kind: string_ref,
            /// The shared event metadata.
            metadata: HostIntentEventMetadata,
            /// The custom-action payload.
            payload: HostIntentCustomActionPayload,
        }

        /// One host intent event payload.
        #[value(crate::platform::os::abi_generated::IntentEventValue)]
        enum HostIntentEvent {
            /// One open-url intent event.
            IntentOpenUrlEvent(HostIntentOpenUrlEvent),
            /// One open-file intent event.
            IntentOpenFileEvent(HostIntentOpenFileEvent),
            /// One share-text intent event.
            IntentShareTextEvent(HostIntentShareTextEvent),
            /// One share-files intent event.
            IntentShareFilesEvent(HostIntentShareFilesEvent),
            /// One custom-action intent event.
            IntentCustomActionEvent(HostIntentCustomActionEvent),
        }
    }
}
