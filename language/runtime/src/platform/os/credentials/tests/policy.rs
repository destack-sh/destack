use crate::platform::os::credentials::tests::tests::{
    assert_not_supported_error, credential_write_options_value, optional_string_value, string_value,
};
use crate::platform::os::tests::with_harness_context;
use crate::platform::os::{CredentialAccessibility, CredentialAuthenticationPolicy};

/// Verify unsupported write-policy lanes report notSupported on Linux and Windows.
#[cfg(any(unix, windows))]
#[test]
fn test_credentials_write_policy_lanes_on_linux_and_windows() {
    with_harness_context(|mut context| {
        // this policy assertion currently applies to Linux and Windows backends only
        if !cfg!(any(target_os = "linux", windows)) {
            return Ok(());
        }

        // reject access-group lanes where backend key stores cannot represent group namespaces
        let with_access_group = credential_write_options_value(
            &mut context,
            "destack.policy",
            "access-group",
            Some("group.identifier"),
            b"payload",
            CredentialAccessibility::HostDefault,
            CredentialAuthenticationPolicy::None,
            true,
        )?;
        let access_group_result = context.destack_os_credentials_write(with_access_group);
        let access_group_error = match access_group_result {
            Ok(_) => panic!("write with accessGroup should report notSupported"),
            Err(error) => error,
        };
        assert_not_supported_error(&access_group_error);

        // reject authentication-policy lanes where backend key stores cannot enforce prompts
        let with_authentication = credential_write_options_value(
            &mut context,
            "destack.policy",
            "authentication",
            None,
            b"payload",
            CredentialAccessibility::HostDefault,
            CredentialAuthenticationPolicy::UserPresence,
            true,
        )?;
        let authentication_result = context.destack_os_credentials_write(with_authentication);
        let authentication_error = match authentication_result {
            Ok(_) => panic!("write with authentication policy should report notSupported"),
            Err(error) => error,
        };
        assert_not_supported_error(&authentication_error);

        // reject accessibility lanes where backend key stores cannot enforce lifecycle classes
        if cfg!(target_os = "linux") {
            let with_accessibility = credential_write_options_value(
                &mut context,
                "destack.policy",
                "accessibility",
                None,
                b"payload",
                CredentialAccessibility::WhenUnlocked,
                CredentialAuthenticationPolicy::None,
                true,
            )?;
            let accessibility_result = context.destack_os_credentials_write(with_accessibility);
            let accessibility_error = match accessibility_result {
                Ok(_) => panic!("write with non-default accessibility should report notSupported"),
                Err(error) => error,
            };
            assert_not_supported_error(&accessibility_error);
        }

        Ok(())
    });
}

/// Verify unsupported read-policy lanes report notSupported on Linux.
#[cfg(target_os = "linux")]
#[test]
fn test_credentials_read_policy_lanes_on_linux() {
    with_harness_context(|mut context| {
        // reject interactive-auth read lanes where backend key stores have no challenge API
        let query = credential_query_value(&mut context, "destack.policy", "read-auth", None, true);
        let result = context.destack_os_credentials_read(query);
        let error = match result {
            Ok(_) => panic!("read with requireAuthentication should report notSupported"),
            Err(error) => error,
        };
        assert_not_supported_error(&error);

        Ok(())
    });
}

/// Verify unsupported contains and delete access-group lanes report notSupported on Linux and Windows.
#[cfg(any(unix, windows))]
#[test]
fn test_credentials_delete_and_contains_access_group_policy_on_linux_and_windows() {
    with_harness_context(|mut context| {
        // this policy assertion currently applies to Linux and Windows backends only
        if !cfg!(any(target_os = "linux", windows)) {
            return Ok(());
        }

        // reject access-group lanes for contains on backends without group routing
        let service = string_value(&mut context, "destack.policy");
        let account = string_value(&mut context, "contains-access-group");
        let access_group = optional_string_value(&mut context, Some("group.identifier"));
        let contains_result =
            context.destack_os_credentials_contains(service, account, access_group);
        let contains_error = match contains_result {
            Ok(_) => panic!("contains with accessGroup should report notSupported"),
            Err(error) => error,
        };
        assert_not_supported_error(&contains_error);

        // reject access-group lanes for delete on backends without group routing
        let service = string_value(&mut context, "destack.policy");
        let account = string_value(&mut context, "delete-access-group");
        let access_group = optional_string_value(&mut context, Some("group.identifier"));
        let delete_result = context.destack_os_credentials_delete(service, account, access_group);
        let delete_error = match delete_result {
            Ok(_) => panic!("delete with accessGroup should report notSupported"),
            Err(error) => error,
        };
        assert_not_supported_error(&delete_error);

        Ok(())
    });
}
