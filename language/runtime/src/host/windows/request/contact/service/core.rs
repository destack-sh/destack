use std::sync::Arc;

use windows::ApplicationModel::Contacts::{Contact, ContactStoreAccessType};
use windows::core::Error as WindowsError;

use super::draft::apply_contact_draft;
use super::native::contact_value_from_native;
use super::page::paginate_contacts;
use super::store::{
    default_contact_list, full_contact_query, get_contact, read_contacts, request_contact_store,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::service::registry::global_service;
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy};

/// One process-global Windows contact service.
pub(crate) struct WindowsContactService {
    /// Dedicated WinRT executor for contact store work.
    executor: ServiceThreadExecutor<()>,
}

impl WindowsContactService {
    /// The execution policy for the Windows contact service.
    pub(crate) const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Thread)
        .with_affinity(ExecutionAffinity::WindowsMta);

    /// List contacts through the Windows contact store.
    pub(crate) fn list_contacts(
        &self,
        query: &ContactQueryValue,
        operation: &'static str,
    ) -> RuntimeResult<ContactPageValue> {
        let query = query.clone();

        self.executor.call(operation, move |_state| {
            let store =
                request_contact_store(ContactStoreAccessType::AllContactsReadOnly, operation)?;
            let native_contacts = read_contacts(&store, None, &query, operation)?;

            paginate_contacts(native_contacts, &query)
        })
    }

    /// Search contacts through the Windows contact store.
    pub(crate) fn search_contacts(
        &self,
        query_text: &str,
        query: &ContactQueryValue,
        operation: &'static str,
    ) -> RuntimeResult<ContactPageValue> {
        let query_text = query_text.to_string();
        let query = query.clone();

        self.executor.call(operation, move |_state| {
            let store =
                request_contact_store(ContactStoreAccessType::AllContactsReadOnly, operation)?;
            let native_contacts = read_contacts(&store, Some(&query_text), &query, operation)?;

            paginate_contacts(native_contacts, &query)
        })
    }

    /// Read one contact by identifier through the Windows contact store.
    pub(crate) fn read_contact(
        &self,
        id: &str,
        operation: &'static str,
    ) -> RuntimeResult<ContactValue> {
        let id = id.to_string();

        self.executor.call(operation, move |_state| {
            let store =
                request_contact_store(ContactStoreAccessType::AllContactsReadOnly, operation)?;
            let native_contact = get_contact(&store, &id, operation)?;
            let query = full_contact_query();

            contact_value_from_native(&native_contact, &query, operation)
        })
    }

    /// Create one contact through the Windows contact store.
    pub(crate) fn create_contact(
        &self,
        draft: &ContactDraftValue,
        operation: &'static str,
    ) -> RuntimeResult<String> {
        let draft = draft.clone();

        self.executor.call(operation, move |_state| {
            let store =
                request_contact_store(ContactStoreAccessType::AllContactsReadWrite, operation)?;
            let contact_list = default_contact_list(&store, operation)?;
            let native_contact = Contact::new()
                .map_err(|error| windows_contact_error(operation, "Contact::new", &error))?;

            // apply the runtime draft before saving
            apply_contact_draft(&native_contact, &draft, operation)?;

            contact_list
                .SaveContactAsync(&native_contact)
                .map_err(|error| {
                    windows_contact_error(operation, "ContactList::SaveContactAsync", &error)
                })?
                .get()
                .map_err(|error| windows_contact_error(operation, "IAsyncAction::get", &error))?;

            let id = native_contact
                .Id()
                .map_err(|error| windows_contact_error(operation, "Contact::Id", &error))?
                .to_string();

            if id.is_empty() {
                return Err(io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    "windows contact store created one contact without one stable identifier",
                ));
            }

            Ok(id)
        })
    }

    /// Update one contact through the Windows contact store.
    pub(crate) fn update_contact(
        &self,
        id: &str,
        draft: &ContactDraftValue,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        let id = id.to_string();
        let draft = draft.clone();

        self.executor.call(operation, move |_state| {
            let store =
                request_contact_store(ContactStoreAccessType::AllContactsReadWrite, operation)?;
            let native_contact = get_contact(&store, &id, operation)?;
            let contact_list =
                super::store::contact_list_for_contact(&store, &native_contact, operation)?;

            // apply the runtime draft before saving
            apply_contact_draft(&native_contact, &draft, operation)?;

            contact_list
                .SaveContactAsync(&native_contact)
                .map_err(|error| {
                    windows_contact_error(operation, "ContactList::SaveContactAsync", &error)
                })?
                .get()
                .map_err(|error| windows_contact_error(operation, "IAsyncAction::get", &error))?;

            Ok(())
        })
    }

    /// Delete one contact through the Windows contact store.
    pub(crate) fn delete_contact(&self, id: &str, operation: &'static str) -> RuntimeResult<()> {
        let id = id.to_string();

        self.executor.call(operation, move |_state| {
            let store =
                request_contact_store(ContactStoreAccessType::AllContactsReadWrite, operation)?;
            let native_contact = get_contact(&store, &id, operation)?;
            let contact_list =
                super::store::contact_list_for_contact(&store, &native_contact, operation)?;

            contact_list
                .DeleteContactAsync(&native_contact)
                .map_err(|error| {
                    windows_contact_error(operation, "ContactList::DeleteContactAsync", &error)
                })?
                .get()
                .map_err(|error| windows_contact_error(operation, "IAsyncAction::get", &error))?;

            Ok(())
        })
    }
}

/// Return the shared Windows contact service.
pub(crate) fn windows_contact_service(
    _operation: &'static str,
) -> RuntimeResult<Arc<WindowsContactService>> {
    global_service(|| {
        let executor = ServiceThreadExecutor::spawn(
            "destack-windows-contact",
            WindowsContactService::POLICY,
            || Ok(()),
        )?;

        Ok(WindowsContactService { executor })
    })
}

/// Map one WinRT contact error into one runtime IO error.
pub(super) fn windows_contact_error(
    operation: &'static str,
    stage: &str,
    error: &WindowsError,
) -> Box<RuntimeError> {
    match error.code().0 as u32 {
        0x80070490 => io_not_found(operation, "windows contact was not found"),
        0x80070005 => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "windows contact access was denied",
        ),
        0x80004005 => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoWouldBlock),
            "windows contact operation could not complete yet",
        ),
        _ => io_operation_error(operation, None, format!("{stage} failed: {error}")),
    }
}
