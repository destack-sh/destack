use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, LitStr, Meta, Result, Token, Visibility, parse_macro_input};

/// Input for the declare_pass! macro.
struct DeclarePassInput {
    /// Doc comments and other attributes.
    attrs: Vec<Attribute>,
    /// The #[pass(...)] attribute with configuration.
    pass_attr: PassAttr,
    /// Visibility (usually `pub`).
    visibility: Visibility,
    /// The struct name (e.g., ConstantFold).
    name: Ident,
    /// Short description string.
    description: String,
}

/// Parsed #[pass(...)] attribute.
struct PassAttr {
    /// The pass ID (e.g., "constant-fold").
    id: String,
    // NOTE #Incomplete: add requires/invalidates when we have analysis caching
}

impl Parse for DeclarePassInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // parse all outer attributes (doc comments and #[pass(...)])
        let attrs = input.call(Attribute::parse_outer)?;

        // find and parse the #[pass(...)] attribute
        let pass_attr = parse_pass_attr(&attrs)?;

        // filter out the #[pass(...)] attribute from attrs (keep doc comments)
        let attrs: Vec<Attribute> = attrs
            .into_iter()
            .filter(|a| !a.path().is_ident("pass"))
            .collect();

        // parse visibility
        let vis: Visibility = input.parse()?;

        // parse struct name
        let name: Ident = input.parse()?;

        // parse comma
        input.parse::<Token![,]>()?;

        // parse description string
        let description: LitStr = input.parse()?;

        Ok(DeclarePassInput {
            attrs,
            pass_attr,
            visibility: vis,
            name,
            description: description.value(),
        })
    }
}

/// Parse the #[pass(...)] attribute.
fn parse_pass_attr(attrs: &[Attribute]) -> Result<PassAttr> {
    let pass_attr = attrs
        .iter()
        .find(|a| a.path().is_ident("pass"))
        .ok_or_else(|| syn::Error::new_spanned(&attrs[0], "missing #[pass(...)] attribute"))?;

    let mut id: Option<String> = None;

    pass_attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("id") {
            meta.input.parse::<Token![=]>()?;
            let lit: LitStr = meta.input.parse()?;
            id = Some(lit.value());
        }
        // NOTE #Incomplete: parse requires/invalidates when we have analysis caching
        Ok(())
    })?;

    let id =
        id.ok_or_else(|| syn::Error::new_spanned(pass_attr, "missing `id` in #[pass(...)]"))?;

    Ok(PassAttr { id })
}

/// Extract doc comment text from attributes.
#[allow(dead_code)]
fn extract_docs(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(nv) = &attr.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
            {
                return Some(s.value());
            }
            None
        })
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn declare_pass_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeclarePassInput);

    let DeclarePassInput {
        attrs,
        pass_attr,
        visibility: vis,
        name,
        description,
    } = input;

    let PassAttr { id } = pass_attr;

    // generate static name: ConstantFold -> CONSTANT_FOLD
    let static_name = format_ident!("{}", to_screaming_snake_case(&name.to_string()));

    let expanded = quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy)]
        #vis struct #name;

        #vis static #static_name: &crate::optimize::PassMetadata = &crate::optimize::PassMetadata {
            id: #id,
            name: stringify!(#name),
            description: #description,
        };

        impl #name {
            /// Get the pass metadata.
            pub const fn metadata() -> &'static crate::optimize::PassMetadata {
                #static_name
            }
        }
    };

    TokenStream::from(expanded)
}

/// Convert PascalCase to SCREAMING_SNAKE_CASE.
fn to_screaming_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c);
        } else {
            result.push(c.to_ascii_uppercase());
        }
    }
    result
}
