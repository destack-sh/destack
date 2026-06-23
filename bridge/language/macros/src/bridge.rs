use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{Fields, LitStr, parse_macro_input};

/// Schema attribute for C handle types.
const CAPI_HANDLE_ATTRIBUTE: &str = "capi_handle";
/// Schema attribute for copyable scalar types.
const COPY_ATTRIBUTE: &str = "copy";

/// Expand one bridge DTO marker.
pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let attributes = bridge_attributes(attribute);
    let input = parse_macro_input!(item as syn::Item);

    match attributes.and_then(|attributes| expand_item(attributes, input)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expand one bridge DTO item.
fn expand_item(
    attributes: Vec<String>,
    input: syn::Item,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input {
        syn::Item::Struct(item) => {
            let ident = &item.ident;
            let docs = docs(&item.attrs);
            let fields = fields(&item.fields)?;
            let attributes = schema_attributes(attributes, &item.attrs)?;

            Ok(quote! {
                #input

                impl destack_serde::Schema for #ident {
                    fn schema(
                        registry: &mut destack_serde::SchemaRegistry,
                    ) -> destack_serde::SchemaRef {
                        registry.register_with(
                            module_path!(),
                            stringify!(#ident),
                            vec![#(#docs.to_string()),*],
                            vec![#(#attributes.to_string()),*],
                            |registry| destack_serde::SchemaShape::Struct(#fields),
                        )
                    }
                }
            })
        }
        syn::Item::Enum(item) => {
            let ident = &item.ident;
            let docs = docs(&item.attrs);
            let variants = enum_variants(&item.variants)?;
            let attributes = schema_attributes(attributes, &item.attrs)?;

            Ok(quote! {
                #input

                impl destack_serde::Schema for #ident {
                    fn schema(
                        registry: &mut destack_serde::SchemaRegistry,
                    ) -> destack_serde::SchemaRef {
                        registry.register_with(
                            module_path!(),
                            stringify!(#ident),
                            vec![#(#docs.to_string()),*],
                            vec![#(#attributes.to_string()),*],
                            |registry| destack_serde::SchemaShape::Enum(vec![#(#variants),*]),
                        )
                    }
                }
            })
        }
        _ => Err(syn::Error::new_spanned(
            input,
            "bridge marker only supports structs and enums",
        )),
    }
}

/// Parse bridge marker attributes.
fn bridge_attributes(input: TokenStream) -> syn::Result<Vec<String>> {
    let attributes =
        syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated.parse(input)?;

    attributes
        .into_iter()
        .map(|path| {
            let Some(ident) = path.get_ident() else {
                return Err(syn::Error::new_spanned(
                    path,
                    "bridge attributes must be identifiers",
                ));
            };

            let attribute = ident.to_string();
            if attribute == CAPI_HANDLE_ATTRIBUTE {
                Ok(attribute)
            } else {
                Err(syn::Error::new_spanned(ident, "unknown bridge attribute"))
            }
        })
        .collect()
}

/// Build schema attributes for one bridge item.
fn schema_attributes(
    mut attributes: Vec<String>,
    rust_attributes: &[syn::Attribute],
) -> syn::Result<Vec<String>> {
    if derives(rust_attributes, "Copy")? {
        attributes.push(COPY_ATTRIBUTE.to_string());
    }

    Ok(attributes)
}

/// Return whether one item derives a trait.
fn derives(attributes: &[syn::Attribute], needle: &str) -> syn::Result<bool> {
    let mut is_derived = false;

    for attribute in attributes {
        if !attribute.path().is_ident("derive") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident(needle) {
                is_derived = true;
            }

            Ok(())
        })?;
    }

    Ok(is_derived)
}

/// Build schema fields from Rust fields.
fn fields(fields: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    match fields {
        Fields::Named(fields) => {
            let fields = fields
                .named
                .iter()
                .map(|field| {
                    let Some(ident) = field.ident.as_ref() else {
                        return Err(syn::Error::new_spanned(field, "expected named field"));
                    };
                    let name = ident.to_string();
                    let docs = docs(&field.attrs);
                    let ty = &field.ty;

                    Ok(quote! {
                        destack_serde::SchemaField {
                            name: #name.to_string(),
                            docs: vec![#(#docs.to_string()),*],
                            ty: <#ty as destack_serde::Schema>::schema(registry),
                        }
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote!(vec![#(#fields),*]))
        }
        Fields::Unnamed(fields) => {
            let fields = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let name = index.to_string();
                    let docs = docs(&field.attrs);
                    let ty = &field.ty;

                    quote! {
                        destack_serde::SchemaField {
                            name: #name.to_string(),
                            docs: vec![#(#docs.to_string()),*],
                            ty: <#ty as destack_serde::Schema>::schema(registry),
                        }
                    }
                })
                .collect::<Vec<_>>();

            Ok(quote!(vec![#(#fields),*]))
        }
        Fields::Unit => Ok(quote!(Vec::new())),
    }
}

/// Build schema variants from Rust enum variants.
fn enum_variants(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    variants
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
        .collect()
}

/// Build schema payload from Rust enum variant fields.
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

/// Clean one documentation literal.
fn clean_doc(value: &LitStr) -> String {
    value.value().trim().to_string()
}
