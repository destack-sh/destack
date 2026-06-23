use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::generate::schema::{Payload, PayloadNames, Type, Variant};

pub(super) struct PayloadField<'schema> {
    /// Rust field name in the source variant.
    pub(super) source_name: String,
    /// C ABI field name.
    pub(super) name: String,
    /// Field type.
    pub(super) ty: &'schema Type,
}

/// Return unique storage fields for one payload enum.
pub(super) fn payload_storage_fields<'schema>(
    names: &PayloadNames,
    variants: &'schema [Variant],
) -> Vec<PayloadField<'schema>> {
    let mut fields = Vec::new();
    let mut inserted = BTreeSet::new();

    for variant in variants {
        for field in payload_fields(names, variant) {
            let key = (field.name.clone(), field.ty.clone());
            if inserted.insert(key) {
                fields.push(field);
            }
        }
    }

    fields
}

/// Return fields for one payload variant.
pub(super) fn payload_fields<'schema>(
    names: &PayloadNames,
    variant: &'schema Variant,
) -> Vec<PayloadField<'schema>> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Tuple(ty) => vec![PayloadField {
            source_name: variant.payload_field_name(),
            name: names.tuple_label(variant),
            ty,
        }],
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| PayloadField {
                source_name: field.name.clone(),
                name: names.field_name(variant, field),
                ty: &field.ty,
            })
            .collect(),
    }
}

/// Return one Rust pattern for a bridge enum variant.
pub(super) fn variant_pattern(variant: &Variant) -> TokenStream {
    match &variant.payload {
        Payload::Unit => quote!(),
        Payload::Tuple(_) => {
            let name = variant.payload_field_ident();

            quote!((#name))
        }
        Payload::Struct(fields) => {
            let fields = fields.iter().map(|field| field.ident());

            quote!({ #(#fields,)* })
        }
    }
}

/// Return one Rust value for a bridge enum variant.
pub(super) fn variant_value(variant: &Variant, fields: &[PayloadField<'_>]) -> TokenStream {
    match &variant.payload {
        Payload::Unit => quote!(),
        Payload::Tuple(_) => {
            let name = format_ident!("{}", fields[0].name);

            quote!((#name))
        }
        Payload::Struct(_) => {
            let fields = fields.iter().map(|field| {
                let source = format_ident!("{}", field.source_name);
                let name = format_ident!("{}", field.name);

                if field.source_name == field.name {
                    quote!(#source,)
                } else {
                    quote!(#source: #name,)
                }
            });

            quote!({ #(#fields)* })
        }
    }
}
