use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult, HostRuntimeId,
};
use crate::host::linux::linux_notify_location_sample;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue, LocationSampleValue,
    LocationWatchOptionsValue,
};

/// Shared location hook set used by Linux tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LinuxLocationHooks {
    /// Hook for `locationServicesEnabled`.
    pub(crate) services_enabled: Option<fn() -> RuntimeResult<bool>>,
    /// Hook for `locationLastKnown`.
    pub(crate) last_known: Option<fn() -> RuntimeResult<LocationSampleValue>>,
    /// Hook for `locationWatchOpen`.
    pub(crate) watch_open:
        Option<fn(HostRuntimeId, String, LocationWatchOptionsValue) -> RuntimeResult<()>>,
    /// Hook for `locationWatchClose`.
    pub(crate) watch_close: Option<fn(HostRuntimeId, String) -> RuntimeResult<()>>,
}

/// Shared Linux contact hook set used by tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LinuxContactHooks {
    /// Hook for `contactList`.
    pub(crate) list: Option<fn(ContactQueryValue) -> RuntimeResult<ContactPageValue>>,
    /// Hook for `contactSearch`.
    pub(crate) search: Option<fn(String, ContactQueryValue) -> RuntimeResult<ContactPageValue>>,
    /// Hook for `contactRead`.
    pub(crate) read: Option<fn(String) -> RuntimeResult<ContactValue>>,
    /// Hook for `contactCreate`.
    pub(crate) create: Option<fn(ContactDraftValue) -> RuntimeResult<String>>,
    /// Hook for `contactUpdate`.
    pub(crate) update: Option<fn(String, ContactDraftValue) -> RuntimeResult<()>>,
    /// Hook for `contactDelete`.
    pub(crate) delete: Option<fn(String) -> RuntimeResult<()>>,
}

/// Return the shared Linux location hook slot for tests.
fn linux_location_hook_slot() -> &'static Mutex<LinuxLocationHooks> {
    static HOOK: OnceLock<Mutex<LinuxLocationHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(LinuxLocationHooks::default()))
}

/// Return the shared Linux contact hook slot for tests.
fn linux_contact_hook_slot() -> &'static Mutex<LinuxContactHooks> {
    static HOOK: OnceLock<Mutex<LinuxContactHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(LinuxContactHooks::default()))
}

/// Install one Linux location hook set for tests.
pub(crate) fn set_linux_location_test_hooks(hooks: LinuxLocationHooks) {
    let mut slot = linux_location_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Install one Linux contact hook set for tests.
pub(crate) fn set_linux_contact_test_hooks(hooks: LinuxContactHooks) {
    let mut slot = linux_contact_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Return the active Linux location hooks for tests.
fn location_hooks() -> LinuxLocationHooks {
    let slot = linux_location_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot
}

/// Return the active Linux contact hooks for tests.
fn contact_hooks() -> LinuxContactHooks {
    let slot = linux_contact_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot
}

/// Submit one Linux contact request from the host request lane in tests.
pub(crate) fn submit_contact_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let hooks = contact_hooks();

    match request {
        HostRequest::OsContactList { query } => {
            let Some(hook) = hooks.list else {
                return Err(missing_contact_test_hook("contact list"));
            };
            let page = hook(query.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }
        HostRequest::OsContactSearch { query_text, query } => {
            let Some(hook) = hooks.search else {
                return Err(missing_contact_test_hook("contact search"));
            };
            let page = hook(query_text.clone(), query.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }
        HostRequest::OsContactRead { id } => {
            let Some(hook) = hooks.read else {
                return Err(missing_contact_test_hook("contact read"));
            };
            let contact = hook(id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Contact(contact),
            )))
        }
        HostRequest::OsContactCreate { contact } => {
            let Some(hook) = hooks.create else {
                return Err(missing_contact_test_hook("contact create"));
            };
            let id = hook(contact.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsContactUpdate { id, contact } => {
            let Some(hook) = hooks.update else {
                return Err(missing_contact_test_hook("contact update"));
            };
            hook(id.clone(), contact.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsContactDelete { id } => {
            let Some(hook) = hooks.delete else {
                return Err(missing_contact_test_hook("contact delete"));
            };
            hook(id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Submit one Linux location request from the host request lane in tests.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let hooks = location_hooks();

    match request {
        // one services-enabled read
        HostRequest::OsLocationServicesEnabled => {
            let Some(hook) = hooks.services_enabled else {
                return Err(missing_location_test_hook("location services"));
            };
            let is_enabled = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_enabled),
            )))
        }

        // one last-known read
        HostRequest::OsLocationLastKnown => {
            let Some(hook) = hooks.last_known else {
                return Err(missing_location_test_hook("last known location"));
            };
            let sample = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }

        // one watch-open request
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            let Some(hook) = hooks.watch_open else {
                return Err(missing_location_test_hook("location watch open"));
            };

            hook(context.host_runtime_id, watch_id.clone(), *options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // one watch-close request
        HostRequest::OsLocationWatchClose { watch_id } => {
            let Some(hook) = hooks.watch_close else {
                return Err(missing_location_test_hook("location watch close"));
            };

            hook(context.host_runtime_id, watch_id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Remove one Linux runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(_host_runtime_id: HostRuntimeId) {}

/// Publish one Linux location sample from tests.
pub(crate) fn publish_location_sample(
    host_runtime_id: HostRuntimeId,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    linux_notify_location_sample(host_runtime_id.0, watch_id, sample)
}

/// Build one missing-hook error for Linux location tests.
fn missing_location_test_hook(kind: &str) -> Box<crate::diagnostic::RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("Linux location tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}

/// Build one missing-hook error for Linux contact tests.
fn missing_contact_test_hook(kind: &str) -> Box<crate::diagnostic::RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("Linux contact tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}
