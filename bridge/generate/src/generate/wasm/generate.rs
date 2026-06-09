use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;

use crate::generate::core::{
    Field, Item, Payload, PayloadNames, Schema, SchemaModule, Shape, Type, Variant, write_rust,
};

/// Generate WASM bridge bindings.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    for module in &schema.modules {
        let tokens = render_module(schema, &module.names);
        let path = generated_path(module);

        write_rust(root, &path, tokens)?;
    }

    Ok(())
}

/// Return one generated WASM module path.
fn generated_path(module: &SchemaModule) -> String {
    format!("bridge/wasm/src/{}.generated.rs", module.path.slash_path())
}

/// Render one generated WASM module.
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
        use wasm_bindgen::prelude::wasm_bindgen;
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

/// Render one WASM struct.
fn render_struct(schema: &Schema, ty: &Item, fields: &[Field]) -> TokenStream {
    let name = ty.ident();
    let docs = ty.docs();
    let field_defs = fields.iter().map(|field| {
        let name = field.ident();
        let ty = render_type(schema, &field.ty);

        quote!(#name: #ty,)
    });
    let constructor = render_struct_constructor(schema, fields);
    let getters = fields
        .iter()
        .map(|field| render_struct_getter(schema, field));
    let into_bridge = ty
        .generates_into_bridge()
        .then(|| render_struct_into_bridge(schema, ty));
    let from_bridge = ty
        .generates_from_bridge()
        .then(|| render_struct_from_bridge(schema, ty));

    quote! {
        #docs
        #[derive(Debug, Clone)]
        #[wasm_bindgen]
        pub struct #name {
            #(#field_defs)*
        }

        #[wasm_bindgen]
        impl #name {
            #constructor
            #(#getters)*
        }

        #into_bridge
        #from_bridge
    }
}

/// Render one WASM struct constructor.
fn render_struct_constructor(schema: &Schema, fields: &[Field]) -> TokenStream {
    let arguments = fields.iter().map(|field| {
        let name = field.ident();
        let ty = render_type(schema, &field.ty);

        quote!(#name: #ty,)
    });
    let field_values = fields.iter().map(|field| {
        let name = field.ident();

        quote!(#name,)
    });

    quote! {
        /// Create one value.
        #[wasm_bindgen(constructor)]
        pub fn new(#(#arguments)*) -> Self {
            Self {
                #(#field_values)*
            }
        }
    }
}

/// Render one WASM struct getter.
fn render_struct_getter(schema: &Schema, field: &Field) -> TokenStream {
    let docs = field.docs();
    let name = field.ident();
    let js_name = field.label();
    let ty = render_type(schema, &field.ty);
    let value = render_clone_value(quote!(self.#name), &field.ty);

    quote! {
        #docs
        #[wasm_bindgen(getter, js_name = #js_name)]
        pub fn #name(&self) -> #ty {
            #value
        }
    }
}

/// Render one WASM struct to bridge conversion.
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
            /// Convert this WASM value into one bridge value.
            pub(crate) fn into_bridge(self) -> bridge::#name {
                bridge::#name {
                    #(#field_values)*
                }
            }
        }
    }
}

/// Render one bridge to WASM struct conversion.
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
            /// Convert one bridge value into one WASM value.
            pub(crate) fn from_bridge(value: bridge::#name) -> Self {
                Self {
                    #(#field_values)*
                }
            }
        }
    }
}

/// Render one WASM payload enum.
fn render_payload_enum(schema: &Schema, ty: &Item, variants: &[Variant]) -> TokenStream {
    let name = ty.ident();
    let content_name = ty.payload_content_ident();
    let docs = ty.docs();
    let content_variants = variants
        .iter()
        .map(|variant| render_content_variant(schema, variant));
    let constructors = variants
        .iter()
        .map(|variant| render_payload_constructor(schema, variant, &content_name));
    let getters = ty
        .generates_from_bridge()
        .then(|| render_payload_getters(schema, variants, &content_name));
    let into_bridge = ty
        .generates_into_bridge()
        .then(|| render_payload_enum_into_bridge(schema, ty, variants, &content_name));
    let from_bridge = ty
        .generates_from_bridge()
        .then(|| render_payload_enum_from_bridge(schema, ty, variants, &content_name));

    quote! {
        #docs
        #[derive(Debug, Clone)]
        #[wasm_bindgen]
        pub struct #name {
            content: #content_name,
        }

        /// Concrete payload enum content.
        #[derive(Debug, Clone)]
        enum #content_name {
            #(#content_variants)*
        }

        #[wasm_bindgen]
        impl #name {
            #(#constructors)*
            #getters
        }

        #into_bridge
        #from_bridge
    }
}

/// Render WASM getters for one payload enum.
fn render_payload_getters(
    schema: &Schema,
    variants: &[Variant],
    content_name: &proc_macro2::Ident,
) -> TokenStream {
    let kind_getter = render_payload_kind_getter(variants, content_name);
    let payload_names = PayloadNames::new(variants);
    let payload_fields = payload_enum_fields(variants, &payload_names);
    let field_getters = payload_fields.iter().map(|field| {
        render_payload_field_getter(schema, variants, &payload_names, content_name, field)
    });

    quote! {
        #kind_getter
        #(#field_getters)*
    }
}

/// Render the variant label getter for one payload enum.
fn render_payload_kind_getter(
    variants: &[Variant],
    content_name: &proc_macro2::Ident,
) -> TokenStream {
    let arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let label = variant.label();

        match &variant.payload {
            Payload::Unit => quote!(#content_name::#variant_name => #label,),
            Payload::Tuple(_) => quote!(#content_name::#variant_name(..) => #label,),
            Payload::Struct(_) => quote!(#content_name::#variant_name { .. } => #label,),
        }
    });

    quote! {
        /// Payload variant label.
        #[wasm_bindgen(getter, js_name = "kind")]
        pub fn kind(&self) -> String {
            let label = match &self.content {
                #(#arms)*
            };

            label.to_string()
        }
    }
}

/// Render one optional payload field getter.
fn render_payload_field_getter(
    schema: &Schema,
    variants: &[Variant],
    payload_names: &PayloadNames,
    content_name: &proc_macro2::Ident,
    field: &Field,
) -> TokenStream {
    let field_name = field.ident();
    let label = field.label();
    let ty = render_type(schema, &field.ty);
    let arms = variants.iter().filter_map(|variant| {
        render_payload_field_getter_arm(content_name, variant, payload_names, field)
    });
    let docs = field.docs();

    quote! {
        #docs
        #[wasm_bindgen(getter, js_name = #label)]
        pub fn #field_name(&self) -> Option<#ty> {
            match &self.content {
                #(#arms)*
                _ => None,
            }
        }
    }
}

/// Render one payload field getter arm when a variant carries the field.
fn render_payload_field_getter_arm(
    content_name: &proc_macro2::Ident,
    variant: &Variant,
    payload_names: &PayloadNames,
    field: &Field,
) -> Option<TokenStream> {
    let variant_name = variant.ident();
    let value = render_borrowed_clone_value(quote!(value), &field.ty);

    match &variant.payload {
        Payload::Unit => None,
        Payload::Tuple(ty) if payload_names.tuple_label(variant) == field.name => {
            let value = render_borrowed_clone_value(quote!(value), ty);

            Some(quote!(#content_name::#variant_name(value) => Some(#value),))
        }
        Payload::Tuple(_) => None,
        Payload::Struct(fields) => {
            let Some(field_name) = fields
                .iter()
                .find(|variant_field| {
                    payload_names.field_name(variant, variant_field) == field.name
                })
                .map(Field::ident)
            else {
                return None;
            };

            Some(quote!(
                #content_name::#variant_name {
                    #field_name: value,
                    ..
                } => Some(#value),
            ))
        }
    }
}

/// Return unique payload enum fields across variants.
fn payload_enum_fields(variants: &[Variant], payload_names: &PayloadNames) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();

    for variant in variants {
        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let name = payload_names.tuple_label(variant);
                if names.insert(name.clone()) {
                    fields.push(Field {
                        name,
                        docs: variant.docs.clone(),
                        ty: ty.clone(),
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

/// Render one internal payload content variant.
fn render_content_variant(schema: &Schema, variant: &Variant) -> TokenStream {
    let variant_name = variant.ident();
    let docs = variant.docs();

    match &variant.payload {
        Payload::Unit => quote! {
            #docs
            #variant_name,
        },
        Payload::Tuple(ty) => {
            let ty = render_type(schema, ty);

            quote! {
                #docs
                #variant_name(#ty),
            }
        }
        Payload::Struct(fields) => {
            let fields = fields.iter().map(|field| {
                let docs = field.docs();
                let name = field.ident();
                let ty = render_type(schema, &field.ty);

                quote! {
                    #docs
                    #name: #ty,
                }
            });

            quote! {
                #docs
                #variant_name {
                    #(#fields)*
                },
            }
        }
    }
}

/// Render one WASM payload constructor.
fn render_payload_constructor(
    schema: &Schema,
    variant: &Variant,
    content_name: &proc_macro2::Ident,
) -> TokenStream {
    let method = variant.payload_method_ident();
    let js_name = variant.label();
    let variant_name = variant.ident();

    match &variant.payload {
        Payload::Unit => quote! {
            /// Create one payload variant.
            #[wasm_bindgen(js_name = #js_name)]
            pub fn #method() -> Self {
                Self {
                    content: #content_name::#variant_name,
                }
            }
        },
        Payload::Tuple(ty) => {
            let field = variant.payload_field_ident();
            let ty = render_type(schema, ty);

            quote! {
                /// Create one payload variant.
                #[wasm_bindgen(js_name = #js_name)]
                pub fn #method(#field: #ty) -> Self {
                    Self {
                        content: #content_name::#variant_name(#field),
                    }
                }
            }
        }
        Payload::Struct(fields) => {
            let arguments = fields.iter().map(|field| {
                let name = field.ident();
                let ty = render_type(schema, &field.ty);

                quote!(#name: #ty,)
            });
            let field_values = fields.iter().map(|field| {
                let name = field.ident();

                quote!(#name,)
            });

            quote! {
                /// Create one payload variant.
                #[wasm_bindgen(js_name = #js_name)]
                pub fn #method(#(#arguments)*) -> Self {
                    Self {
                        content: #content_name::#variant_name {
                            #(#field_values)*
                        },
                    }
                }
            }
        }
    }
}

/// Render one WASM payload enum to bridge conversion.
fn render_payload_enum_into_bridge(
    schema: &Schema,
    ty: &Item,
    variants: &[Variant],
    content_name: &proc_macro2::Ident,
) -> TokenStream {
    let name = ty.ident();
    let arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();

        match &variant.payload {
            Payload::Unit => quote! {
                #content_name::#variant_name => bridge::#name::#variant_name,
            },
            Payload::Tuple(ty) => {
                let value = render_into_bridge_value(schema, quote!(value), ty);

                quote! {
                    #content_name::#variant_name(value) => bridge::#name::#variant_name(#value),
                }
            }
            Payload::Struct(fields) => {
                let bindings = fields.iter().map(Field::ident);
                let field_values = fields.iter().map(|field| {
                    let name = field.ident();
                    if !field.ty.needs_wasm_into_bridge_conversion(schema) {
                        return quote!(#name,);
                    }

                    let value = render_into_bridge_value(schema, quote!(#name), &field.ty);

                    quote!(#name: #value,)
                });

                quote! {
                    #content_name::#variant_name { #(#bindings,)* } => bridge::#name::#variant_name {
                        #(#field_values)*
                    },
                }
            }
        }
    });

    quote! {
        impl #name {
            /// Convert this WASM payload enum into one bridge enum.
            pub(crate) fn into_bridge(self) -> bridge::#name {
                match self.content {
                    #(#arms)*
                }
            }
        }
    }
}

/// Render one bridge payload enum to WASM conversion.
fn render_payload_enum_from_bridge(
    schema: &Schema,
    ty: &Item,
    variants: &[Variant],
    content_name: &proc_macro2::Ident,
) -> TokenStream {
    let name = ty.ident();
    let arms = variants.iter().map(|variant| {
        let variant_name = variant.ident();

        match &variant.payload {
            Payload::Unit => quote! {
                bridge::#name::#variant_name => Self {
                    content: #content_name::#variant_name,
                },
            },
            Payload::Tuple(ty) => {
                let value = render_from_bridge_value(schema, quote!(value), ty);

                quote! {
                    bridge::#name::#variant_name(value) => Self {
                        content: #content_name::#variant_name(#value),
                    },
                }
            }
            Payload::Struct(fields) => {
                let bindings = fields.iter().map(Field::ident);
                let field_values = fields.iter().map(|field| {
                    let name = field.ident();
                    if !field.ty.needs_from_bridge_conversion(schema) {
                        return quote!(#name,);
                    }

                    let value = render_from_bridge_value(schema, quote!(#name), &field.ty);

                    quote!(#name: #value,)
                });

                quote! {
                    bridge::#name::#variant_name { #(#bindings,)* } => Self {
                        content: #content_name::#variant_name {
                            #(#field_values)*
                        },
                    },
                }
            }
        }
    });

    quote! {
        impl #name {
            /// Convert one bridge payload enum into one WASM payload enum.
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

/// Render one WASM type reference.
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

/// Render one value cloned out of a WASM value.
fn render_clone_value(value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Bool | Type::U8 | Type::U32 | Type::Usize => value,
        _ => quote!(#value.clone()),
    }
}

/// Render one value cloned from a borrowed WASM field.
fn render_borrowed_clone_value(value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Bool | Type::U8 | Type::U32 | Type::Usize => quote!(*#value),
        _ => quote!(#value.clone()),
    }
}

/// Render one value converted into bridge space.
fn render_into_bridge_value(schema: &Schema, value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Vec(ty) => {
            if !ty.needs_wasm_into_bridge_conversion(schema) {
                return value;
            }

            let item = render_into_bridge_value(schema, quote!(item), ty);

            quote!(#value.into_iter().map(|item| #item).collect())
        }
        Type::Option(ty) => {
            if !ty.needs_wasm_into_bridge_conversion(schema) {
                return value;
            }

            let item = render_into_bridge_value(schema, quote!(item), ty);

            quote!(#value.map(|item| #item))
        }
        Type::Named(name) if schema.is_unit_enum(name) => quote!(#value),
        Type::Named(_) => quote!(#value.into_bridge()),
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
