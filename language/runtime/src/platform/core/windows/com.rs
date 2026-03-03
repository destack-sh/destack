use std::ffi::c_void;
use std::panic::UnwindSafe;
use std::ptr::NonNull;

use windows_sys::core::GUID;

/// One COM `IUnknown` interface identifier.
pub(crate) const COM_IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);

/// Return whether two COM interface identifiers are byte-for-byte equal.
pub(crate) fn com_guid_equals(left: &GUID, right: &GUID) -> bool {
    left.data1 == right.data1
        && left.data2 == right.data2
        && left.data3 == right.data3
        && left.data4 == right.data4
}

/// Release one COM pointer by one release callback.
pub(crate) unsafe fn com_release_with(
    pointer: *mut c_void,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
) {
    // ignore null pointers
    if pointer.is_null() {
        return;
    }

    // release one COM reference
    unsafe {
        release(pointer);
    }
}

/// Return one typed non-null COM pointer from one raw callback pointer.
pub(crate) unsafe fn com_non_null_from_raw<T>(
    pointer: *mut c_void,
    operation: &'static str,
) -> NonNull<T> {
    // validate callback object pointer in debug builds
    debug_assert!(
        !pointer.is_null(),
        "{operation} received one null callback object pointer",
    );

    // cast one raw callback pointer into one typed non-null pointer
    unsafe { NonNull::new_unchecked(pointer.cast::<T>()) }
}

/// Run one callback body inside one panic boundary.
pub(crate) fn callback_boundary(callback: impl FnOnce() + UnwindSafe) {
    // isolate callback panics from FFI call sites
    let _ = std::panic::catch_unwind(callback);
}

/// Define one COM callback `IUnknown` implementation for one runtime-owned callback object.
macro_rules! define_com_iunknown_methods {
    (
        object = $object:ident,
        from_raw = $from_raw:ident,
        query_interface = $query_interface:ident,
        add_ref = $add_ref:ident,
        release = $release:ident,
        interfaces = [$($interface:expr),+ $(,)?]
    ) => {
        /// Return one callback object pointer cast from one COM instance pointer.
        unsafe fn $from_raw(this: *mut std::ffi::c_void) -> std::ptr::NonNull<$object> {
            // cast one raw callback pointer
            unsafe {
                $crate::platform::core::com_non_null_from_raw::<$object>(this, stringify!($from_raw))
            }
        }

        /// Query one callback interface from one COM callback object.
        unsafe extern "system" fn $query_interface(
            this: *mut std::ffi::c_void,
            interface_id: *const windows_sys::core::GUID,
            out_interface: *mut *mut std::ffi::c_void,
        ) -> windows_sys::core::HRESULT {
            // reject invalid output pointer
            if out_interface.is_null() {
                return windows_sys::Win32::Foundation::E_POINTER;
            }

            // initialize output to null before probing
            unsafe {
                *out_interface = std::ptr::null_mut();
            }

            // reject invalid callback object pointer
            if this.is_null() {
                return windows_sys::Win32::Foundation::E_POINTER;
            }

            // reject invalid interface-id pointer
            if interface_id.is_null() {
                return windows_sys::Win32::Foundation::E_NOINTERFACE;
            }

            // copy one interface id for stable comparisons
            let interface_id = unsafe { *interface_id };

            // check IUnknown plus one module-specific interface set
            let supported_interface =
                $crate::platform::core::com_guid_equals(
                    &interface_id,
                    &$crate::platform::core::COM_IID_IUNKNOWN,
                )
                $(|| $crate::platform::core::com_guid_equals(&interface_id, &$interface))*;

            // return one reference to the same callback object when supported
            if supported_interface {
                unsafe {
                    *out_interface = this;
                    $add_ref(this);
                }

                return windows_sys::Win32::Foundation::S_OK;
            }

            // reject unsupported interface ids
            windows_sys::Win32::Foundation::E_NOINTERFACE
        }

        /// Increment one COM callback reference count.
        unsafe extern "system" fn $add_ref(this: *mut std::ffi::c_void) -> u32 {
            // reject invalid callback object pointers
            if this.is_null() {
                return 0;
            }

            // cast the callback pointer
            let callback = unsafe { $from_raw(this) };
            let callback = callback.as_ptr();

            // increment and return the updated count
            let next_count = unsafe {
                (*callback)
                    .reference_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1
            };

            next_count
        }

        /// Decrement one COM callback reference count and free on zero.
        unsafe extern "system" fn $release(this: *mut std::ffi::c_void) -> u32 {
            // reject invalid callback object pointers
            if this.is_null() {
                return 0;
            }

            // cast the callback pointer
            let callback = unsafe { $from_raw(this) };
            let callback = callback.as_ptr();

            // decrement and compute the remaining count
            let remaining = unsafe {
                (*callback)
                    .reference_count
                    .fetch_sub(1, std::sync::atomic::Ordering::Release)
                    - 1
            };

            // free callback memory when the final reference drops
            if remaining == 0 {
                std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);

                unsafe {
                    drop(Box::from_raw(callback));
                }
            }

            // return one remaining reference count
            remaining
        }
    };
}

/// Define one COM callback vtable with shared `IUnknown` lanes.
macro_rules! define_com_callback_vtable {
    (
        static = $static_name:ident,
        type = $vtable_type:ty,
        value = $vtable_value:ident,
        query_interface = $query_interface:ident,
        add_ref = $add_ref:ident,
        release = $release:ident,
        methods = { $($method_field:ident = $method_value:ident),+ $(,)? }
    ) => {
        static $static_name: $vtable_type = $vtable_value {
            query_interface: $query_interface,
            add_ref: $add_ref,
            release: $release,
            $($method_field: $method_value,)+
        };
    };
}

pub(crate) use {define_com_callback_vtable, define_com_iunknown_methods};
