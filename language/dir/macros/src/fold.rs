use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, GenericParam, Ident, Index, parse_macro_input};

/// Expand one `TypeFold` derive invocation.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_input(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expand one parsed type fold derive input.
fn expand_input(input: DeriveInput) -> syn::Result<TokenStream2> {
    let ident = input.ident;
    let body = body(&input.data)?;
    let mut generics = input.generics;

    // require folding support for each generic type parameter
    for parameter in &mut generics.params {
        if let GenericParam::Type(parameter) = parameter {
            parameter
                .bounds
                .push(syn::parse_quote!(destack_dir::TypeFold));
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics destack_dir::TypeFold for #ident #type_generics #where_clause {
            fn map_types<TypeFoldError>(
                &mut self,
                map: &mut impl FnMut(
                    destack_dir::GlobalTypeId,
                ) -> ::core::result::Result<destack_dir::GlobalTypeId, TypeFoldError>,
            ) -> ::core::result::Result<(), TypeFoldError> {
                #body

                ::core::result::Result::Ok(())
            }
        }
    })
}

/// Build the fold body for one Rust item.
fn body(data: &Data) -> syn::Result<TokenStream2> {
    match data {
        Data::Struct(data) => {
            let folds = data.fields.iter().enumerate().map(|(index, field)| {
                let place = match &field.ident {
                    Some(ident) => quote!(self.#ident),
                    None => {
                        let index = Index::from(index);

                        quote!(self.#index)
                    }
                };

                fold(quote!(&mut #place))
            });

            Ok(quote!(#(#folds)*))
        }
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let names = names(&variant.fields);
                let bindings = bindings(&variant.fields, &names);
                let folds = names.iter().map(|name| fold(quote!(#name)));

                quote!(Self::#ident #bindings => { #(#folds)* })
            });

            Ok(quote!(match self { #(#arms)* }))
        }
        Data::Union(data) => Err(syn::Error::new(
            data.union_token.span,
            "Destack type folds do not support unions",
        )),
    }
}

/// Name every field of one enum variant.
fn names(fields: &Fields) -> Vec<Ident> {
    match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter_map(|field| field.ident.clone())
            .collect(),
        Fields::Unnamed(fields) => (0..fields.unnamed.len())
            .map(|index| format_ident!("field{index}"))
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

/// Bind every field of one enum variant by its name.
fn bindings(fields: &Fields, names: &[Ident]) -> TokenStream2 {
    match fields {
        Fields::Named(_) => quote!({ #(#names),* }),
        Fields::Unnamed(_) => quote!((#(#names),*)),
        Fields::Unit => quote!(),
    }
}

/// Fold one field place.
fn fold(place: TokenStream2) -> TokenStream2 {
    quote!(destack_dir::TypeFold::map_types(#place, map)?;)
}
