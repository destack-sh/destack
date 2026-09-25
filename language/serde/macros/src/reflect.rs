use proc_macro::TokenStream;

use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericParam, LitStr, Token, parse_macro_input};

const CONTAINER_SERDE_OPTIONS: &[&str] = &[
    "bound",
    "crate",
    "default",
    "deny_unknown_fields",
    "expecting",
    "rename",
    "rename_all",
    "rename_all_fields",
    "transparent",
];
const VARIANT_SERDE_OPTIONS: &[&str] = &["alias", "rename", "skip"];
const FIELD_SERDE_OPTIONS: &[&str] = &["alias", "borrow", "bound", "default", "rename", "skip"];

/// Expand one `Reflect` derive invocation.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_input(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expand one parsed reflection derive input.
fn expand_input(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    validate_serde_options(&input.attrs, CONTAINER_SERDE_OPTIONS)?;

    let ident = input.ident;
    let docs = docs(&input.attrs);
    let module = module(&input.attrs)?;
    let ty = ty(&input.attrs, &input.data)?;
    let mut generics = input.generics;

    // require reflection support for each generic type parameter
    for parameter in &mut generics.params {
        if let GenericParam::Type(parameter) = parameter {
            parameter
                .bounds
                .push(syn::parse_quote!(tspp_serde::Reflect));
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let name = ident.to_string();

    Ok(quote! {
        impl #impl_generics tspp_serde::Reflect for #ident #type_generics #where_clause {
            fn reflect(schema: &mut tspp_serde::Schema) -> tspp_serde::Type {
                schema.declare(
                    #module,
                    #name,
                    vec![#(#docs.to_string()),*],
                    |schema| #ty,
                )
            }
        }
    })
}

/// Return the schema module for one reflected item.
fn module(attributes: &[syn::Attribute]) -> syn::Result<proc_macro2::TokenStream> {
    let mut module = None;

    for attribute in attributes {
        if !attribute.path().is_ident("reflect") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("module") {
                if module.is_some() {
                    return Err(meta.error("duplicate reflect module"));
                }

                let value = meta.value()?;
                module = Some(value.parse::<LitStr>()?);

                Ok(())
            } else {
                Err(meta.error("unsupported reflect attribute"))
            }
        })?;
    }

    if let Some(module) = module {
        Ok(quote!(#module))
    } else {
        Ok(quote!(module_path!()))
    }
}

/// Build the reflected type for one Rust item.
fn ty(attributes: &[syn::Attribute], data: &Data) -> syn::Result<proc_macro2::TokenStream> {
    match data {
        Data::Struct(data) if is_serde_transparent(attributes)? => transparent_type(&data.fields),
        Data::Struct(data) => struct_type(&data.fields),
        Data::Enum(data) => {
            let variants = data
                .variants
                .iter()
                .filter_map(|variant| active_variant(variant).transpose())
                .map(|variant| {
                    let variant = variant?;
                    let name = serde_name(&variant.attrs, variant.ident.to_string())?;
                    let docs = docs(&variant.attrs);
                    let payload = payload(&variant.fields)?;

                    Ok(quote! {
                        tspp_serde::Variant {
                            name: #name.to_string(),
                            docs: vec![#(#docs.to_string()),*],
                            payload: #payload,
                        }
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote!(tspp_serde::Type::Enum(vec![#(#variants),*])))
        }
        Data::Union(data) => Err(syn::Error::new(
            data.union_token.span,
            "TS++ schemas do not support unions",
        )),
    }
}

/// Build the underlying type for one transparent struct.
fn transparent_type(fields: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    let active = fields
        .iter()
        .filter_map(|field| active_field(field).transpose())
        .collect::<syn::Result<Vec<_>>>()?;
    let [field] = active.as_slice() else {
        return Err(syn::Error::new_spanned(
            fields,
            "transparent struct must have exactly one serialized field",
        ));
    };
    let ty = &field.ty;

    Ok(quote!(<#ty as tspp_serde::Reflect>::reflect(schema)))
}

/// Build one reflected struct type.
fn struct_type(fields: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    match fields {
        Fields::Named(fields) => {
            let fields = named_fields(fields)?;

            Ok(quote!(tspp_serde::Type::Struct(#fields)))
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let Some(field) = fields.unnamed.first() else {
                return Err(syn::Error::new_spanned(fields, "expected newtype field"));
            };
            let Some(field) = active_field(field)? else {
                return Ok(quote!(tspp_serde::Type::Tuple(Vec::new())));
            };
            let ty = &field.ty;

            Ok(quote!(<#ty as tspp_serde::Reflect>::reflect(schema)))
        }
        Fields::Unnamed(fields) => {
            let fields = unnamed_types(fields)?;

            Ok(quote!(tspp_serde::Type::Tuple(#fields)))
        }
        Fields::Unit => Ok(quote!(tspp_serde::Type::Struct(Vec::new()))),
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
            let name = serde_name(&field.attrs, field_name(ident))?;

            field_schema(field, name)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote!(vec![#(#fields),*]))
}

/// Build unnamed field types.
fn unnamed_types(fields: &syn::FieldsUnnamed) -> syn::Result<proc_macro2::TokenStream> {
    let fields = fields
        .unnamed
        .iter()
        .filter_map(|field| active_field(field).transpose())
        .map(|field| {
            let field = field?;
            let ty = &field.ty;

            Ok(quote!(<#ty as tspp_serde::Reflect>::reflect(schema)))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote!(vec![#(#fields),*]))
}

/// Return one variant unless it is skipped by serde.
fn active_variant(variant: &syn::Variant) -> syn::Result<Option<&syn::Variant>> {
    validate_serde_options(&variant.attrs, VARIANT_SERDE_OPTIONS)?;

    if is_serde_skip(&variant.attrs)? {
        Ok(None)
    } else {
        Ok(Some(variant))
    }
}

/// Return one field unless it is skipped by serde.
fn active_field(field: &syn::Field) -> syn::Result<Option<&syn::Field>> {
    validate_serde_options(&field.attrs, FIELD_SERDE_OPTIONS)?;

    if is_serde_skip(&field.attrs)? {
        Ok(None)
    } else {
        Ok(Some(field))
    }
}

/// Build one reflection field.
fn field_schema(field: &syn::Field, name: String) -> syn::Result<proc_macro2::TokenStream> {
    let docs = docs(&field.attrs);
    let ty = &field.ty;

    Ok(quote! {
        tspp_serde::Field {
            name: #name.to_string(),
            docs: vec![#(#docs.to_string()),*],
            ty: <#ty as tspp_serde::Reflect>::reflect(schema),
        }
    })
}

/// Build enum variant payload schema.
fn payload(input: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    match input {
        Fields::Unit => Ok(quote!(tspp_serde::Payload::Unit)),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let Some(field) = fields.unnamed.first() else {
                return Err(syn::Error::new_spanned(input, "expected newtype field"));
            };
            let Some(field) = active_field(field)? else {
                return Ok(quote!(tspp_serde::Payload::Unit));
            };
            let ty = &field.ty;

            Ok(quote!(tspp_serde::Payload::Value(
                <#ty as tspp_serde::Reflect>::reflect(schema),
            )))
        }
        Fields::Unnamed(fields) => {
            let fields = unnamed_types(fields)?;

            Ok(quote!(tspp_serde::Payload::Value(
                tspp_serde::Type::Tuple(#fields),
            )))
        }
        Fields::Named(fields) => {
            let fields = named_fields(fields)?;

            Ok(quote!(tspp_serde::Payload::Struct(#fields)))
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

            consume_serde_option(meta)
        })?;
    }

    Ok(is_skipped)
}

/// Return an explicit serde name or its Rust fallback.
fn serde_name(attributes: &[syn::Attribute], fallback: String) -> syn::Result<String> {
    let mut name = None;

    // read an explicit serialized name
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                name = Some(value.parse::<LitStr>()?.value());

                Ok(())
            } else {
                consume_serde_option(meta)
            }
        })?;
    }

    Ok(name.unwrap_or(fallback))
}

/// Reject serde options that are not represented by the reflected wire schema.
fn validate_serde_options(attributes: &[syn::Attribute], allowed: &[&str]) -> syn::Result<()> {
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            let name = meta
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_else(|| "<qualified>".to_string());
            if !allowed.iter().any(|allowed| *allowed == name) {
                return Err(meta.error(format!(
                    "serde option `{name}` is not represented by the TS++ wire schema"
                )));
            }

            consume_serde_option(meta)
        })?;
    }

    Ok(())
}

/// Return whether one item uses transparent serde representation.
fn is_serde_transparent(attributes: &[syn::Attribute]) -> syn::Result<bool> {
    let mut is_transparent = false;

    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") {
                is_transparent = true;
            }

            consume_serde_option(meta)
        })?;
    }

    Ok(is_transparent)
}

/// Consume one validated serde option payload.
fn consume_serde_option(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(Token![=]) {
        let value = meta.value()?;
        let _ = value.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        let _ = content.parse::<proc_macro2::TokenStream>()?;
    }

    Ok(())
}

/// Clean one Rust documentation literal.
fn clean_doc(value: &LitStr) -> String {
    value.value().trim().to_string()
}

/// Return the serialized field name for one Rust field identifier.
fn field_name(ident: &syn::Ident) -> String {
    let name = ident.to_string();

    name.strip_prefix("r#").unwrap_or(&name).to_string()
}
