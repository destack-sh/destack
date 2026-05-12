use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::host::Platform;
use crate::host::core::registry::HostSessionId;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
    BackgroundTaskResultValue, CalendarDescriptorValue, CalendarEventDraftValue,
    CalendarEventQueryValue, CalendarEventValue, ContactDraftValue, ContactPageValue,
    ContactQueryValue, ContactValue, DocumentDescriptorValue, DocumentPickOptionsValue,
    LocationSampleValue, LocationWatchOptionsValue, MediaAssetDescriptorValue, MediaPageValue,
    MediaQueryValue, NotificationCategoryValue, NotificationRequestValue,
    NotificationScheduledDescriptorValue,
};
use crate::platform::os::{
    MediaAssetKind, NotificationPermissionState, Permission, PermissionEntry, PermissionState,
};
use crate::platform::{PlatformError, fs};
use destack_workspace::{AppIdentityOptions, HostOsOptions};

/// Stable runtime-session-scoped identifier for one outbound host request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HostRequestId(pub u64);

/// Runtime-session-scoped context shared by host ingress and request submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionContext {
    /// Process-global routing id for the active host session.
    pub(crate) host_session_id: HostSessionId,
    /// Host platform for the active adapter.
    pub(crate) platform: Platform,
    /// Runtime OS options for host-backed service state.
    pub(crate) os_options: HostOsOptions,
    /// Runtime app identity for host-facing integration.
    pub(crate) app_identity: AppIdentityOptions,
    /// Whether the caller already runs on the process main context.
    pub(crate) is_process_main_context: bool,
}

/// Runtime-session-scoped context for one outbound host request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestContext {
    /// Stable runtime-session-scoped identifier for this request submission.
    pub(crate) request_id: HostRequestId,
    /// Process-global routing id for the active host session.
    pub(crate) host_session_id: HostSessionId,
    /// Host platform for the active adapter.
    pub(crate) platform: Platform,
    /// Runtime OS options for host-backed service state.
    pub(crate) os_options: HostOsOptions,
    /// Runtime app identity for host-facing integration.
    pub(crate) app_identity: AppIdentityOptions,
    /// Whether the caller already runs on the process main context.
    pub(crate) is_process_main_context: bool,
}

/// Normalized outbound host command owned by the runtime layer.
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub(crate) enum HostRequest {
    /// Read one background scheduler status through the host.
    OsBackgroundStatus,
    /// List background task registrations through the host.
    OsBackgroundList,
    /// Register one background task through the host.
    OsBackgroundRegister {
        /// Task registration payload.
        options: BackgroundTaskOptionsValue,
    },
    /// Unregister one background task through the host.
    OsBackgroundUnregister {
        /// Stable task identifier.
        identifier: String,
    },
    /// Trigger one background task through the host test bridge.
    OsBackgroundTriggerTest {
        /// Stable task identifier.
        identifier: String,
    },
    /// Complete one active background task execution through the host.
    OsBackgroundComplete {
        /// Stable execution identifier.
        execution_id: String,
        /// Final execution result.
        result: BackgroundTaskResultValue,
    },
    /// List host calendars through the host.
    OsCalendarList,
    /// List host calendar events through the host.
    OsCalendarEventList {
        /// Calendar query payload.
        query: CalendarEventQueryValue,
    },
    /// Read one host calendar event through the host.
    OsCalendarEventRead {
        /// Stable event identifier.
        id: String,
    },
    /// Create one host calendar event through the host.
    OsCalendarEventCreate {
        /// Calendar draft payload.
        event: CalendarEventDraftValue,
    },
    /// Update one host calendar event through the host.
    OsCalendarEventUpdate {
        /// Stable event identifier.
        id: String,
        /// Calendar draft payload.
        event: CalendarEventDraftValue,
    },
    /// Delete one host calendar event through the host.
    OsCalendarEventDelete {
        /// Stable event identifier.
        id: String,
    },
    /// List host contacts through the host.
    OsContactList {
        /// Contact query payload.
        query: ContactQueryValue,
    },
    /// Search host contacts through the host.
    OsContactSearch {
        /// Backend query string.
        query_text: String,
        /// Contact query payload.
        query: ContactQueryValue,
    },
    /// Read one host contact through the host.
    OsContactRead {
        /// Stable contact identifier.
        id: String,
    },
    /// Create one host contact through the host.
    OsContactCreate {
        /// Contact draft payload.
        contact: ContactDraftValue,
    },
    /// Update one host contact through the host.
    OsContactUpdate {
        /// Stable contact identifier.
        id: String,
        /// Contact draft payload.
        contact: ContactDraftValue,
    },
    /// Delete one host contact through the host.
    OsContactDelete {
        /// Stable contact identifier.
        id: String,
    },
    /// Query whether the host can route one URL.
    OsIntentCanOpenUrl {
        /// URL target to query.
        url: String,
    },
    /// Open one URL through the host.
    OsIntentOpenUrl {
        /// URL target to route.
        url: String,
    },
    /// Open one path through the host.
    OsIntentOpenPath {
        /// Filesystem path to route.
        path: fs::OsPath,
    },
    /// Share one text payload through the host.
    OsIntentShareText {
        /// Text payload to share.
        text: String,
        /// Normalized content type associated with the text payload.
        content_type: Option<String>,
    },
    /// Share one path list through the host.
    OsIntentSharePaths {
        /// Filesystem paths to share.
        paths: Vec<fs::OsPath>,
        /// Normalized content type associated with the shared files.
        content_type: Option<String>,
    },
    /// Open one document picker through the host.
    OsDocumentPick {
        /// Picker options for the host document chooser.
        options: DocumentPickOptionsValue,
    },
    /// Read whether host location services are enabled.
    OsLocationServicesEnabled,
    /// Read one cached host location sample.
    OsLocationLastKnown,
    /// Open one host location watch.
    OsLocationWatchOpen {
        /// Runtime-scoped watch identifier.
        watch_id: String,
        /// Location watch policy.
        options: LocationWatchOptionsValue,
    },
    /// Close one host location watch.
    OsLocationWatchClose {
        /// Runtime-scoped watch identifier.
        watch_id: String,
    },
    /// List media assets through the host.
    OsMediaList {
        /// Media query payload.
        query: MediaQueryValue,
    },
    /// Read one media asset through the host.
    OsMediaRead {
        /// Stable asset identifier.
        id: String,
    },
    /// Import one path into the host media library.
    OsMediaImportPath {
        /// Runtime-visible path to import.
        path: fs::OsPath,
        /// Declared media asset kind.
        kind: MediaAssetKind,
    },
    /// Delete media assets through the host.
    OsMediaDelete {
        /// Stable asset identifiers to delete.
        ids: Vec<String>,
    },
    /// Open host settings for runtime permissions.
    OsPermissionOpenSettings,
    /// Request one permission through the host.
    OsPermissionRequest {
        /// Permission selector to request.
        permission: Permission,
    },
    /// Request multiple permissions through the host.
    OsPermissionRequestMany {
        /// Permission selectors to request.
        permissions: Vec<Permission>,
    },
    /// Request notification permission through the host.
    OsNotificationRequestPermission,
    /// Cancel one posted notification through the host.
    OsNotificationCancel {
        /// Host notification identifier.
        id: String,
    },
    /// Cancel every posted notification through the host.
    OsNotificationCancelAll,
    /// List registered notification categories through the host.
    OsNotificationCategoryList,
    /// Register notification categories through the host.
    OsNotificationCategorySet {
        /// Categories to register on the host.
        categories: Vec<NotificationCategoryValue>,
    },
    /// List pending scheduled notifications through the host.
    OsNotificationPendingList,
    /// Cancel one pending scheduled notification through the host.
    OsNotificationPendingCancel {
        /// Scheduled notification identifier.
        id: String,
    },
    /// Cancel every pending scheduled notification through the host.
    OsNotificationPendingCancelAll,
    /// Post one host notification immediately.
    OsNotificationPost {
        /// Notification request payload.
        request: NotificationRequestValue,
    },
    /// Schedule one host notification.
    OsNotificationSchedule {
        /// Notification request payload.
        request: NotificationRequestValue,
    },
}

/// Completion semantics for one host request submission.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostRequestCompletion {
    /// Submission completed the visible host work immediately.
    Immediate,
    /// Submission was accepted and later completion arrives through one event.
    Deferred,
    /// Submission opened one long lived resource stream.
    OpenedResource,
}

/// Normalized host request result payload.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HostRequestResult {
    /// Request completed without one return payload.
    None,
    /// Request completed with one boolean payload.
    Bool(bool),
    /// Request completed with one string payload.
    String(String),
    /// Request completed with one unsigned 32-bit payload.
    U32(u32),
    /// Request completed with one background scheduler status payload.
    BackgroundStatus(BackgroundStatusValue),
    /// Request completed with one background task descriptor list payload.
    BackgroundTaskDescriptors(Vec<BackgroundTaskDescriptorValue>),
    /// Request completed with one calendar descriptor list payload.
    CalendarDescriptors(Vec<CalendarDescriptorValue>),
    /// Request completed with one calendar event list payload.
    CalendarEvents(Vec<CalendarEventValue>),
    /// Request completed with one calendar event payload.
    CalendarEvent(CalendarEventValue),
    /// Request completed with one contact page payload.
    ContactPage(ContactPageValue),
    /// Request completed with one contact payload.
    Contact(ContactValue),
    /// Request completed with one document descriptor list.
    DocumentDescriptors(Vec<DocumentDescriptorValue>),
    /// Request completed with one location sample payload.
    LocationSample(LocationSampleValue),
    /// Request completed with one media asset descriptor payload.
    MediaAssetDescriptor(MediaAssetDescriptorValue),
    /// Request completed with one media page payload.
    MediaPage(MediaPageValue),
    /// Request completed with one permission state payload.
    PermissionState(PermissionState),
    /// Request completed with one permission entry list.
    PermissionEntries(Vec<PermissionEntry>),
    /// Request completed with one notification permission state payload.
    NotificationPermissionState(NotificationPermissionState),
    /// Request completed with one notification identifier payload.
    NotificationId(String),
    /// Request completed with one notification category list payload.
    NotificationCategories(Vec<NotificationCategoryValue>),
    /// Request completed with one scheduled notification descriptor list payload.
    NotificationScheduledDescriptors(Vec<NotificationScheduledDescriptorValue>),
}

/// Normalized host request outcome.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HostRequestOutcome {
    /// Completion semantics for this request.
    pub(crate) completion: HostRequestCompletion,
    /// Result payload for this request.
    pub(crate) result: HostRequestResult,
}

impl HostRequest {
    /// Return the canonical operation name for this request.
    pub(crate) fn operation_name(&self) -> &'static str {
        match self {
            Self::OsBackgroundStatus => "destack.os.background.status",
            Self::OsBackgroundList => "destack.os.background.list",
            Self::OsBackgroundRegister { .. } => "destack.os.background.register",
            Self::OsBackgroundUnregister { .. } => "destack.os.background.unregister",
            Self::OsBackgroundTriggerTest { .. } => "destack.os.background.triggerTest",
            Self::OsBackgroundComplete { .. } => "destack.os.background.complete",
            Self::OsCalendarList => "destack.os.calendar.list",
            Self::OsCalendarEventList { .. } => "destack.os.calendar.eventList",
            Self::OsCalendarEventRead { .. } => "destack.os.calendar.eventRead",
            Self::OsCalendarEventCreate { .. } => "destack.os.calendar.eventCreate",
            Self::OsCalendarEventUpdate { .. } => "destack.os.calendar.eventUpdate",
            Self::OsCalendarEventDelete { .. } => "destack.os.calendar.eventDelete",
            Self::OsContactList { .. } => "destack.os.contact.list",
            Self::OsContactSearch { .. } => "destack.os.contact.search",
            Self::OsContactRead { .. } => "destack.os.contact.read",
            Self::OsContactCreate { .. } => "destack.os.contact.create",
            Self::OsContactUpdate { .. } => "destack.os.contact.update",
            Self::OsContactDelete { .. } => "destack.os.contact.delete",
            Self::OsIntentCanOpenUrl { .. } => "destack.os.intent.canOpenUrl",
            Self::OsIntentOpenUrl { .. } => "destack.os.intent.openUrl",
            Self::OsIntentOpenPath { .. } => "destack.os.intent.openPath",
            Self::OsIntentShareText { .. } => "destack.os.intent.shareText",
            Self::OsIntentSharePaths { .. } => "destack.os.intent.sharePaths",
            Self::OsDocumentPick { .. } => "destack.os.document.pick",
            Self::OsLocationServicesEnabled => "destack.os.location.servicesEnabled",
            Self::OsLocationLastKnown => "destack.os.location.lastKnown",
            Self::OsLocationWatchOpen { .. } => "destack.os.location.watchOpen",
            Self::OsLocationWatchClose { .. } => "destack.os.location.watchClose",
            Self::OsMediaList { .. } => "destack.os.media.list",
            Self::OsMediaRead { .. } => "destack.os.media.read",
            Self::OsMediaImportPath { .. } => "destack.os.media.importPath",
            Self::OsMediaDelete { .. } => "destack.os.media.delete",
            Self::OsPermissionOpenSettings => "destack.os.permission.openSettings",
            Self::OsPermissionRequest { .. } => "destack.os.permission.request",
            Self::OsPermissionRequestMany { .. } => "destack.os.permission.requestMany",
            Self::OsNotificationRequestPermission => "destack.os.notification.requestPermission",
            Self::OsNotificationCancel { .. } => "destack.os.notification.cancel",
            Self::OsNotificationCancelAll => "destack.os.notification.cancelAll",
            Self::OsNotificationCategoryList => "destack.os.notification.categoryList",
            Self::OsNotificationCategorySet { .. } => "destack.os.notification.categorySet",
            Self::OsNotificationPendingList => "destack.os.notification.pendingList",
            Self::OsNotificationPendingCancel { .. } => "destack.os.notification.pendingCancel",
            Self::OsNotificationPendingCancelAll => "destack.os.notification.pendingCancelAll",
            Self::OsNotificationPost { .. } => "destack.os.notification.post",
            Self::OsNotificationSchedule { .. } => "destack.os.notification.schedule",
        }
    }
}

impl HostRequestOutcome {
    /// Build one immediate host request outcome.
    pub(crate) const fn immediate(result: HostRequestResult) -> Self {
        Self {
            completion: HostRequestCompletion::Immediate,
            result,
        }
    }

    /// Build one deferred host request outcome.
    #[allow(dead_code)]
    pub(crate) const fn deferred(result: HostRequestResult) -> Self {
        Self {
            completion: HostRequestCompletion::Deferred,
            result,
        }
    }

    /// Build one resource-opening host request outcome.
    #[allow(dead_code)]
    pub(crate) const fn opened_resource(result: HostRequestResult) -> Self {
        Self {
            completion: HostRequestCompletion::OpenedResource,
            result,
        }
    }
}

/// Return one explicit host request result mismatch error.
pub(crate) fn unexpected_request_result(
    operation: &'static str,
    expected: &'static str,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoInvalidData),
        format!("{operation} returned one unexpected host request result, expected {expected}"),
    ))
    .boxed()
}
