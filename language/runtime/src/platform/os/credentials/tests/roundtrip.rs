use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::credentials::tests::tests::{
    assert_runtime_error_code, credential_query_value, credential_write_options_value,
    decode_credential_record_value, optional_string_value, string_value, unique_suffix,
};
use crate::platform::os::tests::with_harness_context;
use crate::platform::os::{CredentialAccessibility, CredentialAuthenticationPolicy};

/// Verify credentials roundtrip behavior and parity across native and vm bindings.
#[test]
fn test_credentials_roundtrip() {
    with_harness_context(|mut context| {
        // build one unique service and account key for this run
        let suffix = unique_suffix();
        let service = format!("destack.os.tests.credentials.service.{suffix}");
        let account = format!("destack.os.tests.credentials.account.{suffix}");
        let payload = b"destack-os-credentials-roundtrip-payload";

        // write one credential record
        let write_options = credential_write_options_value(
            &mut context,
            &service,
            &account,
            None,
            payload,
            CredentialAccessibility::HostDefault,
            CredentialAuthenticationPolicy::None,
            true,
        )?;
        context.destack_os_credentials_write(write_options)?;

        // verify contains returns true after write
        let service_value = string_value(&mut context, &service);
        let account_value = string_value(&mut context, &account);
        let access_group_value = optional_string_value(&mut context, None);
        let is_present = context.destack_os_credentials_contains(
            service_value,
            account_value,
            access_group_value,
        )?;
        assert!(is_present);

        // read back the written record and verify every field
        let query = credential_query_value(&mut context, &service, &account, None, false);
        let record = context.destack_os_credentials_read(query)?;
        let (record_service, record_account, record_bytes) =
            decode_credential_record_value(&mut context, record)?;
        assert_eq!(record_service, service);
        assert_eq!(record_account, account);
        assert_eq!(record_bytes, payload);

        // reject duplicate writes when replacement is disabled
        let duplicate_write_options = credential_write_options_value(
            &mut context,
            &service,
            &account,
            None,
            payload,
            CredentialAccessibility::HostDefault,
            CredentialAuthenticationPolicy::None,
            false,
        )?;
        let duplicate_result = context.destack_os_credentials_write(duplicate_write_options);
        let error = match duplicate_result {
            Ok(_) => panic!("duplicate write should fail when replaceExisting is false"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoAlreadyExists);

        // replace one existing record when replacement is enabled
        let replacement_payload = b"destack-os-credentials-roundtrip-replacement-payload";
        let replacement_write_options = credential_write_options_value(
            &mut context,
            &service,
            &account,
            None,
            replacement_payload,
            CredentialAccessibility::HostDefault,
            CredentialAuthenticationPolicy::None,
            true,
        )?;
        context.destack_os_credentials_write(replacement_write_options)?;

        // verify read returns replacement bytes after duplicate-update path
        let replacement_query =
            credential_query_value(&mut context, &service, &account, None, false);
        let replacement_record = context.destack_os_credentials_read(replacement_query)?;
        let (_record_service, _record_account, replacement_record_bytes) =
            decode_credential_record_value(&mut context, replacement_record)?;
        assert_eq!(replacement_record_bytes, replacement_payload);

        // delete the record and verify contains returns false
        let service_value = string_value(&mut context, &service);
        let account_value = string_value(&mut context, &account);
        let access_group_value = optional_string_value(&mut context, None);
        context.destack_os_credentials_delete(service_value, account_value, access_group_value)?;

        let service_value = string_value(&mut context, &service);
        let account_value = string_value(&mut context, &account);
        let access_group_value = optional_string_value(&mut context, None);
        let is_present = context.destack_os_credentials_contains(
            service_value,
            account_value,
            access_group_value,
        )?;
        assert!(!is_present);

        // verify reads after delete return ioNotFound
        let query = credential_query_value(&mut context, &service, &account, None, false);
        let read_result = context.destack_os_credentials_read(query);
        let error = match read_result {
            Ok(_) => panic!("read should fail after delete"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoNotFound);

        Ok(())
    });
}
