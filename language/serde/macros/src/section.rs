use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, Index, Type, parenthesized, parse_macro_input};

const INTEGER_REPRESENTATIONS: &[&str] = &["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];

/// One concrete type stored directly in a section image.
enum Entry {
    /// One structure with stable field layout.
    Struct {
        /// The derived type identifier.
        identifier: Ident,
        /// The types stored inside the entry.
        field_types: Vec<Type>,
        /// The concrete stored fields.
        fields: Fields,
    },
    /// One tagged enum with a stable discriminant.
    Enum {
        /// The derived type identifier.
        identifier: Ident,
        /// The types stored inside the entry.
        field_types: Vec<Type>,
        /// The concrete enum variants.
        variants: syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
        /// The integer representation used by the enum tag.
        tag: Ident,
        /// Whether every variant shares one C union payload offset.
        is_c: bool,
    },
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
        let representation = Representation::parse(&input.attrs)?;
        if !representation.is_stable {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "section entries require repr(C), repr(transparent), or an integer repr",
            ));
        }

        // construct only valid structure and enum representations
        match input.data {
            Data::Struct(data) => {
                let mut field_types = Vec::new();
                Self::append_field_types(&data.fields, &mut field_types);

                Ok(Self::Struct {
                    identifier: input.ident,
                    field_types,
                    fields: data.fields,
                })
            }
            Data::Enum(data) => {
                let mut field_types = Vec::new();
                for variant in &data.variants {
                    Self::append_field_types(&variant.fields, &mut field_types);
                }

                let tag = representation.tag.ok_or_else(|| {
                    syn::Error::new_spanned(
                        &input.ident,
                        "section entry enums require an explicit integer repr",
                    )
                })?;
                Ok(Self::Enum {
                    identifier: input.ident,
                    field_types,
                    variants: data.variants,
                    tag,
                    is_c: representation.is_c,
                })
            }
            Data::Union(data) => Err(syn::Error::new_spanned(
                data.union_token,
                "section entries cannot contain unions",
            )),
        }
    }

    /// Expand the section entry implementation.
    fn expand(self) -> proc_macro2::TokenStream {
        let (identifier, field_types, validation, needs_validation) = match self {
            Self::Struct {
                identifier,
                field_types,
                fields,
            } => {
                let validation = Self::expand_struct_validation(&identifier, &fields);
                let needs_validation = quote!(
                    false #(|| <#field_types as tspp_core::SectionEntry>::NEEDS_VALIDATION)*
                );

                (identifier, field_types, validation, needs_validation)
            }
            Self::Enum {
                identifier,
                field_types,
                variants,
                tag,
                is_c,
            } => {
                let validation = Self::expand_enum_validation(tag, &variants, is_c);

                (identifier, field_types, validation, quote!(true))
            }
        };

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

            unsafe impl tspp_core::SectionEntry for #identifier
            where
                #(#field_types: tspp_core::SectionEntry,)*
            {
                const NEEDS_VALIDATION: bool = #needs_validation;

                fn validate(
                    bytes: &[u8],
                    loader: tspp_core::SectionLoader<'_>,
                ) -> ::core::result::Result<(), tspp_core::SectionImageError> {
                    if bytes.len() != ::core::mem::size_of::<Self>() {
                        return Err(tspp_core::SectionImageError::InvalidEntry);
                    }

                    #validation
                }
            }
        }
    }

    /// Append the types stored by one field shape.
    fn append_field_types(fields: &Fields, field_types: &mut Vec<Type>) {
        match fields {
            Fields::Named(fields) => {
                field_types.extend(fields.named.iter().map(|field| field.ty.clone()));
            }
            Fields::Unnamed(fields) => {
                field_types.extend(fields.unnamed.iter().map(|field| field.ty.clone()));
            }
            Fields::Unit => {}
        }
    }

    /// Expand byte validation for one structure.
    fn expand_struct_validation(identifier: &Ident, fields: &Fields) -> proc_macro2::TokenStream {
        let checks = fields.iter().enumerate().map(|(index, field)| {
            let member = field
                .ident
                .clone()
                .map(syn::Member::Named)
                .unwrap_or_else(|| syn::Member::Unnamed(Index::from(index)));
            let ty = &field.ty;

            quote! {
                {
                    let offset = ::core::mem::offset_of!(#identifier, #member);
                    let end = offset + ::core::mem::size_of::<#ty>();
                    if <#ty as tspp_core::SectionEntry>::NEEDS_VALIDATION {
                        <#ty as tspp_core::SectionEntry>::validate(
                            &bytes[offset..end],
                            loader,
                        )?;
                    }
                }
            }
        });

        quote! {
            #(#checks)*

            Ok(())
        }
    }

    /// Expand byte validation for one tagged enum.
    fn expand_enum_validation(
        tag: Ident,
        variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
        is_c: bool,
    ) -> proc_macro2::TokenStream {
        let tags = Self::expand_tags(&tag, variants);
        let tag_read = quote! {
            let mut tag_bytes = [0; ::core::mem::size_of::<#tag>()];
            tag_bytes.copy_from_slice(&bytes[..::core::mem::size_of::<#tag>()]);
            let tag = #tag::from_ne_bytes(tag_bytes);
        };
        let has_payload = variants.iter().any(|variant| !variant.fields.is_empty());

        // unit enums only require one known discriminant
        if !has_payload {
            let arms = variants.iter().enumerate().map(|(index, _)| {
                let tag_name = format_ident!("TAG_{index}");

                quote!(#tag_name => Ok(()))
            });

            return quote! {
                #tags
                #tag_read

                match tag {
                    #(#arms,)*
                    _ => Err(tspp_core::SectionImageError::InvalidEntry),
                }
            };
        }

        let payloads = variants
            .iter()
            .enumerate()
            .filter(|(_, variant)| !variant.fields.is_empty())
            .map(|(index, variant)| Self::expand_payload(index, &variant.fields));
        let representation = if is_c {
            let union_fields = variants
                .iter()
                .enumerate()
                .filter(|(_, variant)| !variant.fields.is_empty())
                .map(|(index, _)| {
                    let field = format_ident!("variant_{index}");
                    let payload = format_ident!("Payload{index}");

                    quote!(#field: ::core::mem::ManuallyDrop<#payload>)
                });

            quote! {
                #[repr(C)]
                union Payload {
                    #(#union_fields,)*
                }
                #[repr(C)]
                struct Representation {
                    tag: #tag,
                    payload: Payload,
                }
            }
        } else {
            let variants = variants
                .iter()
                .enumerate()
                .filter(|(_, variant)| !variant.fields.is_empty())
                .map(|(index, _)| {
                    let representation = format_ident!("Representation{index}");
                    let payload = format_ident!("Payload{index}");

                    quote! {
                        #[repr(C)]
                        struct #representation {
                            tag: #tag,
                            payload: #payload,
                        }
                    }
                });

            quote!(#(#variants)*)
        };
        let arms = variants.iter().enumerate().map(|(index, variant)| {
            let tag_name = format_ident!("TAG_{index}");
            let payload_offset = if is_c {
                quote!(::core::mem::offset_of!(Representation, payload))
            } else {
                let representation = format_ident!("Representation{index}");

                quote!(::core::mem::offset_of!(#representation, payload))
            };
            let checks = variant.fields.iter().enumerate().map(|(field_index, field)| {
                let payload = format_ident!("Payload{index}");
                let member = field
                    .ident
                    .clone()
                    .map(syn::Member::Named)
                    .unwrap_or_else(|| syn::Member::Unnamed(Index::from(field_index)));
                let ty = &field.ty;

                quote! {
                    {
                        let offset = #payload_offset + ::core::mem::offset_of!(#payload, #member);
                        let end = offset + ::core::mem::size_of::<#ty>();
                        if <#ty as tspp_core::SectionEntry>::NEEDS_VALIDATION {
                            <#ty as tspp_core::SectionEntry>::validate(
                                &bytes[offset..end],
                                loader,
                            )?;
                        }
                    }
                }
            });

            quote! {
                #tag_name => {
                    #(#checks)*

                    Ok(())
                }
            }
        });

        quote! {
            #tags
            #(#payloads)*
            #representation

            #tag_read
            match tag {
                #(#arms,)*
                _ => Err(tspp_core::SectionImageError::InvalidEntry),
            }
        }
    }

    /// Expand one enum payload representation.
    fn expand_payload(index: usize, fields: &Fields) -> proc_macro2::TokenStream {
        let payload = format_ident!("Payload{index}");

        match fields {
            Fields::Named(fields) => {
                let entries = fields.named.iter().map(|field| {
                    let name = &field.ident;
                    let ty = &field.ty;

                    quote!(#name: #ty)
                });

                quote!(#[repr(C)] struct #payload { #(#entries,)* })
            }
            Fields::Unnamed(fields) => {
                let entries = fields.unnamed.iter().map(|field| &field.ty);

                quote!(#[repr(C)] struct #payload(#(#entries,)*);)
            }
            Fields::Unit => unreachable!("unit variants do not have payload representations"),
        }
    }

    /// Expand stable constants for enum discriminants.
    fn expand_tags(
        tag: &Ident,
        variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    ) -> proc_macro2::TokenStream {
        let constants = variants.iter().enumerate().map(|(index, variant)| {
            let name = format_ident!("TAG_{index}");
            if let Some((_, value)) = &variant.discriminant {
                quote!(const #name: #tag = #value;)
            } else if index == 0 {
                quote!(const #name: #tag = 0;)
            } else {
                let previous = format_ident!("TAG_{}", index - 1);

                quote!(const #name: #tag = #previous + 1;)
            }
        });

        quote!(#(#constants)*)
    }
}

/// Stable representation selected for one section entry.
#[derive(Default)]
struct Representation {
    /// Whether the representation has stable field layout.
    is_stable: bool,
    /// Whether the representation follows C aggregate layout.
    is_c: bool,
    /// Explicit integer representation for an enum tag.
    tag: Option<Ident>,
}

impl Representation {
    /// Parse stable representation attributes.
    fn parse(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        let mut representation = Self::default();

        // inspect every representation attribute
        for attribute in attributes {
            if !attribute.path().is_ident("repr") {
                continue;
            }

            attribute.parse_nested_meta(|meta| representation.parse_argument(meta))?;
        }

        Ok(representation)
    }

    /// Parse one representation argument.
    fn parse_argument(&mut self, meta: ParseNestedMeta<'_>) -> syn::Result<()> {
        if meta.path.is_ident("C") || meta.path.is_ident("transparent") {
            self.is_stable = true;
        }
        if meta.path.is_ident("C") {
            self.is_c = true;
        }
        for name in INTEGER_REPRESENTATIONS {
            if meta.path.is_ident(name) {
                self.is_stable = true;
                self.tag = Some(Ident::new(name, meta.path.span()));
            }
        }

        // consume optional alignment arguments
        if (meta.path.is_ident("align") || meta.path.is_ident("packed"))
            && meta.input.peek(syn::token::Paren)
        {
            let content;
            parenthesized!(content in meta.input);
            let _ = content.parse::<syn::LitInt>()?;
        }

        Ok(())
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
