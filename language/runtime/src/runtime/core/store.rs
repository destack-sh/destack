use std::any::Any;
use std::cell::RefCell;
use std::ptr;

use crate::platform::{NativeArray, NativeSlice, NativeStringRef, NativeStringSlice};

/// Per-call storage for native ABI references returned by bindings.
///
/// Stored pointers are valid until the next runtime call on the same thread.
#[derive(Debug, Default)]
pub struct BindingCallArena {
    /// Owned strings backing native string references.
    strings: RefCell<Vec<Box<str>>>,
    /// Owned slices backing native slice references.
    values: RefCell<Vec<Box<dyn Any>>>,
}

impl BindingCallArena {
    /// Clear all stored references.
    pub fn clear(&self) {
        self.strings.borrow_mut().clear();
        self.values.borrow_mut().clear();
    }

    /// Store a string and return a native string reference.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        let mut strings = self.strings.borrow_mut();
        strings.push(value.to_owned().into_boxed_str());

        let stored = strings.last().expect("stored string must be available");
        NativeStringRef::from(stored.as_ref())
    }

    /// Store an optional string and return a native string reference.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        match value {
            Some(value) => self.store_string(value),
            None => NativeStringRef {
                data: ptr::null(),
                len: 0,
            },
        }
    }

    /// Store a slice and return a native slice reference.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeSlice { data, len }
    }

    /// Store a slice and return a native array reference.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeArray {
            data,
            len,
            capacity: len,
        }
    }

    /// Store a string slice and return a native string slice.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr() as *const NativeStringRef;
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeStringSlice { data, len }
    }
}
