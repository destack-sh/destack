use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::generate::core::to_snake;

use super::convert::bridge_type;

/// Render one opaque C ABI handle.
pub(super) fn render_handle(name: &str) -> TokenStream {
    let handle = format_ident!("Destack{name}");
    let bridge = bridge_type(&format_ident!("{name}"));
    let docs = format!(" C ABI {} handle.", to_snake(name).replace('_', " "));

    quote! {
        #[doc = #docs]
        #[repr(C)]
        #[derive(Debug)]
        pub struct #handle {
            /// Rust bridge value.
            pub(crate) value: #bridge,
        }
    }
}

/// Render one opaque C ABI handle destructor.
pub(super) fn render_handle_destructor(name: &str) -> TokenStream {
    let handle = format_ident!("Destack{name}");
    let destroy = format_ident!("destack_{}_destroy", to_snake(name));
    let docs = format!(" Destroy one {} handle.", to_snake(name).replace('_', " "));

    quote! {
        #[doc = #docs]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #destroy(value: *mut #handle) {
            if value.is_null() {
                return;
            }

            drop(unsafe { Box::from_raw(value) });
        }
    }
}
