use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::generate::core::to_snake;
use crate::generate::schema::{Field, Item, PayloadNames, Schema, Shape, Variant};

use super::convert::{
    array_impl, bridge_type, destroy_value, empty_value, from_bridge_value, optional_impl,
    rust_c_type, to_bridge_value,
};
use super::payload::{payload_fields, payload_storage_fields, variant_pattern, variant_value};
use super::projection::Projection;

/// Render one projected item.
pub(super) fn render_item(schema: &Schema, projection: &Projection<'_>, name: &str) -> TokenStream {
    let item = schema.item(name);

    match &item.shape {
        Shape::Struct(fields) => render_struct(projection, item, fields),
        Shape::Enum(variants) if schema.is_unit_enum(name) => render_unit_enum(item, variants),
        Shape::Enum(variants) => render_payload_enum(projection, item, variants),
    }
}

/// Render one Rust C struct.
fn render_struct(projection: &Projection<'_>, item: &Item, fields: &[Field]) -> TokenStream {
    let name = format_ident!("Destack{}", item.name);
    let bridge_name = item.ident();
    let bridge = bridge_type(&bridge_name);
    let array_name = format_ident!("Destack{}Array", item.name);
    let optional_name = format_ident!("DestackOptional{}", item.name);
    let array_impl = array_impl(&name, &bridge_name, &array_name);
    let optional_impl = optional_impl(&name, &bridge_name, &optional_name);
    let field_defs = fields.iter().map(|field| {
        let docs = field.docs();
        let name = format_ident!("{}", field.name);
        let ty = rust_c_type(&field.ty, projection);

        quote! {
            #docs
            pub(crate) #name: #ty,
        }
    });
    let from_fields = fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        let value = from_bridge_value(&field.ty, quote!(value.#name), projection);

        quote!(#name: #value,)
    });
    let to_fields = fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        let value = to_bridge_value(&field.ty, quote!(self.#name), projection);

        quote!(#name: #value,)
    });
    let destroy_fields = fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        destroy_value(&field.ty, quote!(self.#name), projection)
    });
    let empty_fields = fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        let value = empty_value(&field.ty, projection);

        quote!(#name: #value,)
    });

    quote! {
        /// C ABI bridge value.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #name {
            #(#field_defs)*
        }

        /// C ABI bridge value array.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #array_name {
            /// Owned value pointer.
            pub(crate) ptr: *mut #name,
            /// Value count.
            pub(crate) len: usize,
        }

        /// C ABI optional bridge value.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #optional_name {
            /// Whether the value is present.
            pub(crate) is_some: bool,
            /// Value when present.
            pub(crate) value: #name,
        }

        impl #name {
            /// Convert one bridge value into one C ABI value.
            pub(crate) fn from_bridge(value: #bridge) -> Result<Self, String> {
                Ok(Self {
                    #(#from_fields)*
                })
            }

            /// Convert this C ABI value into one bridge value.
            pub(crate) fn to_bridge(&self) -> Result<#bridge, String> {
                Ok(#bridge {
                    #(#to_fields)*
                })
            }

            /// Destroy this C ABI value.
            pub(crate) fn destroy(&mut self) {
                #(#destroy_fields)*
            }

            /// Return one empty C ABI value.
            pub(crate) fn empty() -> Self {
                Self {
                    #(#empty_fields)*
                }
            }
        }

        #array_impl

        #optional_impl
    }
}

/// Render one Rust C unit enum.
fn render_unit_enum(item: &Item, variants: &[Variant]) -> TokenStream {
    let name = format_ident!("Destack{}", item.name);
    let bridge_name = item.ident();
    let bridge = bridge_type(&bridge_name);
    let array_name = format_ident!("Destack{}Array", item.name);
    let optional_name = format_ident!("DestackOptional{}", item.name);
    let array_impl = array_impl(&name, &bridge_name, &array_name);
    let optional_impl = optional_impl(&name, &bridge_name, &optional_name);
    let empty = variants[0].ident();
    let variant_defs = variants.iter().enumerate().map(|(index, variant)| {
        let docs = variant.docs();
        let name = variant.ident();
        let index = Literal::usize_unsuffixed(index);

        quote! {
            #docs
            #name = #index,
        }
    });
    let from_arms = variants.iter().map(|variant| {
        let name = variant.ident();

        quote!(#bridge::#name => Self::#name,)
    });
    let into_arms = variants.iter().map(|variant| {
        let name = variant.ident();

        quote!(Self::#name => #bridge::#name,)
    });

    quote! {
        /// C ABI bridge enum.
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #name {
            #(#variant_defs)*
        }

        /// C ABI bridge enum array.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #array_name {
            /// Owned value pointer.
            pub(crate) ptr: *mut #name,
            /// Value count.
            pub(crate) len: usize,
        }

        /// C ABI optional bridge enum.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #optional_name {
            /// Whether the value is present.
            pub(crate) is_some: bool,
            /// Value when present.
            pub(crate) value: #name,
        }

        impl #name {
            /// Convert one bridge enum into one C ABI enum.
            pub(crate) fn from_bridge(value: #bridge) -> Result<Self, String> {
                Ok(match value {
                    #(#from_arms)*
                })
            }

            /// Convert this C ABI enum into one bridge enum.
            pub(crate) fn to_bridge(self) -> Result<#bridge, String> {
                Ok(match self {
                    #(#into_arms)*
                })
            }

            /// Destroy this C ABI enum.
            pub(crate) fn destroy(&mut self) {}

            /// Return one empty C ABI enum.
            pub(crate) fn empty() -> Self {
                Self::#empty
            }
        }

        #array_impl

        #optional_impl
    }
}

/// Render one Rust C payload enum.
fn render_payload_enum(
    projection: &Projection<'_>,
    item: &Item,
    variants: &[Variant],
) -> TokenStream {
    let name = format_ident!("Destack{}", item.name);
    let kind = format_ident!("Destack{}Kind", item.name);
    let bridge_name = item.ident();
    let bridge = bridge_type(&bridge_name);
    let array_name = format_ident!("Destack{}Array", item.name);
    let optional_name = format_ident!("DestackOptional{}", item.name);
    let array_impl = array_impl(&name, &bridge_name, &array_name);
    let optional_impl = optional_impl(&name, &bridge_name, &optional_name);
    let empty_kind = variants[0].ident();
    let kind_defs = variants.iter().enumerate().map(|(index, variant)| {
        let docs = variant.docs();
        let name = variant.ident();
        let index = Literal::usize_unsuffixed(index);

        quote! {
            #docs
            #name = #index,
        }
    });
    let names = PayloadNames::new(variants);
    let all_fields = payload_storage_fields(&names, variants);
    let field_defs = all_fields
        .iter()
        .map(|field| {
            let name = format_ident!("{}", field.name);
            let ty = rust_c_type(field.ty, projection);

            quote!(pub(crate) #name: #ty,)
        })
        .collect::<Vec<_>>();
    let empty_fields = all_fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        let value = empty_value(field.ty, projection);

        quote!(#name: #value,)
    });
    let from_arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let active = payload_fields(&names, variant);
        let values = all_fields.iter().map(|field| {
            let target = format_ident!("{}", field.name);
            if let Some(active) = active.iter().find(|active| active.name == field.name) {
                let source = format_ident!("{}", active.source_name);
                let value = from_bridge_value(active.ty, quote!(#source), projection);

                quote!(#target: #value,)
            } else {
                let value = empty_value(field.ty, projection);

                quote!(#target: #value,)
            }
        });
        let pattern = variant_pattern(variant);

        quote! {
            #bridge::#variant_name #pattern => Self {
                kind: #kind::#variant_name,
                #(#values)*
            },
        }
    });
    let into_arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let fields = payload_fields(&names, variant);
        let conversions = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let value = to_bridge_value(field.ty, quote!(self.#name), projection);

            quote!(let #name = #value;)
        });
        let value = variant_value(variant, &fields);

        quote! {
            #kind::#variant_name => {
                #(#conversions)*
                Ok(#bridge::#variant_name #value)
            }
        }
    });
    let destroy_fields = all_fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        destroy_value(field.ty, quote!(self.#name), projection)
    });

    quote! {
        /// C ABI bridge enum kind.
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #kind {
            #(#kind_defs)*
        }

        /// C ABI bridge enum.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #name {
            /// Active enum variant.
            pub(crate) kind: #kind,
            #(#field_defs)*
        }

        /// C ABI bridge enum array.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #array_name {
            /// Owned value pointer.
            pub(crate) ptr: *mut #name,
            /// Value count.
            pub(crate) len: usize,
        }

        /// C ABI optional bridge enum.
        #[repr(C)]
        #[derive(Debug)]
        pub struct #optional_name {
            /// Whether the value is present.
            pub(crate) is_some: bool,
            /// Value when present.
            pub(crate) value: #name,
        }

        impl #name {
            /// Convert one bridge enum into one C ABI enum.
            pub(crate) fn from_bridge(value: #bridge) -> Result<Self, String> {
                Ok(match value {
                    #(#from_arms)*
                })
            }

            /// Convert this C ABI enum into one bridge enum.
            pub(crate) fn to_bridge(&self) -> Result<#bridge, String> {
                match self.kind {
                    #(#into_arms)*
                }
            }

            /// Destroy this C ABI enum.
            pub(crate) fn destroy(&mut self) {
                #(#destroy_fields)*
            }

            /// Return one empty C ABI enum.
            pub(crate) fn empty() -> Self {
                Self {
                    kind: #kind::#empty_kind,
                    #(#empty_fields)*
                }
            }
        }

        #array_impl

        #optional_impl
    }
}

/// Render destructors for one projected value.
pub(super) fn render_destructors(name: &str) -> TokenStream {
    let value = format_ident!("Destack{name}");
    let array = format_ident!("Destack{name}Array");
    let destroy_value = format_ident!("destack_{}_destroy", to_snake(name));
    let destroy_array = format_ident!("destack_{}_array_destroy", to_snake(name));

    quote! {
        /// Destroy one C ABI bridge value.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #destroy_value(value: *mut #value) {
            if let Some(value) = unsafe { value.as_mut() } {
                value.destroy();
            }
        }

        /// Destroy one C ABI bridge value array.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #destroy_array(mut array: #array) {
            array.destroy();
        }
    }
}
