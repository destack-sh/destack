use proc_macro::TokenStream;

use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericParam, LitStr, parse_macro_input};

/// Expand one `Schema` derive invocation.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_input(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expand one parsed schema derive input.
fn expand_input(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = input.ident;
    let docs = docs(&input.attrs);
    let shape = shape(&input.data)?;
    let mut generics = input.generics;

    // require schema support for each generic type parameter
    for parameter in &mut generics.params {
        if let GenericParam::Type(parameter) = parameter {
            parameter
                .bounds
                .push(syn::parse_quote!(destack_serde::Schema));
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let name = ident.to_string();
    let module = quote!(module_path!());

    Ok(quote! {
        impl #impl_generics destack_serde::Schema for #ident #type_generics #where_clause {
            fn schema(registry: &mut destack_serde::SchemaRegistry) -> destack_serde::SchemaRef {
                registry.register(
                    #module,
                    #name,
                    vec![#(#docs.to_string()),*],
                    |registry| #shape,
                )
            }
        }
    })
}

/// Build the schema shape for one Rust item.
fn shape(data: &Data) -> syn::Result<proc_macro2::TokenStream> {
    match data {
        Data::Struct(data) => {
            let fields = fields(&data.fields)?;

            Ok(quote!(destack_serde::SchemaShape::Struct(#fields)))
        }
        Data::Enum(data) => {
            let variants = data
                .variants
                .iter()
                .map(|variant| {
                    let name = variant.ident.to_string();
                    let docs = docs(&variant.attrs);
                    let payload = payload(&variant.fields)?;

                    Ok(quote! {
                        destack_serde::SchemaVariant {
                            name: #name.to_string(),
                            docs: vec![#(#docs.to_string()),*],
                            payload: #payload,
                        }
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote!(destack_serde::SchemaShape::Enum(
                vec![#(#variants),*]
            )))
        }
        Data::Union(data) => Err(syn::Error::new(
            data.union_token.span,
            "Destack schemas do not support unions",
        )),
    }
}

/// Build struct fields for one schema shape.
fn fields(fields: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    match fields {
        Fields::Named(fields) => named_fields(fields),
        Fields::Unnamed(fields) => unnamed_fields(fields),
        Fields::Unit => Ok(quote!(Vec::new())),
    }
}

/// Build named struct fields.
fn named_fields(fields: &syn::FieldsNamed) -> syn::Result<proc_macro2::TokenStream> {
    let fields = fields
        .named
        .iter()
        .filter_map(|field| active_field(field).transpose())
        .map(|field| {
            let field = field?;
            let Some(ident) = field.ident.as_ref() else {
                return Err(syn::Error::new_spanned(field, "expected named field"));
            };
            let name = ident.to_string();

            field_schema(field, name)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote!(vec![#(#fields),*]))
}

/// Build unnamed struct fields.
fn unnamed_fields(fields: &syn::FieldsUnnamed) -> syn::Result<proc_macro2::TokenStream> {
    let fields = fields
        .unnamed
        .iter()
        .enumerate()
        .filter_map(|(index, field)| active_field(field).transpose().map(|field| (index, field)))
        .map(|(index, field)| {
            let field = field?;
            let name = index.to_string();

            field_schema(field, name)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote!(vec![#(#fields),*]))
}

/// Return one field unless it is skipped by serde.
fn active_field(field: &syn::Field) -> syn::Result<Option<&syn::Field>> {
    if is_serde_skip(&field.attrs)? {
        Ok(None)
    } else {
        Ok(Some(field))
    }
}

/// Build one schema field.
fn field_schema(field: &syn::Field, name: String) -> syn::Result<proc_macro2::TokenStream> {
    let docs = docs(&field.attrs);
    let ty = &field.ty;

    Ok(quote! {
        destack_serde::SchemaField {
            name: #name.to_string(),
            docs: vec![#(#docs.to_string()),*],
            ty: <#ty as destack_serde::Schema>::schema(registry),
        }
    })
}

/// Build enum variant payload schema.
fn payload(input: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    match input {
        Fields::Unit => Ok(quote!(destack_serde::SchemaPayload::Unit)),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let Some(field) = fields.unnamed.first() else {
                return Err(syn::Error::new_spanned(input, "expected tuple field"));
            };
            let ty = &field.ty;

            Ok(quote!(destack_serde::SchemaPayload::Tuple(
                <#ty as destack_serde::Schema>::schema(registry),
            )))
        }
        Fields::Unnamed(_) | Fields::Named(_) => {
            let fields = fields(input)?;

            Ok(quote!(destack_serde::SchemaPayload::Struct(#fields)))
        }
    }
}

/// Extract documentation strings.
fn docs(attributes: &[syn::Attribute]) -> Vec<String> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("doc"))
        .filter_map(|attribute| {
            let meta = attribute.meta.require_name_value().ok()?;
            let syn::Expr::Lit(expr) = &meta.value else {
                return None;
            };
            let syn::Lit::Str(value) = &expr.lit else {
                return None;
            };

            Some(clean_doc(value))
        })
        .collect()
}

/// Return whether one field is omitted from serde payloads.
fn is_serde_skip(attributes: &[syn::Attribute]) -> syn::Result<bool> {
    let mut is_skipped = false;

    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                is_skipped = true;
            }

            Ok(())
        })?;
    }

    Ok(is_skipped)
}

/// Clean one Rust documentation literal.
fn clean_doc(value: &LitStr) -> String {
    value.value().trim().to_string()
}
