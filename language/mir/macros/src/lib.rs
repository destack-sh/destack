use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, GenericParam, Index, parse_macro_input};

/// Derive one structural MIR type fold.
#[proc_macro_derive(TypeFold)]
pub fn type_fold(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Expand one MIR type fold.
fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let ident = input.ident;
    let body = fold_body(&input.data)?;
    let mut generics = input.generics;

    // require folds for generic fields
    for parameter in &mut generics.params {
        if let GenericParam::Type(parameter) = parameter {
            parameter
                .bounds
                .push(syn::parse_quote!(destack_mir::TypeFold));
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics destack_mir::TypeFold for #ident #type_generics #where_clause {
            fn map_types(
                &mut self,
                map: &mut impl FnMut(destack_mir::TypeId) -> destack_mir::TypeId,
            ) {
                #body
            }
        }
    })
}

/// Build one fold body.
fn fold_body(data: &Data) -> syn::Result<TokenStream2> {
    match data {
        Data::Struct(data) => {
            let calls = data.fields.iter().enumerate().map(|(index, field)| {
                let member = match &field.ident {
                    Some(ident) => quote!(#ident),
                    None => {
                        let index = Index::from(index);

                        quote!(#index)
                    }
                };

                quote!(destack_mir::TypeFold::map_types(&mut self.#member, map);)
            });

            Ok(quote!(#(#calls)*))
        }
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let names = field_names(&variant.fields);
                let pattern = field_pattern(&variant.fields, &names);
                let calls = names
                    .iter()
                    .map(|name| quote!(destack_mir::TypeFold::map_types(#name, map);));

                quote!(Self::#ident #pattern => { #(#calls)* })
            });

            Ok(quote!(match self { #(#arms)* }))
        }
        Data::Union(data) => Err(syn::Error::new(
            data.union_token.span,
            "MIR type folds do not support unions",
        )),
    }
}

/// Name every field of one enum variant.
fn field_names(fields: &Fields) -> Vec<syn::Ident> {
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

/// Build one enum variant field pattern.
fn field_pattern(fields: &Fields, names: &[syn::Ident]) -> TokenStream2 {
    match fields {
        Fields::Named(_) => quote!({ #(#names),* }),
        Fields::Unnamed(_) => quote!((#(#names),*)),
        Fields::Unit => quote!(),
    }
}
