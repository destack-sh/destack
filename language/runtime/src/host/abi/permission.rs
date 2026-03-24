use std::ptr;

use crate::platform::os::Permission;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// One host permission request payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostPermissionRequest {
    /// The stable request identifier for this interactive host flow.
    pub request_id: u64,
    /// The normalized permission names requested by the runtime.
    pub permissions: NativeStringSlice,
}

/// One owned host permission request payload.
#[derive(Debug)]
pub(crate) struct HostPermissionRequestPayload {
    /// The owned permission-name backing storage.
    permission_storage: Vec<String>,
    /// The borrowed permission-name refs.
    permission_refs: Vec<NativeStringRef>,
    /// The borrowed ABI request view.
    abi: HostPermissionRequest,
}

impl HostPermissionRequestPayload {
    /// Build one owned single-permission request payload.
    pub(crate) fn single(request_id: u64, permission: Permission) -> Self {
        Self::many(request_id, &[permission])
    }

    /// Build one owned permission request payload.
    pub(crate) fn many(request_id: u64, permissions: &[Permission]) -> Self {
        let permission_storage = permissions
            .iter()
            .copied()
            .map(permission_name)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let permission_refs = permission_storage
            .iter()
            .map(NativeStringRef::from)
            .collect::<Vec<_>>();
        let abi = HostPermissionRequest {
            request_id,
            permissions: string_slice(&permission_refs),
        };

        Self {
            permission_storage,
            permission_refs,
            abi,
        }
    }

    /// Return the ABI request view.
    pub(crate) fn abi(&self) -> HostPermissionRequest {
        let _ = &self.permission_storage;
        let _ = &self.permission_refs;

        self.abi
    }
}

/// Return the canonical host permission name.
pub(crate) const fn permission_name(permission: Permission) -> &'static str {
    match permission {
        Permission::Location => "location",
        Permission::LocationBackground => "locationBackground",
        Permission::Camera => "camera",
        Permission::Microphone => "microphone",
        Permission::Bluetooth => "bluetooth",
        Permission::Notifications => "notifications",
        Permission::ContactsRead => "contactsRead",
        Permission::ContactsWrite => "contactsWrite",
        Permission::MediaRead => "mediaRead",
        Permission::MediaWrite => "mediaWrite",
        Permission::Motion => "motion",
        Permission::ClipboardRead => "clipboardRead",
        Permission::CalendarRead => "calendarRead",
        Permission::CalendarWrite => "calendarWrite",
    }
}

/// Build one string-slice view.
fn string_slice(values: &[NativeStringRef]) -> NativeStringSlice {
    let data = if values.is_empty() {
        ptr::null()
    } else {
        values.as_ptr()
    };

    NativeStringSlice {
        data,
        len: values.len() as u32,
    }
}
