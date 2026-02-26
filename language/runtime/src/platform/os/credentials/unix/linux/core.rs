use crate::diagnostic::RuntimeError;
use crate::platform::os::credentials::core::{invalid_data, not_found, permission_denied};

/// Resolve one keyring entry from service and account.
pub(crate) fn open_keyring_entry(
    service: &str,
    account: &str,
    operation: &'static str,
    action: &'static str,
) -> Result<keyring::Entry, Box<RuntimeError>> {
    keyring::Entry::new(service, account)
        .map_err(|error| map_keyring_error(operation, action, error))
}

/// Write one secret payload to one Linux keyring entry.
pub(crate) fn write_entry_secret(
    service: &str,
    account: &str,
    bytes: &[u8],
    operation: &'static str,
) -> Result<(), Box<RuntimeError>> {
    // resolve one keyring entry from service and account
    let entry = open_keyring_entry(service, account, operation, "open entry")?;

    // store one binary secret payload
    entry
        .set_secret(bytes)
        .map_err(|error| map_keyring_error(operation, "write credential", error))?;

    Ok(())
}

/// Map one keyring backend error into one runtime error.
pub(crate) fn map_keyring_error(
    operation: &'static str,
    action: &'static str,
    error: keyring::Error,
) -> Box<RuntimeError> {
    // map record-not-found errors
    if matches!(error, keyring::Error::NoEntry) {
        return not_found(
            operation,
            format!("linux keyring {action} could not find one credential"),
        );
    }

    // map storage-access denials
    if matches!(error, keyring::Error::NoStorageAccess(_)) {
        return permission_denied(
            operation,
            format!("linux keyring {action} was denied by host storage policy"),
        );
    }

    // map host platform failures into ioInvalidData
    if matches!(error, keyring::Error::PlatformFailure(_)) {
        return invalid_data(
            operation,
            format!("linux keyring {action} failed due to host platform error"),
        );
    }

    // map all remaining backend errors to ioInvalidData
    invalid_data(operation, format!("linux keyring {action} failed: {error}"))
}
