use vobject::Vcard;
use zbus::blocking::Proxy;

use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request::linux::eds::open_address_book_backend;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;

/// The EDS address-book interface name.
const ADDRESS_BOOK_INTERFACE: &str = "org.gnome.evolution.dataserver.AddressBook";

/// One default EDS operation-flags value.
const EDS_OPERATION_FLAGS: u32 = 0;

/// Read one contact list payload from one source address book.
pub(super) fn get_contact_list(
    source_uid: &str,
    query: &str,
    operation: &'static str,
) -> RuntimeResult<Vec<Vcard>> {
    with_address_book_proxy(source_uid, operation, |proxy| {
        let vcards: Vec<String> = proxy.call("GetContactList", &(query,)).map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("evolution address book list failed: {error}"),
            )
        })?;
        let mut contacts = Vec::with_capacity(vcards.len());

        // vcard parsing
        for vcard in vcards {
            let contact = Vcard::build(&vcard).map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution address book returned one invalid vcard: {error}"),
                )
            })?;

            contacts.push(contact);
        }

        Ok(contacts)
    })
}

/// Read one contact vcard from one source address book.
pub(super) fn get_contact(
    source_uid: &str,
    contact_uid: &str,
    operation: &'static str,
) -> RuntimeResult<Vcard> {
    with_address_book_proxy(source_uid, operation, |proxy| {
        let vcard: String = proxy.call("GetContact", &(contact_uid,)).map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("evolution address book read failed: {error}"),
            )
        })?;

        Vcard::build(&vcard).map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("evolution address book returned one invalid vcard: {error}"),
            )
        })
    })
}

/// Create one or more contacts in one source address book.
pub(super) fn create_contacts(
    source_uid: &str,
    vcards: &[String],
    operation: &'static str,
) -> RuntimeResult<Vec<String>> {
    with_address_book_proxy(source_uid, operation, |proxy| {
        proxy
            .call("CreateContacts", &(vcards.to_vec(), EDS_OPERATION_FLAGS))
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution address book create failed: {error}"),
                )
            })
    })
}

/// Modify one or more contacts in one source address book.
pub(super) fn modify_contacts(
    source_uid: &str,
    vcards: &[String],
    operation: &'static str,
) -> RuntimeResult<()> {
    with_address_book_proxy(source_uid, operation, |proxy| {
        proxy
            .call::<_, _, ()>("ModifyContacts", &(vcards.to_vec(), EDS_OPERATION_FLAGS))
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution address book update failed: {error}"),
                )
            })
    })
}

/// Remove one or more contacts from one source address book.
pub(super) fn remove_contacts(
    source_uid: &str,
    ids: &[String],
    operation: &'static str,
) -> RuntimeResult<()> {
    with_address_book_proxy(source_uid, operation, |proxy| {
        proxy
            .call::<_, _, ()>("RemoveContacts", &(ids.to_vec(), EDS_OPERATION_FLAGS))
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution address book delete failed: {error}"),
                )
            })
    })
}

/// Bind one address-book proxy for one operation scope.
fn with_address_book_proxy<T>(
    source_uid: &str,
    operation: &'static str,
    callback: impl FnOnce(&Proxy<'_>) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let backend = open_address_book_backend(source_uid, operation)?;
    let proxy = Proxy::new(
        &backend.connection,
        backend.bus_name.as_str(),
        backend.object_path.as_str(),
        ADDRESS_BOOK_INTERFACE,
    )
    .map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to bind one evolution address book proxy: {error}"),
        )
    })?;
    let result = callback(&proxy);

    // backend cleanup
    if let Err(error) = proxy.call::<_, _, ()>("Close", &()) {
        tracing::warn!(
            target: "destack.runtime.host.unix.contact",
            ?error,
            "failed to close evolution address book backend",
        );
    }

    result
}
