use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::generate::core::to_snake;
use crate::generate::schema::Type;

use super::projection::Projection;
use super::ty::named_type;

/// Render one array implementation.
pub(super) fn array_impl(
    value: &proc_macro2::Ident,
    bridge: &proc_macro2::Ident,
    array: &proc_macro2::Ident,
) -> TokenStream {
    let bridge = bridge_type(bridge);

    quote! {
        impl #array {
            /// Convert bridge values into one C ABI array.
            pub(crate) fn from_bridge(values: Vec<#bridge>) -> Result<Self, String> {
                let mut converted = Vec::with_capacity(values.len());
                for value in values {
                    converted.push(#value::from_bridge(value)?);
                }
                let (ptr, len) = owned_array(converted);

                Ok(Self { ptr, len })
            }

            /// Convert this C ABI array into bridge values.
            pub(crate) fn to_bridge(&self) -> Result<Vec<#bridge>, String> {
                if self.len == 0 {
                    return Ok(Vec::new());
                }
                if self.ptr.is_null() {
                    return Err("array pointer is null".to_string());
                }

                let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
                let mut converted = Vec::with_capacity(values.len());
                for value in values {
                    converted.push(value.to_bridge()?);
                }

                Ok(converted)
            }

            /// Destroy this C ABI array.
            pub(crate) fn destroy(&mut self) {
                if self.ptr.is_null() {
                    return;
                }

                unsafe {
                    destroy_array(self.ptr, self.len, |value| value.destroy());
                }
                self.ptr = ptr::null_mut();
                self.len = 0;
            }
        }
    }
}

/// Render one optional implementation.
pub(super) fn optional_impl(
    value: &proc_macro2::Ident,
    bridge: &proc_macro2::Ident,
    optional: &proc_macro2::Ident,
) -> TokenStream {
    let bridge = bridge_type(bridge);

    quote! {
        impl #optional {
            /// Convert one optional bridge value into one C ABI optional value.
            pub(crate) fn from_bridge(value: Option<#bridge>) -> Result<Self, String> {
                let Some(value) = value else {
                    return Ok(Self {
                        is_some: false,
                        value: #value::empty(),
                    });
                };

                Ok(Self {
                    is_some: true,
                    value: #value::from_bridge(value)?,
                })
            }

            /// Convert this C ABI optional value into one bridge optional value.
            pub(crate) fn to_bridge(&self) -> Result<Option<#bridge>, String> {
                if self.is_some {
                    Ok(Some(self.value.to_bridge()?))
                } else {
                    Ok(None)
                }
            }

            /// Destroy this C ABI optional value.
            pub(crate) fn destroy(&mut self) {
                if self.is_some {
                    self.value.destroy();
                }
                self.is_some = false;
                self.value = #value::empty();
            }
        }
    }
}

/// Return the Rust transport schema path for one bridge type.
pub(super) fn bridge_type(ident: &proc_macro2::Ident) -> TokenStream {
    quote!(destack::language::#ident)
}

pub(super) fn rust_c_type(ty: &Type, projection: &Projection<'_>) -> TokenStream {
    match ty {
        Type::String => quote!(*mut c_char),
        Type::Bool => quote!(bool),
        Type::Char => quote!(u32),
        Type::U8 => quote!(u8),
        Type::U32 => quote!(u32),
        Type::U64 => quote!(u64),
        Type::U128 => quote!(DestackU128),
        Type::Signed(_) => quote!(i32),
        Type::Float(32) => quote!(f32),
        Type::Float(_) => quote!(f64),
        Type::Usize => quote!(usize),
        Type::Vec(inner) if **inner == Type::U8 => quote!(DestackByteArray),
        Type::Vec(inner) if **inner == Type::String => quote!(DestackStringArray),
        Type::Vec(inner) => {
            let ty = format_ident!("Destack{}Array", named_type(inner));

            quote!(#ty)
        }
        Type::Option(inner) if **inner == Type::String => quote!(DestackOptionalString),
        Type::Option(inner) => {
            let ty = format_ident!("DestackOptional{}", named_type(inner));

            quote!(#ty)
        }
        Type::Named(name) if projection.is_handle(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(*mut #ty)
        }
        Type::Named(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(#ty)
        }
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return one value converted from bridge DTO space.
pub(super) fn from_bridge_value(
    ty: &Type,
    value: TokenStream,
    projection: &Projection<'_>,
) -> TokenStream {
    match ty {
        Type::String => quote!(c_string(#value)?),
        Type::Bool
        | Type::U8
        | Type::U32
        | Type::U64
        | Type::Signed(_)
        | Type::Float(_)
        | Type::Usize => value,
        Type::U128 => quote!(DestackU128::from_bridge(#value)),
        Type::Char => quote!(#value as u32),
        Type::Vec(inner) if **inner == Type::U8 => quote!(DestackByteArray::from_vec(#value)),
        Type::Vec(inner) if **inner == Type::String => {
            quote!(DestackStringArray::from_bridge(#value)?)
        }
        Type::Vec(inner) => {
            let ty = format_ident!("Destack{}Array", named_type(inner));

            quote!(#ty::from_bridge(#value)?)
        }
        Type::Option(inner) if **inner == Type::String => {
            quote!(DestackOptionalString::from_bridge(#value)?)
        }
        Type::Option(inner) => {
            let ty = format_ident!("DestackOptional{}", named_type(inner));

            quote!(#ty::from_bridge(#value)?)
        }
        Type::Named(name) if projection.is_handle(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(Box::into_raw(Box::new(#ty { value: #value })))
        }
        Type::Named(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(#ty::from_bridge(#value)?)
        }
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return one value converted into bridge DTO space.
pub(super) fn into_bridge_value(
    ty: &Type,
    value: TokenStream,
    projection: &Projection<'_>,
) -> TokenStream {
    match ty {
        Type::String => quote!(read_string(#value)?),
        Type::Bool
        | Type::U8
        | Type::U32
        | Type::U64
        | Type::Signed(_)
        | Type::Float(_)
        | Type::Usize => value,
        Type::U128 => quote!(#value.to_bridge()),
        Type::Char => quote!(char::from_u32(#value).ok_or("invalid bridge char")?),
        Type::Vec(inner) if **inner == Type::U8 => quote!(#value.into_vec()?),
        Type::Vec(inner) if **inner == Type::String => quote!(#value.to_bridge()?),
        Type::Vec(_) => quote!(#value.to_bridge()?),
        Type::Option(inner) if **inner == Type::String => quote!(#value.to_bridge()?),
        Type::Option(_) => quote!(#value.to_bridge()?),
        Type::Named(name) if projection.is_handle(name) => {
            quote!({
                let value = unsafe { #value.as_ref() }.ok_or("artifact key is null")?;
                value.value.clone()
            })
        }
        Type::Named(_) => quote!(#value.to_bridge()?),
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return one borrowed value converted into bridge DTO space.
pub(super) fn to_bridge_value(
    ty: &Type,
    value: TokenStream,
    projection: &Projection<'_>,
) -> TokenStream {
    match ty {
        Type::String => quote!(read_string(#value)?),
        Type::Bool
        | Type::U8
        | Type::U32
        | Type::U64
        | Type::Signed(_)
        | Type::Float(_)
        | Type::Usize => value,
        Type::U128 => quote!(#value.to_bridge()),
        Type::Char => quote!(char::from_u32(#value).ok_or("invalid bridge char")?),
        Type::Vec(inner) if **inner == Type::U8 => {
            quote!(read_bytes(#value.ptr.cast_const(), #value.len)?)
        }
        Type::Vec(inner) if **inner == Type::String => quote!(#value.to_bridge()?),
        Type::Vec(_) | Type::Option(_) => quote!(#value.to_bridge()?),
        Type::Named(name) if projection.is_handle(name) => {
            quote!({
                let value = unsafe { #value.as_ref() }.ok_or("artifact key is null")?;
                value.value.clone()
            })
        }
        Type::Named(_) => quote!(#value.to_bridge()?),
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return code that destroys one value.
pub(super) fn destroy_value(
    ty: &Type,
    value: TokenStream,
    projection: &Projection<'_>,
) -> TokenStream {
    match ty {
        Type::String => quote! {
            destroy_string(#value);
            #value = ptr::null_mut();
        },
        Type::Vec(inner) if **inner == Type::U8 => quote!(#value.destroy();),
        Type::Vec(inner) if **inner == Type::String => quote!(#value.destroy();),
        Type::Named(name) if projection.is_handle(name) => {
            let destroy = format_ident!("destack_{}_destroy", to_snake(name));

            quote! {
                if !#value.is_null() {
                    unsafe {
                        #destroy(#value);
                    }
                    #value = ptr::null_mut();
                }
            }
        }
        Type::Vec(_) | Type::Option(_) | Type::Named(_) => quote!(#value.destroy();),
        Type::Bool
        | Type::Char
        | Type::U8
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::Signed(_)
        | Type::Float(_)
        | Type::Usize => quote!(),
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return one empty C ABI value.
pub(super) fn empty_value(ty: &Type, projection: &Projection<'_>) -> TokenStream {
    match ty {
        Type::String => quote!(ptr::null_mut()),
        Type::Bool => quote!(false),
        Type::Char => quote!(0),
        Type::U8 | Type::U32 | Type::U64 | Type::Signed(_) | Type::Float(_) | Type::Usize => {
            quote!(0)
        }
        Type::U128 => quote!(DestackU128 { high: 0, low: 0 }),
        Type::Vec(inner) if **inner == Type::U8 => quote!(DestackByteArray {
            ptr: ptr::null_mut(),
            len: 0,
        }),
        Type::Vec(inner) if **inner == Type::String => quote!(DestackStringArray {
            ptr: ptr::null_mut(),
            len: 0,
        }),
        Type::Vec(inner) => {
            let ty = format_ident!("Destack{}Array", named_type(inner));

            quote!(#ty {
                ptr: ptr::null_mut(),
                len: 0,
            })
        }
        Type::Option(inner) if **inner == Type::String => quote!(DestackOptionalString::empty()),
        Type::Option(inner) => {
            let ty = format_ident!("DestackOptional{}", named_type(inner));
            let value = empty_named_value(named_type(inner));

            quote!(#ty {
                is_some: false,
                value: #value,
            })
        }
        Type::Named(name) if projection.is_handle(name) => quote!(ptr::null_mut()),
        Type::Named(name) => empty_named_value(name),
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return one empty named C ABI value.
fn empty_named_value(name: &str) -> TokenStream {
    let ty = format_ident!("Destack{name}");

    quote!(#ty::empty())
}
