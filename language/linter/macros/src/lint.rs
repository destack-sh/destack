//! Implementation of the `define_lints!` macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, Expr, Ident, LitStr, Result, Token, Type,
};

/// A field in a lint variant.
struct Field {
    name: Ident,
    ty: Type,
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(Field { name, ty })
    }
}

/// A lint rule definition.
struct LintRule {
    attrs: Vec<Attribute>,
    name: Ident,
    id: Option<String>,
    default_severity: Option<Ident>,
    docs_url: Option<String>,
    fixable: bool,
    fields: Vec<Field>,
    message: Option<LintMessage>,
}

/// Message definition for a lint.
enum LintMessage {
    /// Simple string literal.
    Simple(String),
    /// Complex closure expression.
    Closure(Expr),
}

impl Parse for LintRule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;

        // extract special attributes
        let mut id = None;
        let mut default_severity = None;
        let mut docs_url = None;
        let mut fixable = false;

        for attr in &attrs {
            if attr.path().is_ident("id") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        id = Some(s.value());
                    }
                }
            } else if attr.path().is_ident("default_severity") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Path(p) = &nv.value {
                        if let Some(ident) = p.path.get_ident() {
                            default_severity = Some(ident.clone());
                        }
                    }
                }
            } else if attr.path().is_ident("docs") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        docs_url = Some(s.value());
                    }
                }
            } else if attr.path().is_ident("fixable") {
                fixable = true;
            }
        }

        let name: Ident = input.parse()?;

        // parse fields: { field: Type, ... }
        let content;
        braced!(content in input);
        let fields_punctuated: Punctuated<Field, Token![,]> =
            content.parse_terminated(Field::parse, Token![,])?;
        let fields: Vec<Field> = fields_punctuated.into_iter().collect();

        // parse optional message: => "..." or => |self, program| { ... }
        let message = if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;

            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;
                Some(LintMessage::Simple(lit.value()))
            } else {
                let expr: Expr = input.parse()?;
                Some(LintMessage::Closure(expr))
            }
        } else {
            None
        };

        Ok(LintRule {
            attrs,
            name,
            id,
            default_severity,
            docs_url,
            fixable,
            fields,
            message,
        })
    }
}

/// The full input to the define_lints! macro.
struct DefineLintsInput {
    attrs: Vec<Attribute>,
    category: Ident,
    rules: Vec<LintRule>,
}

impl Parse for DefineLintsInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;

        // parse: category Correctness { ... }
        input.parse::<Ident>()?; // "category" keyword
        let category: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let mut rules = Vec::new();
        while !content.is_empty() {
            rules.push(content.parse::<LintRule>()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(DefineLintsInput {
            attrs,
            category,
            rules,
        })
    }
}

/// Get the category letter for a category name.
fn category_letter(category: &Ident) -> char {
    match category.to_string().as_str() {
        "Correctness" => 'C',
        "Suspicious" => 'U', // sUspicious
        "Performance" => 'P',
        "Style" => 'Y',      // stYle
        "Security" => 'S',
        "Complexity" => 'X', // compleXity
        _ => 'L',            // generic Lint
    }
}

pub(crate) fn define_lints_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DefineLintsInput);

    let category = &input.category;
    let category_letter = category_letter(category);
    let rules = &input.rules;

    // generate LintDef entries
    let lint_defs: Vec<TokenStream2> = rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let code = format!("L{}{:03}", category_letter, i);
            let name = r.name.to_string();
            let id = r.id.clone().unwrap_or_else(|| to_kebab_case(&name));
            let desc = r
                .attrs
                .iter()
                .filter_map(|a| {
                    if a.path().is_ident("doc") {
                        if let syn::Meta::NameValue(nv) = &a.meta {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(s),
                                ..
                            }) = &nv.value
                            {
                                return Some(s.value().trim().to_string());
                            }
                        }
                    }
                    None
                })
                .collect::<Vec<_>>()
                .join(" ");
            let severity = r
                .default_severity
                .as_ref()
                .map(|s| quote! { DiagnosticSeverity::#s })
                .unwrap_or_else(|| quote! { DiagnosticSeverity::Warning });
            let docs = r
                .docs_url
                .as_ref()
                .map(|u| quote! { Some(#u) })
                .unwrap_or_else(|| quote! { None });
            let fixable = r.fixable;
            let sub_code = i as u8;

            quote! {
                LintDef {
                    code: #code,
                    id: #id,
                    name: #name,
                    description: #desc,
                    category: LintCategory::#category,
                    default_severity: #severity,
                    docs_url: #docs,
                    fixable: #fixable,
                    sub_code: #sub_code,
                }
            }
        })
        .collect();

    // generate ALL_CODES array
    let all_codes: Vec<String> = rules
        .iter()
        .enumerate()
        .map(|(i, _)| format!("L{}{:03}", category_letter, i))
        .collect();

    // generate ALL_IDS array
    let all_ids: Vec<String> = rules
        .iter()
        .map(|r| {
            r.id.clone()
                .unwrap_or_else(|| to_kebab_case(&r.name.to_string()))
        })
        .collect();

    let category_lints_name = format_ident!("{}Lints", category);

    let expanded = quote! {
        /// Static metadata about a lint rule.
        #[derive(Debug, Clone, Copy)]
        pub struct LintDef {
            /// The full code (e.g., "LC003").
            pub code: &'static str,
            /// The kebab-case id (e.g., "no-floating-promise").
            pub id: &'static str,
            /// The variant name (e.g., "NoFloatingPromise").
            pub name: &'static str,
            /// The doc comment description.
            pub description: &'static str,
            /// The category.
            pub category: LintCategory,
            /// The default severity.
            pub default_severity: DiagnosticSeverity,
            /// The documentation URL.
            pub docs_url: Option<&'static str>,
            /// Whether the lint has an auto-fix.
            pub fixable: bool,
            /// The sub-code number.
            pub sub_code: u8,
        }

        /// Lints in the #category category.
        pub struct #category_lints_name;

        impl #category_lints_name {
            /// Category letter for this lint category.
            pub const CATEGORY_LETTER: char = #category_letter;

            /// All lint definitions for this category.
            pub const ALL: &'static [LintDef] = &[
                #(#lint_defs),*
            ];

            /// All lint codes for this category.
            pub const ALL_CODES: &'static [&'static str] = &[
                #(#all_codes),*
            ];

            /// All lint ids for this category.
            pub const ALL_IDS: &'static [&'static str] = &[
                #(#all_ids),*
            ];

            /// Check if a code string is valid for this category.
            #[inline]
            pub fn is_valid_code(code: &str) -> bool {
                Self::ALL_CODES.contains(&code)
            }

            /// Check if an id string is valid for this category.
            #[inline]
            pub fn is_valid_id(id: &str) -> bool {
                Self::ALL_IDS.contains(&id)
            }

            /// Get the definition for a code, if valid.
            pub fn def_for_code(code: &str) -> Option<&'static LintDef> {
                Self::ALL.iter().find(|def| def.code == code)
            }

            /// Get the definition for an id, if valid.
            pub fn def_for_id(id: &str) -> Option<&'static LintDef> {
                Self::ALL.iter().find(|def| def.id == id)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Convert PascalCase to kebab-case.
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}
