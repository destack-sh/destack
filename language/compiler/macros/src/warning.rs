use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Ident, LitStr, Result, Token, Type, braced, parse_macro_input};

/// A field within a warning variant.
struct WarningField {
    name: Ident,
    field_type: Type,
}

impl Parse for WarningField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let field_type: Type = input.parse()?;
        Ok(WarningField { name, field_type })
    }
}

/// A single warning variant definition from the macro input.
struct WarningVariant {
    attributes: Vec<Attribute>,
    code: LitStr,
    name: Ident,
    fields: Vec<WarningField>,
    message: Option<WarningMessage>,
}

/// The message format for a warning variant.
enum WarningMessage {
    /// A simple string literal message.
    Simple(String),
    /// An expression that computes the message.
    Closure(Expr),
}

impl Parse for WarningVariant {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;

        // parse the warning code: "WR001" = VariantName { ... }
        let code: LitStr = input.parse()?;
        input.parse::<Token![=]>()?;

        let name: Ident = input.parse()?;

        // parse fields
        let content;
        braced!(content in input);
        let fields_punctuated: Punctuated<WarningField, Token![,]> =
            content.parse_terminated(WarningField::parse, Token![,])?;
        let fields: Vec<WarningField> = fields_punctuated.into_iter().collect();

        // parse optional message
        let message = if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;

            if input.peek(LitStr) {
                let literal: LitStr = input.parse()?;
                Some(WarningMessage::Simple(literal.value()))
            } else {
                let expression: Expr = input.parse()?;
                Some(WarningMessage::Closure(expression))
            }
        } else {
            None
        };

        Ok(WarningVariant {
            attributes,
            code,
            name,
            fields,
            message,
        })
    }
}

/// Parsed input for the `define_warnings!` macro.
struct DefineWarningsInput {
    attributes: Vec<Attribute>,
    phase: Ident,
    variants: Vec<WarningVariant>,
}

impl Parse for DefineWarningsInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;

        // parse: Resolve, { ... }
        let phase: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let content;
        braced!(content in input);

        let mut variants = Vec::new();
        while !content.is_empty() {
            variants.push(content.parse::<WarningVariant>()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(DefineWarningsInput {
            attributes,
            phase,
            variants,
        })
    }
}

/// Map a compiler phase name to its single-letter code.
fn phase_letter(phase: &Ident) -> char {
    match phase.to_string().as_str() {
        "Import" => 'I',
        "Bind" => 'B',
        "Resolve" => 'R',
        "Analyze" => 'A',
        "Elaborate" => 'E',
        "Lower" => 'L',
        "Verify" => 'V',
        "Optimize" => 'O',
        "Generate" => 'G',
        "Link" => 'K',
        "Emit" => 'M',
        _ => 'X',
    }
}

/// Validate warning code format and extract the numeric part.
fn validate_warning_code(code: &LitStr, expected_letter: char) -> Result<u16> {
    let code_str = code.value();
    let chars: Vec<char> = code_str.chars().collect();

    // must be exactly 5 characters: W + letter + 3 digits
    if chars.len() != 5 {
        return Err(syn::Error::new(
            code.span(),
            format!(
                "warning code must be exactly 5 characters (e.g., \"W{expected_letter}001\"), got \"{code_str}\""
            ),
        ));
    }

    // first character must be 'W'
    if chars[0] != 'W' {
        return Err(syn::Error::new(
            code.span(),
            format!("warning code must start with 'W', got '{}'", chars[0]),
        ));
    }

    // second character must match the phase letter
    if chars[1] != expected_letter {
        return Err(syn::Error::new(
            code.span(),
            format!(
                "warning code phase letter must be '{expected_letter}' for this phase, got '{}'",
                chars[1]
            ),
        ));
    }

    // remaining 3 characters must be digits
    let number_str: String = chars[2..5].iter().collect();
    match number_str.parse::<u16>() {
        Ok(n) if n < 1000 => Ok(n),
        _ => Err(syn::Error::new(
            code.span(),
            format!("warning code must end with 3 digits (000-999), got \"{number_str}\""),
        )),
    }
}

/// Implementation of the `define_warnings!` macro.
pub(crate) fn define_warnings_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DefineWarningsInput);

    let phase = &input.phase;
    let letter = phase_letter(phase);
    let warning_name = format_ident!("{}Warning", phase);

    let attributes = &input.attributes;
    let variants = &input.variants;

    // validate all codes and check for duplicates
    let mut seen_codes: HashSet<String> = HashSet::new();
    let mut code_numbers: Vec<u16> = Vec::new();

    for variant in variants {
        match validate_warning_code(&variant.code, letter) {
            Ok(number) => {
                let code_str = variant.code.value();
                if !seen_codes.insert(code_str.clone()) {
                    return syn::Error::new(
                        variant.code.span(),
                        format!("duplicate warning code \"{code_str}\""),
                    )
                    .to_compile_error()
                    .into();
                }
                code_numbers.push(number);
            }
            Err(e) => return e.to_compile_error().into(),
        }
    }

    // generate enum variants
    let enum_variants: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let name = &variant.name;
            let variant_attributes = &variant.attributes;
            let fields: Vec<TokenStream2> = variant
                .fields
                .iter()
                .map(|field| {
                    let field_name = &field.name;
                    let field_type = &field.field_type;
                    quote! { #field_name: #field_type }
                })
                .collect();
            quote! {
                #(#variant_attributes)*
                #name { #(#fields),* }
            }
        })
        .collect();

    // generate code match arms using the explicit codes
    let code_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let name = &variant.name;
            let code_str = variant.code.value();
            quote! { Self::#name { .. } => #code_str }
        })
        .collect();

    // generate sub_code match arms
    let sub_code_arms: Vec<TokenStream2> = variants
        .iter()
        .zip(code_numbers.iter())
        .map(|(variant, &sub_code)| {
            let name = &variant.name;
            let sub_code = sub_code as u8;
            quote! { Self::#name { .. } => #sub_code }
        })
        .collect();

    // collect all codes for the ALL_CODES array
    let all_codes: Vec<String> = variants.iter().map(|v| v.code.value()).collect();

    // generate DiagnosticDefinition entries
    let diagnostic_defs: Vec<TokenStream2> = variants
        .iter()
        .zip(code_numbers.iter())
        .map(|(variant, &sub_code)| {
            let code = variant.code.value();
            let name = variant.name.to_string();
            let description = extract_doc_comment(&variant.attributes);
            let sub_code = sub_code as u8;
            quote! {
                DiagnosticDefinition {
                    code: #code,
                    name: #name,
                    description: #description,
                    sub_code: #sub_code,
                }
            }
        })
        .collect();

    // generate anchor match arms
    let anchor_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let name = &variant.name;

            // only bind the field we actually use for the anchor
            if variant.fields.iter().any(|f| f.name == "node") {
                quote! { Self::#name { node, .. } => DiagnosticAnchor::Node(*node) }
            } else if variant.fields.iter().any(|f| f.name == "span") {
                quote! { Self::#name { span, .. } => DiagnosticAnchor::File(span.file) }
            } else {
                quote! { Self::#name { .. } => DiagnosticAnchor::Global }
            }
        })
        .collect();

    // generate message match arms
    let message_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let name = &variant.name;

            match &variant.message {
                Some(WarningMessage::Simple(string)) => {
                    // simple string, no fields needed
                    quote! { Self::#name { .. } => #string.to_string() }
                }
                Some(WarningMessage::Closure(expression)) => {
                    // closure needs all fields bound for use in the expression
                    let field_names: Vec<&Ident> = variant.fields.iter().map(|f| &f.name).collect();
                    quote! { Self::#name { #(#field_names),* } => #expression }
                }
                None => {
                    let default_message = variant.name.to_string();
                    quote! { Self::#name { .. } => #default_message.to_string() }
                }
            }
        })
        .collect();

    let expanded = quote! {
        #(#attributes)*
        #[derive(Debug, Clone, PartialEq)]
        pub enum #warning_name {
            #(#enum_variants),*
        }

        impl #warning_name {
            /// Phase letter for this warning type.
            pub const PHASE_LETTER: char = #letter;

            /// All diagnostic definitions for this phase.
            pub const ALL: &'static [DiagnosticDefinition] = &[
                #(#diagnostic_defs),*
            ];

            /// All diagnostic codes for this phase.
            pub const ALL_CODES: &'static [&'static str] = &[
                #(#all_codes),*
            ];

            /// Check if a code string is valid for this warning type.
            #[inline]
            pub fn is_valid_code(code: &str) -> bool {
                Self::ALL_CODES.contains(&code)
            }

            /// Get the definition for a code, if valid.
            pub fn def_for_code(code: &str) -> Option<&'static DiagnosticDefinition> {
                Self::ALL.iter().find(|def| def.code == code)
            }

            /// Get the numeric sub-code of the warning.
            #[inline]
            pub fn sub_code(&self) -> u8 {
                match self {
                    #(#sub_code_arms),*
                }
            }

            /// Get the full code for this warning (e.g., "WA004").
            #[inline]
            pub fn code(&self) -> &'static str {
                match self {
                    #(#code_arms),*
                }
            }

            /// Get the anchor for this warning.
            pub fn anchor(&self) -> DiagnosticAnchor {
                match self {
                    #(#anchor_arms),*
                }
            }

            /// Get the message for this warning.
            #[allow(unused_variables)]
            pub fn message(&self, program: &Program) -> String {
                match self {
                    #(#message_arms),*
                }
            }
        }

        impl std::fmt::Display for #warning_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.code())
            }
        }

        impl From<#warning_name> for TaskWarning {
            fn from(warning: #warning_name) -> Self {
                TaskWarning::#phase(warning)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Extract the doc comment text from a list of attributes.
fn extract_doc_comment(attributes: &[Attribute]) -> String {
    attributes
        .iter()
        .filter_map(|attribute| {
            if attribute.path().is_ident("doc")
                && let syn::Meta::NameValue(name_value) = &attribute.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(string),
                    ..
                }) = &name_value.value
            {
                return Some(string.value().trim().to_string());
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}
