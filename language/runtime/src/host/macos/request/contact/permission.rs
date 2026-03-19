use std::sync::mpsc::channel;

use block2::RcBlock;
use objc2_contacts::{CNAuthorizationStatus, CNContactStore, CNEntityType};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::apple::execution::call_process_main_context_if_needed;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{Permission, PermissionEntry, PermissionState};

/// Request one supported macOS contact permission.
pub(in crate::host::macos::request) fn request_contact_permission(
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    if !matches!(
        permission,
        Permission::ContactsRead | Permission::ContactsWrite
    ) {
        return Ok(None);
    }

    let authorization_status = request_contact_authorization_status()?;
    let permission_state = permission_state_from_contact_status(authorization_status);

    Ok(Some(permission_state))
}

/// Request one supported macOS contact permission batch.
pub(in crate::host::macos::request) fn request_contact_permissions(
    permissions: &[Permission],
) -> RuntimeResult<Option<Vec<PermissionEntry>>> {
    if permissions.iter().any(|permission| {
        !matches!(
            permission,
            Permission::ContactsRead | Permission::ContactsWrite
        )
    }) {
        return Ok(None);
    }

    let authorization_status = if permissions.is_empty() {
        CNAuthorizationStatus::NotDetermined
    } else {
        request_contact_authorization_status()?
    };

    let entries = permissions
        .iter()
        .map(|permission| PermissionEntry {
            permission: *permission,
            state: permission_state_from_contact_status(authorization_status),
        })
        .collect();

    Ok(Some(entries))
}

/// Request one Contacts authorization status through Contacts.
fn request_contact_authorization_status() -> RuntimeResult<CNAuthorizationStatus> {
    call_process_main_context_if_needed(move || {
        let store = unsafe { CNContactStore::new() };
        let (sender, receiver) = channel();
        let completion = RcBlock::new(
            move |_granted: objc2::runtime::Bool, error: *mut objc2_foundation::NSError| {
                let result = if error.is_null() {
                    let status = unsafe {
                        CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts)
                    };

                    Ok(status)
                } else {
                    Err(permission_error("Contacts permission request failed"))
                };

                if sender.send(result).is_err() {
                    tracing::warn!(
                        target: "destack.runtime.host.macos.permission",
                        "contacts permission callback result receiver dropped",
                    );
                }
            },
        );

        // request access on the active process main context
        unsafe {
            store.requestAccessForEntityType_completionHandler(CNEntityType::Contacts, &completion);
        }

        receiver.recv().map_err(|error| {
            permission_error(format!(
                "could not receive one Contacts permission result: {error}"
            ))
        })?
    })
}

/// Map one Contacts authorization status into one runtime permission state.
fn permission_state_from_contact_status(status: CNAuthorizationStatus) -> PermissionState {
    match status {
        CNAuthorizationStatus::NotDetermined => PermissionState::Prompt,
        CNAuthorizationStatus::Restricted => PermissionState::Restricted,
        CNAuthorizationStatus::Denied => PermissionState::Denied,
        CNAuthorizationStatus::Authorized => PermissionState::Granted,
        CNAuthorizationStatus::Limited => PermissionState::Limited,
        _ => PermissionState::Denied,
    }
}

/// Build one loud Contacts permission error.
fn permission_error(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("destack.os.permission.request: {}", message.into()),
    ))
    .boxed()
}
