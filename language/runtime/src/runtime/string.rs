use std::cell::RefCell;
use std::ptr;

use crate::platform::NativeStringRef;

/// Per-call storage for native string references returned by runtime bindings.
///
/// Stored strings are valid until the next runtime call on the same thread.
#[derive(Debug, Default)]
pub struct RuntimeCallStringStore {
    /// Owned strings backing native string references.
    strings: RefCell<Vec<Box<str>>>,
}

impl RuntimeCallStringStore {
    /// Clear all stored strings.
    pub fn clear(&self) {
        self.strings.borrow_mut().clear();
    }

    /// Store a string and return a native string reference.
    pub fn store(&self, value: &str) -> NativeStringRef {
        let mut strings = self.strings.borrow_mut();
        strings.push(value.to_owned().into_boxed_str());

        let stored = strings.last().expect("stored string must be available");
        NativeStringRef::from(stored.as_ref())
    }

    /// Store an optional string and return a native string reference.
    pub fn store_option(&self, value: Option<&String>) -> NativeStringRef {
        match value {
            Some(value) => self.store(value),
            None => NativeStringRef {
                data: ptr::null(),
                len: 0,
            },
        }
    }
}
