use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, GenericParam, Ident, Index, parse_macro_input};

/// One generated recursive DIR operation.
#[derive(Clone, Copy)]
enum Traversal {
    /// Checked type ids rewritten in place.
    TypeFold,
    /// Checked type ids read without mutation.
    TypeVisit,
    /// Authored tree node ids rewritten in place.
    NodeFold,
    /// Checked selections.
    InstanceKeyVisit,
}

/// Expand one `TypeFold` derive invocation.
pub(crate) fn expand_type(input: TokenStream) -> TokenStream {
    expand(input, &[Traversal::TypeVisit, Traversal::TypeFold])
}

/// Expand one `NodeFold` derive invocation.
pub(crate) fn expand_node(input: TokenStream) -> TokenStream {
    expand(input, &[Traversal::NodeFold])
}

/// Expand one `InstanceKeyVisit` derive invocation.
pub(crate) fn expand_selection(input: TokenStream) -> TokenStream {
    expand(input, &[Traversal::InstanceKeyVisit])
}

/// Expand one recursive derive invocation.
fn expand(input: TokenStream, traversals: &[Traversal]) -> TokenStream {
    // parse the item once for every requested traversal
    let input = parse_macro_input!(input as DeriveInput);
    let mut output = TokenStream2::new();

    // generate each implementation from the same fields
    for traversal in traversals {
        match expand_input(&input, *traversal) {
            Ok(tokens) => output.extend(tokens),
            Err(error) => return error.to_compile_error().into(),
        }
    }

    output.into()
}

/// Expand one parsed recursive derive input.
fn expand_input(input: &DeriveInput, traversal: Traversal) -> syn::Result<TokenStream2> {
    // select the operation and its recursive field traversal
    let ident = &input.ident;
    let body = body(&input.data, traversal)?;
    let mut generics = input.generics.clone();
    let trait_ident = match traversal {
        Traversal::TypeFold => quote!(tspp_dir::TypeFold),
        Traversal::TypeVisit => quote!(tspp_dir::TypeVisit),
        Traversal::NodeFold => quote!(tspp_dir::NodeFold),
        Traversal::InstanceKeyVisit => quote!(tspp_dir::InstanceKeyVisit),
    };

    // require recursive support for each generic type parameter
    for parameter in &mut generics.params {
        if let GenericParam::Type(parameter) = parameter {
            parameter.bounds.push(syn::parse2(trait_ident.clone())?);
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let implementation = match traversal {
        Traversal::TypeVisit => quote! {
            impl #impl_generics tspp_dir::TypeVisit for #ident #type_generics #where_clause {
                fn visit_types<TypeVisitError>(
                    &self,
                    visit: &mut impl FnMut(tspp_dir::GlobalTypeId)
                        -> ::core::result::Result<(), TypeVisitError>,
                ) -> ::core::result::Result<(), TypeVisitError> {
                    #body

                    ::core::result::Result::Ok(())
                }
            }
        },
        Traversal::TypeFold => quote! {
            impl #impl_generics tspp_dir::TypeFold for #ident #type_generics #where_clause {
                fn map_types<TypeFoldError>(
                    &mut self,
                    map: &mut impl FnMut(
                        tspp_dir::GlobalTypeId,
                    ) -> ::core::result::Result<tspp_dir::GlobalTypeId, TypeFoldError>,
                ) -> ::core::result::Result<(), TypeFoldError> {
                    #body

                    ::core::result::Result::Ok(())
                }
            }
        },
        Traversal::NodeFold => quote! {
            impl #impl_generics tspp_dir::NodeFold for #ident #type_generics #where_clause {
                fn map_nodes<NodeFoldError>(
                    &mut self,
                    map: &mut impl FnMut(
                        tspp_dir::LocalNodeIdAny,
                    ) -> ::core::result::Result<u32, NodeFoldError>,
                ) -> ::core::result::Result<(), NodeFoldError> {
                    #body

                    ::core::result::Result::Ok(())
                }
            }
        },
        Traversal::InstanceKeyVisit => quote! {
            impl #impl_generics tspp_dir::InstanceKeyVisit
                for #ident #type_generics #where_clause
            {
                fn visit_instance_keys(
                    &self,
                    visit: &mut dyn FnMut(&tspp_dir::InstanceKey),
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
                    Traversal::TypeFold | Traversal::NodeFold => quote!(&mut #place),
                    Traversal::TypeVisit | Traversal::InstanceKeyVisit => quote!(&#place),
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
            "TS++ recursive derives do not support unions",
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
        Traversal::TypeFold => quote!(tspp_dir::TypeFold::map_types(#place, map)?;),
        Traversal::TypeVisit => quote!(tspp_dir::TypeVisit::visit_types(#place, visit)?;),
        Traversal::NodeFold => quote!(tspp_dir::NodeFold::map_nodes(#place, map)?;),
        Traversal::InstanceKeyVisit => {
            quote!(tspp_dir::InstanceKeyVisit::visit_instance_keys(#place, visit);)
        }
    }
}
