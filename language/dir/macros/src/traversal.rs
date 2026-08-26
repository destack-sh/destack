use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, GenericParam, Ident, Index, parse_macro_input};

/// One generated recursive DIR operation.
#[derive(Clone, Copy)]
enum Traversal {
    /// Checked type ids.
    Type,
    /// Authored tree node ids.
    Node,
    /// Checked selections.
    InstanceKey,
}

/// Expand one `TypeFold` derive invocation.
pub(crate) fn expand_type(input: TokenStream) -> TokenStream {
    expand(input, Traversal::Type)
}

/// Expand one `NodeFold` derive invocation.
pub(crate) fn expand_node(input: TokenStream) -> TokenStream {
    expand(input, Traversal::Node)
}

/// Expand one `InstanceKeyVisit` derive invocation.
pub(crate) fn expand_selection(input: TokenStream) -> TokenStream {
    expand(input, Traversal::InstanceKey)
}

/// Expand one recursive derive invocation.
fn expand(input: TokenStream, traversal: Traversal) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_input(input, traversal) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Expand one parsed recursive derive input.
fn expand_input(input: DeriveInput, traversal: Traversal) -> syn::Result<TokenStream2> {
    let ident = input.ident;
    let body = body(&input.data, traversal)?;
    let mut generics = input.generics;
    let trait_ident = match traversal {
        Traversal::Type => quote!(destack_dir::TypeFold),
        Traversal::Node => quote!(destack_dir::NodeFold),
        Traversal::InstanceKey => quote!(destack_dir::InstanceKeyVisit),
    };

    // require recursive support for each generic type parameter
    for parameter in &mut generics.params {
        if let GenericParam::Type(parameter) = parameter {
            parameter.bounds.push(syn::parse2(trait_ident.clone())?);
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let implementation = match traversal {
        Traversal::Type => quote! {
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
        },
        Traversal::Node => quote! {
            impl #impl_generics destack_dir::NodeFold for #ident #type_generics #where_clause {
                fn map_nodes<NodeFoldError>(
                    &mut self,
                    map: &mut impl FnMut(
                        destack_dir::LocalNodeIdAny,
                    ) -> ::core::result::Result<u32, NodeFoldError>,
                ) -> ::core::result::Result<(), NodeFoldError> {
                    #body

                    ::core::result::Result::Ok(())
                }
            }
        },
        Traversal::InstanceKey => quote! {
            impl #impl_generics destack_dir::InstanceKeyVisit
                for #ident #type_generics #where_clause
            {
                fn visit_instance_keys(
                    &self,
                    visit: &mut dyn FnMut(&destack_dir::InstanceKey),
                ) {
                    #body
                }
            }
        },
    };

    Ok(implementation)
}

/// Build the recursive body for one Rust item.
fn body(data: &Data, traversal: Traversal) -> syn::Result<TokenStream2> {
    match data {
        Data::Struct(data) => {
            let calls = data.fields.iter().enumerate().map(|(index, field)| {
                let place = match &field.ident {
                    Some(ident) => quote!(self.#ident),
                    None => {
                        let index = Index::from(index);

                        quote!(self.#index)
                    }
                };
                let place = match traversal {
                    Traversal::Type | Traversal::Node => quote!(&mut #place),
                    Traversal::InstanceKey => quote!(&#place),
                };

                apply(place, traversal)
            });

            Ok(quote!(#(#calls)*))
        }
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let names = names(&variant.fields);
                let bindings = bindings(&variant.fields, &names);
                let calls = names.iter().map(|name| apply(quote!(#name), traversal));

                quote!(Self::#ident #bindings => { #(#calls)* })
            });

            Ok(quote!(match self { #(#arms)* }))
        }
        Data::Union(data) => Err(syn::Error::new(
            data.union_token.span,
            "Destack recursive derives do not support unions",
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

/// Apply one recursive operation to a field place.
fn apply(place: TokenStream2, traversal: Traversal) -> TokenStream2 {
    match traversal {
        Traversal::Type => quote!(destack_dir::TypeFold::map_types(#place, map)?;),
        Traversal::Node => quote!(destack_dir::NodeFold::map_nodes(#place, map)?;),
        Traversal::InstanceKey => {
            quote!(destack_dir::InstanceKeyVisit::visit_instance_keys(#place, visit);)
        }
    }
}
