use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, LitStr, Meta, Result, Token, Visibility, parse_macro_input};

/// Input for the declare_lint! macro.
struct DeclareLintInput {
    /// Doc comments and other attributes.
    attrs: Vec<Attribute>,
    /// The #[lint(...)] attribute with configuration.
    lint_attr: LintAttr,
    /// Visibility (usually `pub`).
    visibility: Visibility,
    /// The struct name (e.g., NoDebugger).
    name: Ident,
    /// Short description string.
    description: String,
}

/// Parsed #[lint(...)] attribute.
struct LintAttr {
    /// The rule ID (e.g., "no-debugger").
    id: String,
    /// The rule code (e.g., "LC001").
    code: String,
    /// The rule category (e.g., Correctness).
    category: Ident,
    /// The rule level (e.g., Dir).
    level: Ident,
    /// The rule scope (e.g., Module).
    scope: Option<Ident>,
    /// Whether the rule can provide fixes (Always, Sometimes, No).
    fixable: Ident,
    /// Whether the rule is recommended (Always, Strict, Off).
    recommended: Ident,
    /// The stability of the rule (Stable, Experimental).
    stability: Ident,
    /// The URL to the rule documentation.
    docs_url: Option<String>,
}

impl Parse for DeclareLintInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // parse all outer attributes (doc comments and #[lint(...)])
        let attrs = input.call(Attribute::parse_outer)?;

        // find and parse the #[lint(...)] attribute
        let lint_attr = parse_lint_attr(&attrs)?;

        // filter out the #[lint(...)] attribute from attrs (keep doc comments)
        let attrs: Vec<Attribute> = attrs
            .into_iter()
            .filter(|a| !a.path().is_ident("lint"))
            .collect();

        // parse visibility
        let vis: Visibility = input.parse()?;

        // parse struct name
        let name: Ident = input.parse()?;

        // parse comma
        input.parse::<Token![,]>()?;

        // parse description string
        let description: LitStr = input.parse()?;

        Ok(DeclareLintInput {
            attrs,
            lint_attr,
            visibility: vis,
            name,
            description: description.value(),
        })
    }
}

/// Parse the #[lint(...)] attribute.
fn parse_lint_attr(attrs: &[Attribute]) -> Result<LintAttr> {
    let lint_attr = attrs
        .iter()
        .find(|a| a.path().is_ident("lint"))
        .ok_or_else(|| syn::Error::new_spanned(&attrs[0], "missing #[lint(...)] attribute"))?;

    let mut id: Option<String> = None;
    let mut code: Option<String> = None;
    let mut category = None;
    let mut level = None;
    let mut scope = None;
    let mut fixable = None;
    let mut recommended = None;
    let mut stability = None;
    let mut docs_url = None;

    lint_attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("id") {
            meta.input.parse::<Token![=]>()?;
            let lit: LitStr = meta.input.parse()?;
            id = Some(lit.value());
        } else if meta.path.is_ident("code") {
            meta.input.parse::<Token![=]>()?;
            let lit: LitStr = meta.input.parse()?;
            code = Some(lit.value());
        } else if meta.path.is_ident("category") {
            meta.input.parse::<Token![=]>()?;
            let ident: Ident = meta.input.parse()?;
            category = Some(ident);
        } else if meta.path.is_ident("level") {
            meta.input.parse::<Token![=]>()?;
            let ident: Ident = meta.input.parse()?;
            level = Some(ident);
        } else if meta.path.is_ident("scope") {
            meta.input.parse::<Token![=]>()?;
            let ident: Ident = meta.input.parse()?;
            scope = Some(ident);
        } else if meta.path.is_ident("fixable") {
            meta.input.parse::<Token![=]>()?;
            let ident: Ident = meta.input.parse()?;
            fixable = Some(ident);
        } else if meta.path.is_ident("recommended") {
            meta.input.parse::<Token![=]>()?;
            let ident: Ident = meta.input.parse()?;
            recommended = Some(ident);
        } else if meta.path.is_ident("stability") {
            meta.input.parse::<Token![=]>()?;
            let ident: Ident = meta.input.parse()?;
            stability = Some(ident);
        } else if meta.path.is_ident("docs") {
            meta.input.parse::<Token![=]>()?;
            let lit: LitStr = meta.input.parse()?;
            docs_url = Some(lit.value());
        }
        Ok(())
    })?;

    let id =
        id.ok_or_else(|| syn::Error::new_spanned(lint_attr, "missing `id` in #[lint(...)]"))?;
    let code =
        code.ok_or_else(|| syn::Error::new_spanned(lint_attr, "missing `code` in #[lint(...)]"))?;
    let category = category
        .ok_or_else(|| syn::Error::new_spanned(lint_attr, "missing `category` in #[lint(...)]"))?;
    let level = level
        .ok_or_else(|| syn::Error::new_spanned(lint_attr, "missing `level` in #[lint(...)]"))?;
    let fixable = fixable
        .ok_or_else(|| syn::Error::new_spanned(lint_attr, "missing `fixable` in #[lint(...)]"))?;
    let recommended = recommended.ok_or_else(|| {
        syn::Error::new_spanned(lint_attr, "missing `recommended` in #[lint(...)]")
    })?;
    let stability = stability
        .ok_or_else(|| syn::Error::new_spanned(lint_attr, "missing `stability` in #[lint(...)]"))?;

    Ok(LintAttr {
        id,
        code,
        category,
        level,
        scope,
        fixable,
        recommended,
        stability,
        docs_url,
    })
}

/// Extract doc comment text from attributes.
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

pub(crate) fn declare_lint_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeclareLintInput);

    let DeclareLintInput {
        attrs,
        lint_attr,
        visibility: vis,
        name,
        description,
    } = input;

    let LintAttr {
        id,
        code,
        category,
        level,
        scope,
        fixable,
        recommended,
        stability,
        docs_url,
    } = lint_attr;

    // generate static name: NoDebugger -> NO_DEBUGGER
    let static_name = format_ident!("{}", to_screaming_snake_case(&name.to_string()));

    // scope defaults to Module
    let scope = scope.unwrap_or_else(|| format_ident!("Module"));

    // docs_url
    let docs_url_tokens = match docs_url {
        Some(url) => quote! { Some(#url) },
        None => quote! { None },
    };

    // extract long description from doc comments
    let _long_description = extract_docs(&attrs);

    // default severity based on category (correctness = error, others = warning)
    let default_severity = match category.to_string().as_str() {
        "Correctness" => quote! { destack_source::DiagnosticSeverity::Error },
        _ => quote! { destack_source::DiagnosticSeverity::Warning },
    };

    let expanded = quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy)]
        #vis struct #name;

        #vis static #static_name: &crate::linter::LintMeta = &crate::linter::LintMeta {
            id: #id,
            code: #code,
            name: stringify!(#name),
            description: #description,
            category: crate::linter::LintCategory::#category,
            default_severity: #default_severity,
            docs_url: #docs_url_tokens,
            fixable: crate::linter::Fixable::#fixable,
            recommended: crate::linter::Recommended::#recommended,
            stability: crate::linter::Stability::#stability,
            level: crate::linter::LintLevel::#level,
            scope: crate::linter::LintScope::#scope,
        };

        impl #name {
            /// Get the lint metadata.
            pub const fn meta() -> &'static crate::linter::LintMeta {
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
