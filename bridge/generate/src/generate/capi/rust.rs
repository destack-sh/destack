use proc_macro2::TokenStream;
use quote::quote;

use crate::generate::schema::Schema;

use super::artifact::render_artifact_key_constructor;
use super::handle::{render_handle, render_handle_destructor};
use super::projection::Projection;
use super::value::{render_destructors, render_item};

pub(super) struct Rust<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// C ABI projection.
    projection: &'schema Projection<'schema>,
}

impl<'schema> Rust<'schema> {
    /// Create one C ABI Rust generator.
    pub(super) fn new(schema: &'schema Schema, projection: &'schema Projection<'schema>) -> Self {
        Self { schema, projection }
    }

    /// Render the generated Rust C ABI.
    pub(super) fn render(&self) -> TokenStream {
        let items = self
            .projection
            .values
            .iter()
            .map(|name| render_item(self.schema, self.projection, name));
        let destructors = self
            .projection
            .values
            .iter()
            .map(|name| render_destructors(name));
        let handles = self
            .projection
            .handles
            .iter()
            .map(|name| render_handle(name));
        let handle_destructors = self
            .projection
            .handles
            .iter()
            .map(|name| render_handle_destructor(name));
        let constructors = self
            .projection
            .artifact_key_variants
            .iter()
            .map(|variant| render_artifact_key_constructor(self.projection, variant));

        quote! {
            use std::{ffi::c_char, ptr, slice};

            use crate::core::{
                DestackError, DestackStatus, c_string, destroy_array, destroy_string, owned_array,
                read_bytes, read_string, return_status, write_out,
            };

            /// C ABI owned byte array.
            #[repr(C)]
            #[derive(Debug)]
            pub struct DestackByteArray {
                /// Owned byte pointer.
                pub(crate) ptr: *mut u8,
                /// Byte count.
                pub(crate) len: usize,
            }

            /// C ABI unsigned 128 bit value.
            #[repr(C)]
            #[derive(Debug, Clone, Copy)]
            pub struct DestackU128 {
                /// Most significant 64 bits.
                pub(crate) high: u64,
                /// Least significant 64 bits.
                pub(crate) low: u64,
            }

            /// C ABI owned string array.
            #[repr(C)]
            #[derive(Debug)]
            pub struct DestackStringArray {
                /// Owned string pointer.
                pub(crate) ptr: *mut *mut c_char,
                /// String count.
                pub(crate) len: usize,
            }

            /// C ABI optional owned string.
            #[repr(C)]
            #[derive(Debug)]
            pub struct DestackOptionalString {
                /// Whether the value is present.
                pub(crate) is_some: bool,
                /// Owned string value when present.
                pub(crate) value: *mut c_char,
            }

            impl DestackByteArray {
                /// Convert Rust bytes into one C ABI byte array.
                pub(crate) fn from_vec(values: Vec<u8>) -> Self {
                    let (ptr, len) = owned_array(values);

                    Self { ptr, len }
                }

                /// Convert this C ABI byte array into Rust bytes.
                pub(crate) fn into_vec(self) -> Result<Vec<u8>, String> {
                    read_bytes(self.ptr.cast_const(), self.len)
                }

                /// Destroy this C ABI byte array.
                pub(crate) fn destroy(&mut self) {
                    if self.ptr.is_null() {
                        return;
                    }

                    unsafe {
                        destroy_array(self.ptr, self.len, |_| {});
                    }
                    self.ptr = ptr::null_mut();
                    self.len = 0;
                }
            }

            impl DestackU128 {
                /// Convert one Rust `u128` into one C ABI value.
                pub(crate) fn from_bridge(value: u128) -> Self {
                    Self {
                        high: (value >> 64) as u64,
                        low: value as u64,
                    }
                }

                /// Convert this C ABI value into one Rust `u128`.
                pub(crate) fn to_bridge(self) -> u128 {
                    ((self.high as u128) << 64) | self.low as u128
                }
            }

            impl DestackStringArray {
                /// Convert Rust strings into one C ABI string array.
                pub(crate) fn from_bridge(values: Vec<String>) -> Result<Self, String> {
                    let mut converted = Vec::with_capacity(values.len());
                    for value in values {
                        converted.push(c_string(value)?);
                    }
                    let (ptr, len) = owned_array(converted);

                    Ok(Self { ptr, len })
                }

                /// Convert this C ABI string array into Rust strings.
                pub(crate) fn to_bridge(&self) -> Result<Vec<String>, String> {
                    if self.len == 0 {
                        return Ok(Vec::new());
                    }
                    if self.ptr.is_null() {
                        return Err("string array pointer is null".to_string());
                    }

                    let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
                    let mut converted = Vec::with_capacity(values.len());
                    for value in values {
                        converted.push(read_string(*value)?);
                    }

                    Ok(converted)
                }

                /// Destroy this C ABI string array.
                pub(crate) fn destroy(&mut self) {
                    if self.ptr.is_null() {
                        return;
                    }

                    unsafe {
                        destroy_array(self.ptr, self.len, |value| {
                            destroy_string(*value);
                            *value = ptr::null_mut();
                        });
                    }
                    self.ptr = ptr::null_mut();
                    self.len = 0;
                }
            }

            impl DestackOptionalString {
                /// Convert one optional string into one C ABI optional string.
                pub(crate) fn from_bridge(value: Option<String>) -> Result<Self, String> {
                    let Some(value) = value else {
                        return Ok(Self::empty());
                    };

                    Ok(Self {
                        is_some: true,
                        value: c_string(value)?,
                    })
                }

                /// Convert this C ABI optional string into one optional string.
                pub(crate) fn to_bridge(&self) -> Result<Option<String>, String> {
                    if self.is_some {
                        Ok(Some(read_string(self.value)?))
                    } else {
                        Ok(None)
                    }
                }

                /// Destroy this C ABI optional string.
                pub(crate) fn destroy(&mut self) {
                    if self.is_some {
                        destroy_string(self.value);
                    }
                    *self = Self::empty();
                }

                /// Return one empty C ABI optional string.
                pub(crate) fn empty() -> Self {
                    Self {
                        is_some: false,
                        value: ptr::null_mut(),
                    }
                }
            }

            #(#handles)*

            #(#items)*

            #(#constructors)*

            /// Destroy one owned byte array.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn destack_byte_array_destroy(mut array: DestackByteArray) {
                array.destroy();
            }

            /// Destroy one owned string array.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn destack_string_array_destroy(mut array: DestackStringArray) {
                array.destroy();
            }

            /// Destroy one optional owned string.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn destack_optional_string_destroy(mut value: DestackOptionalString) {
                value.destroy();
            }

            #(#handle_destructors)*

            #(#destructors)*
        }
    }
}
