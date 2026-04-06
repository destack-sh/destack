use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derive the `AdaptImage` trait for one structural type.
pub(crate) fn derive_adapt_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_adapt_image_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Generate one `AdaptImage` impl for the chosen input type.
fn derive_adapt_image_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let body = match input.data {
        Data::Struct(data) => derive_struct_body(data.fields)?,
        Data::Enum(data) => derive_enum_body(data.variants.into_iter().collect())?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                name,
                "AdaptImage can only be derived for structs and enums",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics destack_source::AdaptImage for #name #type_generics #where_clause {
            fn adapt_image(&mut self, adapter: &mut impl destack_source::ImageAdapter) {
                #body
            }
        }
    })
}

/// Generate the field-mapping body for one struct.
fn derive_struct_body(fields: Fields) -> syn::Result<proc_macro2::TokenStream> {
    let mappings = field_mappings(fields, quote!(self))?;

    Ok(quote! {
        #(#mappings)*
    })
}

/// Generate the field-mapping body for one enum.
fn derive_enum_body(variants: Vec<syn::Variant>) -> syn::Result<proc_macro2::TokenStream> {
    let arms = variants
        .into_iter()
        .map(|variant| {
            let name = variant.ident;

            match variant.fields {
                Fields::Named(fields) => {
                    let bindings = fields
                        .named
                        .iter()
                        .map(|field| field.ident.clone().expect("named field"))
                        .collect::<Vec<_>>();
                    let mappings = bindings.iter().map(|name| {
                        quote! {
                            destack_source::AdaptImage::adapt_image(#name, adapter);
                        }
                    });

                    Ok(quote! {
                        Self::#name { #(#bindings),* } => {
                            #(#mappings)*
                        }
                    })
                }
                Fields::Unnamed(fields) => {
                    let bindings = (0..fields.unnamed.len())
                        .map(|index| syn::Ident::new(&format!("field_{index}"), name.span()))
                        .collect::<Vec<_>>();
                    let mappings = bindings.iter().map(|binding| {
                        quote! {
                            destack_source::AdaptImage::adapt_image(#binding, adapter);
                        }
                    });

                    Ok(quote! {
                        Self::#name(#(#bindings),*) => {
                            #(#mappings)*
                        }
                    })
                }
                Fields::Unit => Ok(quote! {
                    Self::#name => {}
                }),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        match self {
            #(#arms),*
        }
    })
}

/// Generate mapping statements for one field set.
fn field_mappings(
    fields: Fields,
    target: proc_macro2::TokenStream,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    match fields {
        Fields::Named(fields) => Ok(fields
            .named
            .into_iter()
            .map(|field| {
                let name = field.ident.expect("named field");

                quote! {
                    destack_source::AdaptImage::adapt_image(&mut #target.#name, adapter);
                }
            })
            .collect()),
        Fields::Unnamed(fields) => Ok(fields
            .unnamed
            .into_iter()
            .enumerate()
            .map(|(index, _)| {
                let index = syn::Index::from(index);

                quote! {
                    destack_source::AdaptImage::adapt_image(&mut #target.#index, adapter);
                }
            })
            .collect()),
        Fields::Unit => Ok(Vec::new()),
    }
}
