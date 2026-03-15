use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::{
    NotificationPermissionState, Permission, PermissionEntry, PermissionState,
};
use crate::platform::{PlatformError, fs};
use crate::runtime::world::RuntimeId;

/// Runtime scoped host request context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HostRequestContext {
    /// Runtime id for the active host session.
    pub(crate) runtime_id: RuntimeId,
    /// Host platform for the active adapter.
    pub(crate) platform: Platform,
    /// Whether the caller already runs on the process main context.
    pub(crate) is_process_main_context: bool,
}

/// Normalized outbound host command.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum HostRequest {
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
        /// MIME type associated with the text payload.
        mime_type: Option<String>,
    },
    /// Share one path list through the host.
    OsIntentSharePaths {
        /// Filesystem paths to share.
        paths: Vec<fs::OsPath>,
        /// MIME type associated with the shared files.
        mime_type: Option<String>,
    },
    /// Open one document picker through the host.
    OsDocumentPick {
        /// Picker options for the host document chooser.
        options: DocumentPickOptionsValue,
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
}

/// Completion semantics for one host request submission.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostRequestCompletion {
    /// Submission completed the visible host work immediately.
    Immediate,
    /// Submission was accepted and later state changes are deferred.
    Deferred,
    /// Submission completes through one later host event.
    EventCompleting,
}

/// Normalized host request result payload.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HostRequestResult {
    /// Request completed without one return payload.
    None,
    /// Request completed with one boolean payload.
    Bool(bool),
    /// Request completed with one document descriptor list.
    DocumentDescriptors(Vec<DocumentDescriptorValue>),
    /// Request completed with one permission state payload.
    PermissionState(PermissionState),
    /// Request completed with one permission entry list.
    PermissionEntries(Vec<PermissionEntry>),
    /// Request completed with one notification permission state payload.
    NotificationPermissionState(NotificationPermissionState),
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
            Self::OsIntentCanOpenUrl { .. } => "destack.os.intent.canOpenUrl",
            Self::OsIntentOpenUrl { .. } => "destack.os.intent.openUrl",
            Self::OsIntentOpenPath { .. } => "destack.os.intent.openPath",
            Self::OsIntentShareText { .. } => "destack.os.intent.shareText",
            Self::OsIntentSharePaths { .. } => "destack.os.intent.sharePaths",
            Self::OsDocumentPick { .. } => "destack.os.document.pick",
            Self::OsPermissionOpenSettings => "destack.os.permission.openSettings",
            Self::OsPermissionRequest { .. } => "destack.os.permission.request",
            Self::OsPermissionRequestMany { .. } => "destack.os.permission.requestMany",
            Self::OsNotificationRequestPermission => "destack.os.notification.requestPermission",
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

    /// Decode one boolean request payload.
    pub(crate) fn into_bool(self, operation: &'static str) -> RuntimeResult<bool> {
        match self.result {
            HostRequestResult::Bool(value) => Ok(value),
            _ => Err(unexpected_request_result(operation, "bool")),
        }
    }

    /// Decode one document descriptor list payload.
    pub(crate) fn into_document_descriptors(
        self,
        operation: &'static str,
    ) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
        match self.result {
            HostRequestResult::DocumentDescriptors(value) => Ok(value),
            _ => Err(unexpected_request_result(operation, "document descriptors")),
        }
    }

    /// Decode one permission state payload.
    pub(crate) fn into_permission_state(
        self,
        operation: &'static str,
    ) -> RuntimeResult<PermissionState> {
        match self.result {
            HostRequestResult::PermissionState(value) => Ok(value),
            _ => Err(unexpected_request_result(operation, "permission state")),
        }
    }

    /// Decode one permission entry list payload.
    pub(crate) fn into_permission_entries(
        self,
        operation: &'static str,
    ) -> RuntimeResult<Vec<PermissionEntry>> {
        match self.result {
            HostRequestResult::PermissionEntries(value) => Ok(value),
            _ => Err(unexpected_request_result(operation, "permission entries")),
        }
    }

    /// Decode one notification permission state payload.
    pub(crate) fn into_notification_permission_state(
        self,
        operation: &'static str,
    ) -> RuntimeResult<NotificationPermissionState> {
        match self.result {
            HostRequestResult::NotificationPermissionState(value) => Ok(value),
            _ => Err(unexpected_request_result(
                operation,
                "notification permission state",
            )),
        }
    }
}

/// Return one explicit host request result mismatch error.
fn unexpected_request_result(
    operation: &'static str,
    expected: &'static str,
) -> Box<crate::diagnostic::RuntimeError> {
    crate::diagnostic::RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoInvalidData),
        format!("{operation} returned one unexpected host request result, expected {expected}"),
    ))
    .boxed()
}
