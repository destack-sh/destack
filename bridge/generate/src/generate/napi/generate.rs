use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;

use crate::generate::core::{
    Field, Item, Payload, PayloadNames, Schema, SchemaModule, Shape, Type, Variant, write_rust,
};

/// Generate NAPI bridge bindings.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    for module in &schema.modules {
        let tokens = render_module(schema, &module.names);
        let path = generated_path(module);

        write_rust(root, &path, tokens)?;
    }

    Ok(())
}

/// Return one generated NAPI module path.
fn generated_path(module: &SchemaModule) -> String {
    format!("bridge/napi/src/{}/generated.rs", module.path.slash_path())
}

/// Render one generated NAPI module.
fn render_module(schema: &Schema, names: &[String]) -> TokenStream {
    let mut items = Vec::new();
    let mut helpers = Vec::new();
    let imports = render_imports(schema, names);

    for name in names {
        let ty = schema.item(name);

        match &ty.shape {
            Shape::Struct(fields) => {
                items.push(render_struct(schema, ty, fields));
            }
            Shape::Enum(variants) if schema.is_unit_enum(&ty.name) => {
                if schema.unit_enum_needs_parse(&ty.name) {
                    helpers.push(render_unit_enum_parse(ty, variants));
                }
                if schema.unit_enum_needs_label(&ty.name) {
                    helpers.push(render_unit_enum_label(ty, variants));
                }
            }
            Shape::Enum(variants) => {
                items.push(render_payload_enum(schema, ty, variants));
            }
        }
    }

    quote! {
        use destack_bridge_language as bridge;
        use napi_derive::napi;
        #imports

        #(#items)*
        #(#helpers)*
    }
}

/// Render binding imports referenced by this generated module.
fn render_imports(schema: &Schema, names: &[String]) -> TokenStream {
    let imports = schema.referenced_items(names);

    if imports.is_empty() {
        return quote!();
    }

    let imports = imports.iter().map(|item| item.ident());

    quote!(use crate::{#(#imports,)*};)
}

/// Render one NAPI struct.
fn render_struct(schema: &Schema, ty: &Item, fields: &[Field]) -> TokenStream {
    let name = ty.ident();
    let docs = ty.docs();
    let fields = fields
        .iter()
        .map(|field| render_struct_field(schema, field));
    let into_bridge = ty
        .generates_into_bridge()
        .then(|| render_struct_into_bridge(schema, ty));
    let from_bridge = ty
        .generates_from_bridge()
        .then(|| render_struct_from_bridge(schema, ty));

    quote! {
        #docs
        #[derive(Debug)]
        #[napi(object)]
        pub struct #name {
            #(#fields)*
        }

        #into_bridge
        #from_bridge
    }
}

/// Render one NAPI struct field.
fn render_struct_field(schema: &Schema, field: &Field) -> TokenStream {
    let docs = field.docs();
    let name = field.ident();
    let ty = render_type(schema, &field.ty);

    quote! {
        #docs
        pub #name: #ty,
    }
}

/// Render one NAPI struct to bridge conversion.
fn render_struct_into_bridge(schema: &Schema, ty: &Item) -> TokenStream {
    let name = ty.ident();
    let fields = match &ty.shape {
        Shape::Struct(fields) => fields,
        Shape::Enum(_) => unreachable!("struct conversion requires struct"),
    };
    let field_values = fields.iter().map(|field| {
        let name = field.ident();
        let value = render_into_bridge_value(schema, quote!(self.#name), &field.ty);

        quote!(#name: #value,)
    });

    quote! {
        impl #name {
            /// Convert this NAPI value into one bridge value.
            pub(crate) fn into_bridge(self) -> napi::Result<bridge::#name> {
                Ok(bridge::#name {
                    #(#field_values)*
                })
            }
        }
    }
}

/// Render one bridge to NAPI struct conversion.
fn render_struct_from_bridge(schema: &Schema, ty: &Item) -> TokenStream {
    let name = ty.ident();
    let fields = match &ty.shape {
        Shape::Struct(fields) => fields,
        Shape::Enum(_) => unreachable!("struct conversion requires struct"),
    };
    let field_values = fields.iter().map(|field| {
        let name = field.ident();
        let value = render_from_bridge_value(schema, quote!(value.#name), &field.ty);

        quote!(#name: #value,)
    });

    quote! {
        impl #name {
            /// Convert one bridge value into one NAPI value.
            pub(crate) fn from_bridge(value: bridge::#name) -> Self {
                Self {
                    #(#field_values)*
                }
            }
        }
    }
}

/// Render one NAPI payload enum object.
fn render_payload_enum(schema: &Schema, ty: &Item, variants: &[Variant]) -> TokenStream {
    let name = ty.ident();
    let docs = ty.docs();
    let payload_names = PayloadNames::new(variants);
    let payload_fields = payload_enum_fields(variants, &payload_names);
    let fields = payload_fields
        .iter()
        .map(|field| render_payload_enum_field(schema, field));
    let into_bridge = ty
        .generates_into_bridge()
        .then(|| render_payload_enum_into_bridge(schema, ty, variants));
    let from_bridge = ty
        .generates_from_bridge()
        .then(|| render_payload_enum_from_bridge(schema, ty, variants, &payload_fields));

    quote! {
        #docs
        #[derive(Debug)]
        #[napi(object)]
        pub struct #name {
            /// Payload variant label.
            pub kind: String,
            #(#fields)*
        }

        #into_bridge
        #from_bridge
    }
}

/// Render one NAPI payload enum field.
fn render_payload_enum_field(schema: &Schema, field: &Field) -> TokenStream {
    let docs = field.docs();
    let name = field.ident();
    let ty = render_type(schema, &field.ty);

    quote! {
        #docs
        pub #name: Option<#ty>,
    }
}

/// Return unique payload enum fields across variants.
fn payload_enum_fields(variants: &[Variant], payload_names: &PayloadNames) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();

    for variant in variants {
        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(_) => {
                let name = payload_names.tuple_label(variant);
                if names.insert(name.clone()) {
                    fields.push(Field {
                        name,
                        docs: variant.docs.clone(),
                        ty: match &variant.payload {
                            Payload::Tuple(ty) => ty.clone(),
                            _ => unreachable!("tuple payload changed while collecting fields"),
                        },
                    });
                }
            }
            Payload::Struct(variant_fields) => {
                for field in variant_fields {
                    let name = payload_names.field_name(variant, field);
                    if names.insert(name.clone()) {
                        let mut field = field.clone();
                        field.name = name;
                        fields.push(field);
                    }
                }
            }
        }
    }

    fields
}

/// Render one NAPI payload enum to bridge conversion.
fn render_payload_enum_into_bridge(
    schema: &Schema,
    ty: &Item,
    variants: &[Variant],
) -> TokenStream {
    let name = ty.ident();
    let payload_names = PayloadNames::new(variants);
    let payload_fields = payload_enum_fields(variants, &payload_names);
    let arms = variants.iter().map(|variant| {
        let label = variant.label();
        let variant_name = variant.ident();
        let unexpected_payloads =
            render_unexpected_payload_checks(&payload_fields, variant, &payload_names);

        match &variant.payload {
            Payload::Unit => quote! {
                #label => {
                    #unexpected_payloads

                    Ok(bridge::#name::#variant_name)
                }
            },
            Payload::Tuple(ty) => {
                let field = payload_names.tuple_ident(variant);
                let value = render_into_bridge_value(schema, quote!(value), ty);

                quote! {
                    #label => {
                        #unexpected_payloads

                        let Some(value) = self.#field else {
                            return Err(missing_payload(#label));
                        };

                        Ok(bridge::#name::#variant_name(#value))
                    }
                }
            }
            Payload::Struct(fields) => {
                let payload = fields.iter().map(|field| {
                    let transport_name = payload_names.field_ident(variant, field);
                    let field_name = field.ident();
                    let value = render_into_bridge_value(schema, quote!(value), &field.ty);
                    let label = payload_names.field_label(variant, field);

                    quote! {
                        let Some(value) = self.#transport_name else {
                            return Err(missing_payload(#label));
                        };
                        let #field_name = #value;
                    }
                });
                let field_values = fields.iter().map(|field| {
                    let field_name = field.ident();

                    quote!(#field_name,)
                });

                quote! {
                    #label => {
                        #unexpected_payloads

                        #(#payload)*

                        Ok(bridge::#name::#variant_name {
                            #(#field_values)*
                        })
                    }
                }
            }
        }
    });

    quote! {
        impl #name {
            /// Convert this NAPI payload enum into one bridge enum.
            pub(crate) fn into_bridge(self) -> napi::Result<bridge::#name> {
                match self.kind.as_str() {
                    #(#arms)*
                    _ => Err(napi::Error::from_reason(format!(
                        "unknown {}: {}",
                        stringify!(#name),
                        self.kind
                    ))),
                }
            }
        }

        /// Return one missing payload error.
        fn missing_payload(kind: &str) -> napi::Error {
            napi::Error::from_reason(format!("{kind} payload is missing"))
        }

        /// Return one unexpected payload error.
        fn unexpected_payload(kind: &str) -> napi::Error {
            napi::Error::from_reason(format!("{kind} payload is unexpected"))
        }
    }
}

/// Render unexpected payload checks for one payload variant.
fn render_unexpected_payload_checks(
    payload_fields: &[Field],
    variant: &Variant,
    payload_names: &PayloadNames,
) -> TokenStream {
    let expected = payload_field_names(variant, payload_names);
    let checks = payload_fields
        .iter()
        .filter(|field| !expected.contains(&field.name))
        .map(|field| {
            let name = field.ident();
            let label = field.name.as_str();

            quote! {
                if self.#name.is_some() {
                    return Err(unexpected_payload(#label));
                }
            }
        });

    quote!(#(#checks)*)
}

/// Return payload field names used by one variant.
fn payload_field_names(variant: &Variant, payload_names: &PayloadNames) -> BTreeSet<String> {
    match &variant.payload {
        Payload::Unit => BTreeSet::new(),
        Payload::Tuple(_) => BTreeSet::from([payload_names.tuple_label(variant)]),
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| payload_names.field_name(variant, field))
            .collect(),
    }
}

/// Render one bridge payload enum to NAPI conversion.
fn render_payload_enum_from_bridge(
    schema: &Schema,
    ty: &Item,
    variants: &[Variant],
    payload_fields: &[Field],
) -> TokenStream {
    let name = ty.ident();
    let payload_names = PayloadNames::new(variants);
    let arms = variants.iter().map(|variant| {
        let label = variant.label();
        let variant_name = variant.ident();
        let expected_fields = payload_field_names(variant, &payload_names);
        let empty_fields = payload_fields
            .iter()
            .filter(|field| !expected_fields.contains(&field.name))
            .map(|field| {
                let field_name = field.ident();

                quote!(#field_name: None,)
            });

        match &variant.payload {
            Payload::Unit => quote! {
                bridge::#name::#variant_name => Self {
                    kind: #label.to_string(),
                    #(#empty_fields)*
                },
            },
            Payload::Tuple(ty) => {
                let field = variant.payload_field_ident();
                let field_value = render_from_bridge_value(schema, quote!(value), ty);

                quote! {
                    bridge::#name::#variant_name(value) => Self {
                        kind: #label.to_string(),
                        #field: Some(#field_value),
                        #(#empty_fields)*
                    },
                }
            }
            Payload::Struct(fields) => {
                let bindings = fields.iter().map(Field::ident);
                let field_values = fields.iter().map(|field| {
                    let transport_name = payload_names.field_ident(variant, field);
                    let field_name = field.ident();
                    let field_value =
                        render_from_bridge_value(schema, quote!(#field_name), &field.ty);

                    quote!(#transport_name: Some(#field_value),)
                });

                quote! {
                    bridge::#name::#variant_name { #(#bindings,)* } => Self {
                        kind: #label.to_string(),
                        #(#field_values)*
                        #(#empty_fields)*
                    },
                }
            }
        }
    });

    quote! {
        impl #name {
            /// Convert one bridge payload enum into one NAPI payload enum.
            pub(crate) fn from_bridge(value: bridge::#name) -> Self {
                match value {
                    #(#arms)*
                }
            }
        }
    }
}

/// Render one unit enum label helper.
fn render_unit_enum_label(ty: &Item, variants: &[Variant]) -> TokenStream {
    let name = ty.ident();
    let helper = ty.label_ident();
    let arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let label = variant.label();

        quote!(bridge::#name::#variant_name => #label,)
    });

    quote! {
        /// Return one target enum label.
        fn #helper(value: bridge::#name) -> String {
            let label = match value {
                #(#arms)*
            };

            label.to_string()
        }
    }
}

/// Render one unit enum parse helper.
fn render_unit_enum_parse(ty: &Item, variants: &[Variant]) -> TokenStream {
    let name = ty.ident();
    let helper = ty.parse_ident();
    let arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let label = variant.label();

        quote!(#label => Ok(bridge::#name::#variant_name),)
    });

    quote! {
        /// Parse one target enum label.
        fn #helper(value: &str) -> napi::Result<bridge::#name> {
            match value {
                #(#arms)*
                _ => Err(napi::Error::from_reason(format!(
                    "unknown {}: {value}",
                    stringify!(#name),
                ))),
            }
        }
    }
}

/// Render one NAPI type reference.
fn render_type(schema: &Schema, ty: &Type) -> TokenStream {
    match ty {
        Type::String => quote!(String),
        Type::Bool => quote!(bool),
        Type::U8 => quote!(u8),
        Type::U32 => quote!(u32),
        Type::Usize => quote!(u32),
        Type::Vec(ty) => {
            let ty = render_type(schema, ty);

            quote!(Vec<#ty>)
        }
        Type::Option(ty) => {
            let ty = render_type(schema, ty);

            quote!(Option<#ty>)
        }
        Type::Named(name) if schema.is_unit_enum(name) => quote!(String),
        Type::Named(name) => {
            let name = schema.item(name).ident();

            quote!(#name)
        }
    }
}

/// Render one value converted into bridge space.
fn render_into_bridge_value(schema: &Schema, value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Vec(ty) => {
            if !ty.needs_napi_into_bridge_conversion(schema) {
                return value;
            }

            let item = render_into_bridge_value(schema, quote!(item), ty);

            quote!(#value.into_iter().map(|item| Ok::<_, napi::Error>(#item)).collect::<napi::Result<Vec<_>>>()?)
        }
        Type::Option(ty) => {
            if !ty.needs_napi_into_bridge_conversion(schema) {
                return value;
            }

            let item = render_into_bridge_value(schema, quote!(item), ty);

            quote!(#value.map(|item| Ok::<_, napi::Error>(#item)).transpose()?)
        }
        Type::Named(name) if !schema.is_unit_enum(name) => quote!(#value.into_bridge()?),
        Type::Named(name) if schema.is_unit_enum(name) => {
            let helper = schema.item(name).parse_ident();

            quote!(#helper(#value.as_str())?)
        }
        _ => value,
    }
}

/// Render one value converted out of bridge space.
fn render_from_bridge_value(schema: &Schema, value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Vec(ty) => {
            if !ty.needs_from_bridge_conversion(schema) {
                return value;
            }

            let item = render_from_bridge_value(schema, quote!(item), ty);

            quote!(#value.into_iter().map(|item| #item).collect())
        }
        Type::Option(ty) => {
            if !ty.needs_from_bridge_conversion(schema) {
                return value;
            }

            let item = render_from_bridge_value(schema, quote!(item), ty);

            quote!(#value.map(|item| #item))
        }
        Type::Named(name) if schema.is_unit_enum(name) => {
            let helper = schema.item(name).label_ident();

            quote!(#helper(#value))
        }
        Type::Named(name) => {
            let name = schema.item(name).ident();

            quote!(#name::from_bridge(#value))
        }
        _ => value,
    }
}
