use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derive the artifact `Image` trait for one structural image type.
#[proc_macro_derive(Image)]
pub fn derive_image(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_image_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Generate one `Image` impl for the chosen input type.
fn derive_image_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let (dehydrate_fields, rehydrate_fields) = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => {
                let field_names = fields
                    .named
                    .into_iter()
                    .map(|field| field.ident.expect("named field"))
                    .collect::<Vec<_>>();

                let dehydrate_fields = field_names.iter().map(|name| {
                    quote! {
                        crate::module::image::Field::dehydrate(
                            &mut image.#name,
                            context,
                        );
                    }
                });

                let rehydrate_fields = field_names.iter().map(|name| {
                    quote! {
                        crate::module::image::Field::rehydrate(
                            &mut image.#name,
                            context,
                        );
                    }
                });

                (
                    quote! { #(#dehydrate_fields)* },
                    quote! { #(#rehydrate_fields)* },
                )
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let dehydrate_fields = (0..field_count).map(|index| {
                    let index = syn::Index::from(index);

                    quote! {
                        crate::module::image::Field::dehydrate(
                            &mut image.#index,
                            context,
                        );
                    }
                });

                let rehydrate_fields = (0..field_count).map(|index| {
                    let index = syn::Index::from(index);

                    quote! {
                        crate::module::image::Field::rehydrate(
                            &mut image.#index,
                            context,
                        );
                    }
                });

                (
                    quote! { #(#dehydrate_fields)* },
                    quote! { #(#rehydrate_fields)* },
                )
            }
            Fields::Unit => (quote! {}, quote! {}),
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Image can only be derived for structs",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics crate::module::Image for #name #type_generics #where_clause {
            type Live = Self;

            fn dehydrate(
                live: &Self::Live,
                context: &mut impl crate::module::DehydrationContext,
            ) -> Self {
                let mut image = live.clone();

                #dehydrate_fields

                image
            }

            fn rehydrate(
                self,
                context: &mut impl crate::module::HydrationContext,
            ) -> Self::Live {
                let mut image = self;

                #rehydrate_fields

                image
            }
        }
    })
}
