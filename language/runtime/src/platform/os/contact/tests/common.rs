use std::sync::{Mutex, OnceLock};

use destack_vm::{BindingContext, StringHandle};
use destack_workspace::{AppPermission, RuntimeOptions};

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "linux")]
use crate::host::os::linux::tests::{
    LinuxContactHooks as DesktopContactHooks, set_linux_contact_test_hooks,
};
#[cfg(target_os = "macos")]
use crate::host::os::macos::request::contact::{
    MacosContactHooks as DesktopContactHooks, set_macos_contact_test_hooks,
};
#[cfg(windows)]
use crate::host::os::windows::request::contact::{
    WindowsContactHooks as DesktopContactHooks, set_windows_contact_test_hooks,
};
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    Contact, ContactAddressValue, ContactDraft, ContactDraftVm, ContactEmailValue,
    ContactNameValue, ContactOrganizationValue, ContactPage, ContactPageVm, ContactPhoneValue,
    ContactQuery, ContactQueryVm, ContactVm,
};
use crate::platform::{NativeAbiCodec, VmAbiCodec};

/// Shared desktop contact test state.
#[derive(Default)]
pub(super) struct DesktopContactTestState {
    /// Recorded list queries.
    pub(super) list_queries: Vec<ContactQueryValue>,
    /// Recorded search queries.
    pub(super) search_queries: Vec<(String, ContactQueryValue)>,
    /// Recorded contact reads.
    pub(super) read_ids: Vec<String>,
    /// Recorded created drafts.
    pub(super) created_contacts: Vec<ContactDraftValue>,
    /// Recorded updates.
    pub(super) updated_contacts: Vec<(String, ContactDraftValue)>,
    /// Recorded deletions.
    pub(super) deleted_ids: Vec<String>,
}

/// Return the shared desktop contact test state slot.
pub(super) fn desktop_contact_test_state() -> &'static Mutex<DesktopContactTestState> {
    static STATE: OnceLock<Mutex<DesktopContactTestState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(DesktopContactTestState::default()))
}

/// Install contact permissions for desktop host tests.
pub(super) fn enable_contact_declarations(options: &mut RuntimeOptions) {
    options
        .app
        .permissions
        .insert(AppPermission::ContactsRead, Default::default());
    options
        .app
        .permissions
        .insert(AppPermission::ContactsWrite, Default::default());
}

/// Install one scoped desktop contact hook set.
pub(super) fn with_desktop_contact_test_hooks<T>(callback: impl FnOnce() -> T) -> T {
    {
        let mut state = desktop_contact_test_state()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        *state = DesktopContactTestState::default();
    }

    let hooks = desktop_contact_test_hooks();
    set_desktop_contact_test_hooks(hooks);
    let guard = DesktopContactHookGuard;
    let result = callback();

    drop(guard);

    result
}

/// Run one desktop contact harness pass with hooks and declarations installed.
pub(super) fn with_desktop_contact_context<T>(
    mut callback: impl for<'call> FnMut(HarnessContext<'call>) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut result = None;

    with_desktop_contact_test_hooks(|| {
        with_configured_harness_context(enable_contact_declarations, |context| {
            result = Some(callback(context));
            Ok(())
        });
    });

    result.expect("desktop contact harness should capture one result")
}

/// Return the desktop contact hook set for the active target.
fn desktop_contact_test_hooks() -> DesktopContactHooks {
    DesktopContactHooks {
        list: Some(test_list_contacts),
        search: Some(test_search_contacts),
        read: Some(test_read_contact),
        create: Some(test_create_contact),
        update: Some(test_update_contact),
        delete: Some(test_delete_contact),
    }
}

/// Install the active target contact hook set.
fn set_desktop_contact_test_hooks(hooks: DesktopContactHooks) {
    #[cfg(target_os = "linux")]
    set_linux_contact_test_hooks(hooks);

    #[cfg(target_os = "macos")]
    set_macos_contact_test_hooks(hooks);

    #[cfg(windows)]
    set_windows_contact_test_hooks(hooks);
}

/// One scoped desktop contact hook installation.
struct DesktopContactHookGuard;

impl Drop for DesktopContactHookGuard {
    /// Clear the active desktop contact hooks after one test.
    fn drop(&mut self) {
        set_desktop_contact_test_hooks(DesktopContactHooks::default());
    }
}

/// Return one deterministic desktop contact page and record the list query.
fn test_list_contacts(query: ContactQueryValue) -> RuntimeResult<ContactPageValue> {
    let mut state = desktop_contact_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.list_queries.push(query);

    Ok(sample_contact_page())
}

/// Return one deterministic desktop contact page and record the search query.
fn test_search_contacts(
    query_text: String,
    query: ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let mut state = desktop_contact_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.search_queries.push((query_text, query));

    Ok(sample_contact_page())
}

/// Return one deterministic contact and record the read id.
fn test_read_contact(id: String) -> RuntimeResult<ContactValue> {
    let mut state = desktop_contact_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.read_ids.push(id);

    Ok(sample_contact())
}

/// Return one deterministic created id and record the draft.
fn test_create_contact(contact: ContactDraftValue) -> RuntimeResult<String> {
    let mut state = desktop_contact_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.created_contacts.push(contact);

    Ok("contact-created".to_string())
}

/// Record one contact update.
fn test_update_contact(id: String, contact: ContactDraftValue) -> RuntimeResult<()> {
    let mut state = desktop_contact_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.updated_contacts.push((id, contact));

    Ok(())
}

/// Record one contact deletion.
fn test_delete_contact(id: String) -> RuntimeResult<()> {
    let mut state = desktop_contact_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.deleted_ids.push(id);

    Ok(())
}

/// Build one deterministic contact page fixture.
pub(super) fn sample_contact_page() -> ContactPageValue {
    ContactPageValue {
        contacts: vec![sample_contact()],
        next_cursor: String::new(),
        has_more: false,
    }
}

/// Build one deterministic contact fixture.
pub(super) fn sample_contact() -> ContactValue {
    ContactValue {
        id: "contact-1".to_string(),
        name: ContactNameValue {
            given_name: "Ada".to_string(),
            middle_name: String::new(),
            family_name: "Lovelace".to_string(),
            prefix: String::new(),
            suffix: String::new(),
            nickname: "Ada".to_string(),
            phonetic_given_name: String::new(),
            phonetic_family_name: String::new(),
        },
        phones: vec![ContactPhoneValue {
            label: "mobile".to_string(),
            number: "+41 44 555 12 34".to_string(),
            normalized_number: "+41445551234".to_string(),
            primary: true,
        }],
        emails: vec![ContactEmailValue {
            label: "work".to_string(),
            address: "ada@example.com".to_string(),
            primary: true,
        }],
        addresses: vec![ContactAddressValue {
            label: "home".to_string(),
            street: "Analytical Engine Way 1".to_string(),
            city: "Zurich".to_string(),
            region: "ZH".to_string(),
            postal_code: "8000".to_string(),
            country: "Switzerland".to_string(),
            country_code: "CH".to_string(),
        }],
        organization: ContactOrganizationValue {
            company: "Symbol".to_string(),
            department: "Research".to_string(),
            title: "Engineer".to_string(),
        },
        note: "First programmer".to_string(),
    }
}

/// Build one deterministic contact draft fixture.
pub(super) fn sample_contact_draft(given_name: &str) -> ContactDraftValue {
    ContactDraftValue {
        name: ContactNameValue {
            given_name: given_name.to_string(),
            middle_name: String::new(),
            family_name: "Example".to_string(),
            prefix: String::new(),
            suffix: String::new(),
            nickname: given_name.to_string(),
            phonetic_given_name: String::new(),
            phonetic_family_name: String::new(),
        },
        phones: vec![ContactPhoneValue {
            label: "mobile".to_string(),
            number: "+41 44 555 12 34".to_string(),
            normalized_number: "+41445551234".to_string(),
            primary: true,
        }],
        emails: vec![ContactEmailValue {
            label: "work".to_string(),
            address: "person@example.com".to_string(),
            primary: true,
        }],
        addresses: vec![ContactAddressValue {
            label: "office".to_string(),
            street: "Example Street 2".to_string(),
            city: "Zurich".to_string(),
            region: "ZH".to_string(),
            postal_code: "8001".to_string(),
            country: "Switzerland".to_string(),
            country_code: "CH".to_string(),
        }],
        organization: ContactOrganizationValue {
            company: "Symbol".to_string(),
            department: "Runtime".to_string(),
            title: "Developer".to_string(),
        },
        note: "Desktop contact test".to_string(),
    }
}

/// Build one deterministic contact query fixture.
pub(super) fn sample_contact_query() -> ContactQueryValue {
    ContactQueryValue {
        cursor: None,
        limit: Some(10),
        include_phones: true,
        include_emails: true,
        include_addresses: true,
        include_organization: true,
        include_notes: true,
    }
}

/// Build one contact query payload for the active harness.
pub(super) fn contact_query_harness_value(
    context: &mut HarnessContext<'_>,
    query: ContactQueryValue,
) -> RuntimeResult<HarnessValue<ContactQuery, ContactQueryVm>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let query = ContactQueryVm::from_value(&mut vm_context.write(), query)?;

            Ok(HarnessValue::Vm(query))
        }
        None => Ok(HarnessValue::Native(ContactQuery::from_value(
            context.call_context,
            query,
        ))),
    }
}

/// Build one contact draft payload for the active harness.
pub(super) fn contact_draft_harness_value(
    context: &mut HarnessContext<'_>,
    draft: ContactDraftValue,
) -> HarnessValue<ContactDraft, ContactDraftVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let draft = ContactDraftVm::from_value(&mut vm_context.write(), draft)
                .expect("vm contact draft should encode");

            HarnessValue::Vm(draft)
        }
        None => HarnessValue::Native(ContactDraft::from_value(context.call_context, draft)),
    }
}

/// Build one string payload for the active harness.
pub(super) fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let value = StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm contact test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Decode one string payload into one owned string.
pub(super) fn decode_string_value(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeStringRef, StringHandle>,
) -> RuntimeResult<String> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_str() }?.to_string()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };

            vm_context
                .string_value(value.value())
                .map_err(|error| RuntimeError::from(error).boxed())
        }
    }
}

/// Decode one contact page into one owned value.
pub(super) fn decode_contact_page(
    context: &mut HarnessContext<'_>,
    page: HarnessValue<ContactPage, ContactPageVm>,
) -> RuntimeResult<ContactPageValue> {
    match page {
        HarnessValue::Native(page) => unsafe { ContactPage::into_value(page) },
        HarnessValue::Vm(page) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };

            ContactPageVm::into_value(page, &vm_context.read())
        }
    }
}

/// Decode one contact into one owned value.
pub(super) fn decode_contact(
    context: &mut HarnessContext<'_>,
    contact: HarnessValue<Contact, ContactVm>,
) -> RuntimeResult<ContactValue> {
    match contact {
        HarnessValue::Native(contact) => unsafe { Contact::into_value(contact) },
        HarnessValue::Vm(contact) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };

            ContactVm::into_value(contact, &vm_context.read())
        }
    }
}
