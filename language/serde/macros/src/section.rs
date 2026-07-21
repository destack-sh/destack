use proc_macro::TokenStream;

use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{Data, DeriveInput, Fields, Type, parenthesized, parse_macro_input};

/// One concrete type stored directly in a section image.
struct Entry {
    /// The derived type identifier.
    identifier: syn::Ident,
    /// The types stored inside the entry.
    field_types: Vec<Type>,
}

impl Entry {
    /// Parse one section entry derive input.
    fn parse(input: DeriveInput) -> syn::Result<Self> {
        // reject layouts that depend on type substitution
        if !input.generics.params.is_empty() {
            return Err(syn::Error::new_spanned(
                &input.generics,
                "generic section entries require an explicit SectionEntry implementation",
            ));
        }

        // require a representation that remains stable across builds
        if !Self::has_stable_repr(&input.attrs)? {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "section entries require repr(C), repr(transparent), or an integer repr",
            ));
        }

        // collect every type stored in the image
        let field_types = Self::field_types(input.data)?;

        Ok(Self {
            identifier: input.ident,
            field_types,
        })
    }

    /// Expand the section entry implementation.
    fn expand(self) -> proc_macro2::TokenStream {
        let identifier = self.identifier;
        let field_types = self.field_types;

        quote! {
            const _: () = {
                assert!(
                    ::core::mem::size_of::<#identifier>() != 0,
                    "section entries cannot be zero-sized",
                );
                assert!(
                    ::core::mem::align_of::<#identifier>() <= ::core::mem::align_of::<u128>(),
                    "section entry alignment exceeds image alignment",
                );
            };

            unsafe impl destack_core::SectionEntry for #identifier
            where
                #(#field_types: destack_core::SectionEntry,)*
            {}
        }
    }

    /// Return whether attributes select a stable representation.
    fn has_stable_repr(attributes: &[syn::Attribute]) -> syn::Result<bool> {
        let mut is_stable = false;

        // inspect every representation attribute
        for attribute in attributes {
            if !attribute.path().is_ident("repr") {
                continue;
            }

            // fold every representation argument into the layout decision
            attribute.parse_nested_meta(|meta| {
                let repr_is_stable = Self::parse_repr(meta)?;
                is_stable |= repr_is_stable;

                Ok(())
            })?;
        }

        Ok(is_stable)
    }

    /// Parse one representation argument.
    fn parse_repr(meta: ParseNestedMeta<'_>) -> syn::Result<bool> {
        let is_stable = meta.path.is_ident("C")
            || meta.path.is_ident("transparent")
            || meta.path.is_ident("u8")
            || meta.path.is_ident("u16")
            || meta.path.is_ident("u32")
            || meta.path.is_ident("u64")
            || meta.path.is_ident("i8")
            || meta.path.is_ident("i16")
            || meta.path.is_ident("i32")
            || meta.path.is_ident("i64");

        // consume optional alignment arguments
        if (meta.path.is_ident("align") || meta.path.is_ident("packed"))
            && meta.input.peek(syn::token::Paren)
        {
            let content;
            parenthesized!(content in meta.input);
            let _ = content.parse::<syn::LitInt>()?;
        }

        Ok(is_stable)
    }

    /// Collect every type stored by one data shape.
    fn field_types(data: Data) -> syn::Result<Vec<Type>> {
        let mut field_types = Vec::new();

        // flatten every stored field into one bound list
        match data {
            Data::Struct(data) => Self::append_field_types(data.fields, &mut field_types),
            Data::Enum(data) => {
                for variant in data.variants {
                    Self::append_field_types(variant.fields, &mut field_types);
                }
            }
            Data::Union(data) => {
                return Err(syn::Error::new_spanned(
                    data.union_token,
                    "section entries cannot contain unions",
                ));
            }
        }

        Ok(field_types)
    }

    /// Append the types stored by one field shape.
    fn append_field_types(fields: Fields, field_types: &mut Vec<Type>) {
        match fields {
            Fields::Named(fields) => {
                field_types.extend(fields.named.into_iter().map(|field| field.ty));
            }
            Fields::Unnamed(fields) => {
                field_types.extend(fields.unnamed.into_iter().map(|field| field.ty));
            }
            Fields::Unit => {}
        }
    }
}

/// Expand one `SectionEntry` derive invocation.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match Entry::parse(input) {
        Ok(entry) => entry.expand().into(),
        Err(error) => error.to_compile_error().into(),
    }
}
