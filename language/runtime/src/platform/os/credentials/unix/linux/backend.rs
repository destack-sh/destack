use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::credentials::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned, OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    OS_CREDENTIALS_CONTAINS_OPERATION, OS_CREDENTIALS_DELETE_OPERATION,
    OS_CREDENTIALS_READ_OPERATION, OS_CREDENTIALS_WRITE_OPERATION, already_exists,
    with_no_replace_write_guard,
};
use crate::platform::os::credentials::unix::linux::core::{
    map_keyring_error, open_keyring_entry, write_entry_secret,
};
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationPolicy, CredentialAuthenticationResult,
};
use crate::runtime::BindingCallContext;

/// Read one credential record from the Linux keyring backend.
pub(crate) fn read_credentials(
    _binding: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // reject access-group routes because linux keyring has no access-group model
    if query.access_group.is_some() {
        return Err(core_platform::not_supported(OS_CREDENTIALS_READ_OPERATION));
    }

    // reject auth-required reads because this backend has no interactive auth lane
    if query.require_authentication {
        return Err(core_platform::not_supported(OS_CREDENTIALS_READ_OPERATION));
    }

    // resolve one keyring entry from service and account
    let entry = open_keyring_entry(
        &query.service,
        &query.account,
        OS_CREDENTIALS_READ_OPERATION,
        "open entry",
    )?;

    // load one binary secret payload
    let bytes = entry.get_secret().map_err(|error| {
        map_keyring_error(OS_CREDENTIALS_READ_OPERATION, "read credential", error)
    })?;

    Ok(CredentialRecordOwned {
        service: query.service.clone(),
        account: query.account.clone(),
        bytes,
        created_unix_ns: 0,
        modified_unix_ns: 0,
    })
}

/// Write one credential record to the Linux keyring backend.
pub(crate) fn write_credentials(
    binding: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    // reject access-group routes because linux keyring has no access-group model
    if options.access_group.is_some() {
        return Err(core_platform::not_supported(OS_CREDENTIALS_WRITE_OPERATION));
    }

    // reject accessibility classes that this backend cannot enforce
    if options.accessibility != CredentialAccessibility::HostDefault {
        return Err(core_platform::not_supported(OS_CREDENTIALS_WRITE_OPERATION));
    }

    // reject authentication policies that this backend cannot enforce
    if options.authentication != CredentialAuthenticationPolicy::None {
        return Err(core_platform::not_supported(OS_CREDENTIALS_WRITE_OPERATION));
    }

    // reject duplicate writes when replacement is disabled
    if !options.replace_existing {
        // serialize check-then-write in-runtime: this is strict per runtime, not cross-process atomic
        return with_no_replace_write_guard(binding, || {
            let exists = contains_credentials(binding, &options.service, &options.account, None)?;
            if exists {
                return Err(already_exists(
                    OS_CREDENTIALS_WRITE_OPERATION,
                    "credential already exists and replaceExisting is false",
                ));
            }

            // write the new secret while the no-replace guard is held
            write_entry_secret(
                &options.service,
                &options.account,
                &options.bytes,
                OS_CREDENTIALS_WRITE_OPERATION,
            )
        });
    }

    // write one replacement secret payload
    write_entry_secret(
        &options.service,
        &options.account,
        &options.bytes,
        OS_CREDENTIALS_WRITE_OPERATION,
    )
}

/// Delete one credential record from the Linux keyring backend.
pub(crate) fn delete_credentials(
    _binding: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    // reject access-group routes because linux keyring has no access-group model
    if access_group.is_some() {
        return Err(core_platform::not_supported(
            OS_CREDENTIALS_DELETE_OPERATION,
        ));
    }

    // resolve one keyring entry from service and account
    let entry = open_keyring_entry(
        service,
        account,
        OS_CREDENTIALS_DELETE_OPERATION,
        "open entry",
    )?;

    // delete one credential payload
    entry.delete_credential().map_err(|error| {
        map_keyring_error(OS_CREDENTIALS_DELETE_OPERATION, "delete credential", error)
    })?;

    Ok(())
}

/// Return whether one credential record exists in the Linux keyring backend.
pub(crate) fn contains_credentials(
    _binding: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    // reject access-group routes because linux keyring has no access-group model
    if access_group.is_some() {
        return Err(core_platform::not_supported(
            OS_CREDENTIALS_CONTAINS_OPERATION,
        ));
    }

    // resolve one keyring entry from service and account
    let entry = open_keyring_entry(
        service,
        account,
        OS_CREDENTIALS_CONTAINS_OPERATION,
        "open entry",
    )?;

    // probe one credential payload and translate no-entry into false
    let result = entry.get_secret();
    match result {
        Ok(_) => Ok(true),
        Err(error) => {
            if matches!(error, keyring::Error::NoEntry) {
                return Ok(false);
            }

            Err(map_keyring_error(
                OS_CREDENTIALS_CONTAINS_OPERATION,
                "query credential",
                error,
            ))
        }
    }
}

/// Run one host authentication challenge on Linux keyring backend.
pub(crate) fn authenticate_credentials(
    _binding: &BindingCallContext,
    _options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    Err(core_platform::not_supported(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    ))
}
