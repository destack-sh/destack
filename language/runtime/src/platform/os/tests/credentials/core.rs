use std::time::{SystemTime, UNIX_EPOCH};

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::VmSlice;
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationOptions, CredentialAuthenticationOptionsVm,
    CredentialAuthenticationPolicy, CredentialAuthenticationRequirement, CredentialQuery,
    CredentialQueryVm, CredentialRecord, CredentialRecordVm, CredentialWriteOptions,
    CredentialWriteOptionsVm,
};
use crate::runtime::NativeStringRef;
pub(super) use crate::tests::platform::{
    assert_not_not_supported_error, assert_not_supported_error, assert_runtime_error_code,
};

use super::super::{HarnessValue, OsHarnessContext};

/// Return whether this target supports credentials roundtrip in harness runs.
pub(super) fn credentials_roundtrip_supported_on_target() -> bool {
    cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        windows
    ))
}

/// Return one raw VM context pointer when this harness run uses vm bindings.
fn vm_context_pointer(context: &OsHarnessContext<'_>) -> Option<*mut ()> {
    context.vm_context
}

/// Build one deterministic unique suffix for key names.
pub(super) fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos()
}

/// Build one harness string value for native and vm calls.
pub(super) fn string_value(
    context: &mut OsHarnessContext<'_>,
    text: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    // build one vm string value for vm runs
    if let Some(vm_context) = vm_context_pointer(context) {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        return HarnessValue::Vm(vm::StringHandle::new(vm_context.intern_string(text)));
    }

    // build one native string value for native runs
    HarnessValue::Native(context.call_context.store_string(text))
}

/// Build one harness write-options payload for native and vm calls.
pub(super) fn credential_write_options_value(
    context: &mut OsHarnessContext<'_>,
    service: &str,
    account: &str,
    access_group: &str,
    bytes: &[u8],
    accessibility: CredentialAccessibility,
    authentication: CredentialAuthenticationPolicy,
    replace_existing: bool,
) -> RuntimeResult<HarnessValue<CredentialWriteOptions, CredentialWriteOptionsVm>> {
    // build one vm write-options payload for vm runs
    if let Some(vm_context) = vm_context_pointer(context) {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        let options = CredentialWriteOptionsVm {
            service: vm::StringHandle::new(vm_context.intern_string(service)),
            account: vm::StringHandle::new(vm_context.intern_string(account)),
            access_group: vm::StringHandle::new(vm_context.intern_string(access_group)),
            bytes: VmSlice::from_bytes(vm_context, bytes),
            accessibility,
            authentication,
            replace_existing,
        };

        return Ok(HarnessValue::Vm(options));
    }

    // build one native write-options payload for native runs
    let options = CredentialWriteOptions {
        service: context.call_context.store_string(service),
        account: context.call_context.store_string(account),
        access_group: context.call_context.store_string(access_group),
        bytes: context.call_context.store_slice(bytes.to_vec()),
        accessibility,
        authentication,
        replace_existing,
    };

    Ok(HarnessValue::Native(options))
}

/// Build one harness query payload for native and vm calls.
pub(super) fn credential_query_value(
    context: &mut OsHarnessContext<'_>,
    service: &str,
    account: &str,
    access_group: &str,
    require_authentication: bool,
) -> HarnessValue<CredentialQuery, CredentialQueryVm> {
    // build one vm query payload for vm runs
    if let Some(vm_context) = vm_context_pointer(context) {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        let query = CredentialQueryVm {
            service: vm::StringHandle::new(vm_context.intern_string(service)),
            account: vm::StringHandle::new(vm_context.intern_string(account)),
            access_group: vm::StringHandle::new(vm_context.intern_string(access_group)),
            require_authentication,
        };

        return HarnessValue::Vm(query);
    }

    // build one native query payload for native runs
    let query = CredentialQuery {
        service: context.call_context.store_string(service),
        account: context.call_context.store_string(account),
        access_group: context.call_context.store_string(access_group),
        require_authentication,
    };

    HarnessValue::Native(query)
}

/// Build one harness authentication-options payload for native and vm calls.
pub(super) fn credential_authentication_options_value(
    context: &mut OsHarnessContext<'_>,
    title: &str,
    subtitle: &str,
    message: &str,
    requirement: CredentialAuthenticationRequirement,
) -> HarnessValue<CredentialAuthenticationOptions, CredentialAuthenticationOptionsVm> {
    // build one vm authentication-options payload for vm runs
    if let Some(vm_context) = vm_context_pointer(context) {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
        let options = CredentialAuthenticationOptionsVm {
            title: vm::StringHandle::new(vm_context.intern_string(title)),
            subtitle: vm::StringHandle::new(vm_context.intern_string(subtitle)),
            message: vm::StringHandle::new(vm_context.intern_string(message)),
            requirement,
        };

        return HarnessValue::Vm(options);
    }

    // build one native authentication-options payload for native runs
    let options = CredentialAuthenticationOptions {
        title: context.call_context.store_string(title),
        subtitle: context.call_context.store_string(subtitle),
        message: context.call_context.store_string(message),
        requirement,
    };

    HarnessValue::Native(options)
}

/// Decode one harness credential-record payload into owned fields.
pub(super) fn decode_credential_record_value(
    context: &mut OsHarnessContext<'_>,
    record: HarnessValue<CredentialRecord, CredentialRecordVm>,
) -> RuntimeResult<(String, String, Vec<u8>)> {
    // decode the variant that matches this harness mode
    match record {
        HarnessValue::Native(value) => {
            let service = unsafe { value.service.as_str() }?.to_string();
            let account = unsafe { value.account.as_str() }?.to_string();
            let bytes = unsafe { value.bytes.as_slice() }?.to_vec();

            Ok((service, account, bytes))
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_pointer(context).expect("vm context should be available");
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let service = vm_context
                .string_ref(value.service)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let account = vm_context
                .string_ref(value.account)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let bytes = value.bytes.read_bytes(vm_context)?.to_vec();

            Ok((service, account, bytes))
        }
    }
}
