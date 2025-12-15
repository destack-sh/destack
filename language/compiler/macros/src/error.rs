use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, Ident, Lit, LitStr, Meta, Result, Type,
    parse_macro_input,
};

/// Marker for variants that represent task yielding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum YieldMarker {
    /// Not a yield variant.
    None,
    /// Variant marked with `yield`.
    Yield,
    /// Variant marked with `yield_failed`.
    YieldFailed,
}

/// Parsed error variant data.
struct ErrorVariant {
    name: Ident,
    code: LitStr,
    yield_marker: YieldMarker,
    message: Option<String>,
    fields: Vec<(Ident, Type)>,
}

/// Map a compiler phase name to its single-letter code.
fn phase_letter(phase: &str) -> char {
    match phase {
        "Import" => 'I',
        "Bind" => 'B',
        "Resolve" => 'R',
        "Analyze" => 'A',
        "Elaborate" => 'E',
        "Lower" => 'M',
        "Verify" => 'V',
        "Optimize" => 'O',
        "Generate" => 'G',
        "Link" => 'K',
        "Emit" => 'X',
        "Lint" => 'L',
        _ => '?',
    }
}

/// Validate error code format and extract the numeric part.
fn validate_error_code(code: &LitStr, expected_letter: char) -> Result<u16> {
    let code_str = code.value();
    let chars: Vec<char> = code_str.chars().collect();

    // must be exactly 5 characters: E + letter + 3 digits
    if chars.len() != 5 {
        return Err(Error::new(
            code.span(),
            format!(
                "error code must be exactly 5 characters (e.g., \"E{expected_letter}001\"), got \"{code_str}\""
            ),
        ));
    }

    // first character must be 'E'
    if chars[0] != 'E' {
        return Err(Error::new(
            code.span(),
            format!("error code must start with 'E', got '{}'", chars[0]),
        ));
    }

    // second character must match the phase letter
    if chars[1] != expected_letter {
        return Err(Error::new(
            code.span(),
            format!(
                "error code phase letter must be '{expected_letter}' for this phase, got '{}'",
                chars[1]
            ),
        ));
    }

    // remaining 3 characters must be digits
    let number_str: String = chars[2..5].iter().collect();
    match number_str.parse::<u16>() {
        Ok(n) if n < 1000 => Ok(n),
        _ => Err(Error::new(
            code.span(),
            format!("error code must end with 3 digits (000-999), got \"{number_str}\""),
        )),
    }
}

/// Generate the formatting expression for a field using DiagnosticFormat trait.
fn format_field_expr(field_name: &Ident) -> TokenStream2 {
    quote! { #field_name.diagnostic_fmt(program) }
}

/// Parse a format string and generate the formatting code.
fn generate_format_expr(
    format_str: &str,
    fields: &[(Ident, Type)],
    span: Span,
) -> Result<TokenStream2> {
    // find all {field_name} placeholders
    let mut result_format = String::new();
    let mut format_args: Vec<TokenStream2> = Vec::new();
    let mut chars = format_str.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                // escaped brace
                chars.next();
                result_format.push_str("{{");
            } else {
                // placeholder
                let mut field_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        break;
                    }
                    field_name.push(chars.next().unwrap());
                }

                // find the field
                let field = fields.iter().find(|(name, _)| name == &field_name);
                if let Some((name, _ty)) = field {
                    result_format.push_str("{}");
                    format_args.push(format_field_expr(name));
                } else {
                    return Err(Error::new(
                        span,
                        format!("unknown field `{field_name}` in format string"),
                    ));
                }
            }
        } else if c == '}' {
            if chars.peek() == Some(&'}') {
                // escaped brace
                chars.next();
                result_format.push_str("}}");
            } else {
                result_format.push(c);
            }
        } else {
            result_format.push(c);
        }
    }

    Ok(quote! { format!(#result_format, #(#format_args),*) })
}

/// Extract the doc comment from attributes.
fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(nv) = &attr.meta
                && let Expr::Lit(expr_lit) = &nv.value
                && let Lit::Str(s) = &expr_lit.lit
            {
                return Some(s.value().trim().to_string());
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Implementation of the `DefineError` derive macro.
pub(crate) fn define_error_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match define_error_inner(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn define_error_inner(input: DeriveInput) -> Result<TokenStream2> {
    let enum_name = &input.ident;

    // extract phase from #[phase(X)] attribute
    let phase = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("phase") {
                attr.parse_args::<Ident>().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| Error::new(Span::call_site(), "missing #[phase(Name)] attribute"))?;

    let phase_str = phase.to_string();
    let letter = phase_letter(&phase_str);

    // verify enum name matches phase
    let expected_name = format!("{phase_str}Error");
    if enum_name != &expected_name {
        return Err(Error::new(
            enum_name.span(),
            format!("enum name must be `{expected_name}` for phase `{phase_str}`"),
        ));
    }

    let result_name = format_ident!("{}Result", phase_str);

    // extract variants
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(Error::new(
                Span::call_site(),
                "DefineError only works on enums",
            ));
        }
    };

    let mut variants: Vec<ErrorVariant> = Vec::new();
    let mut seen_codes: HashSet<String> = HashSet::new();

    for variant in &data.variants {
        let name = variant.ident.clone();

        // parse #[error(...)] attribute
        let error_attr = variant
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("error"))
            .ok_or_else(|| Error::new(name.span(), "missing #[error(code = \"...\")] attribute"))?;

        let mut code: Option<LitStr> = None;
        let mut yield_marker = YieldMarker::None;
        let mut message: Option<String> = None;

        error_attr.parse_nested_meta(|meta| {
            // code
            if meta.path.is_ident("code") {
                let value = meta.value()?;
                code = Some(value.parse()?);
            }
            // message
            else if meta.path.is_ident("message") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                message = Some(lit.value());
            }
            // yield
            else if meta.path.is_ident("r#yield")
                || meta.path.get_ident().map(|i| i.to_string()) == Some("yield".to_string())
            {
                yield_marker = YieldMarker::Yield;
            }
            // yield_failed
            else if meta.path.is_ident("yield_failed") {
                yield_marker = YieldMarker::YieldFailed;
            }
            // unknown attribute
            else {
                return Err(meta.error("unknown attribute"));
            }
            Ok(())
        })?;

        let code =
            code.ok_or_else(|| Error::new(name.span(), "missing `code` in #[error(...)]"))?;

        // validate code
        let _code_number = validate_error_code(&code, letter)?;
        let code_str = code.value();
        if !seen_codes.insert(code_str.clone()) {
            return Err(Error::new(
                code.span(),
                format!("duplicate error code \"{code_str}\""),
            ));
        }

        // extract fields
        let fields: Vec<(Ident, Type)> = match &variant.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|f| (f.ident.clone().unwrap(), f.ty.clone()))
                .collect(),
            Fields::Unnamed(_) => {
                return Err(Error::new(name.span(), "tuple variants not supported"));
            }
            Fields::Unit => Vec::new(),
        };

        variants.push(ErrorVariant {
            name,
            code,
            yield_marker,
            message,
            fields,
        });
    }

    // generate code match arms
    let code_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;
            let code_str = v.code.value();
            quote! { Self::#name { .. } => #code_str }
        })
        .collect();

    // generate sub_code match arms
    let sub_code_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;
            let code_str = v.code.value();
            let sub_code: u8 = code_str[2..].parse().unwrap_or(0);
            quote! { Self::#name { .. } => #sub_code }
        })
        .collect();

    // collect all codes for the ALL_CODES array
    let all_codes: Vec<String> = variants.iter().map(|v| v.code.value()).collect();

    // generate DiagnosticDefinition entries
    let diagnostic_defs: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let code = v.code.value();
            let name = v.name.to_string();
            let description = extract_doc_comment(
                &data
                    .variants
                    .iter()
                    .find(|var| var.ident == v.name)
                    .unwrap()
                    .attrs,
            );
            let sub_code: u8 = code[2..].parse().unwrap_or(0);
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
        .map(|v| {
            let name = &v.name;
            if v.fields.iter().any(|(n, _)| n == "dependency") {
                quote! { Self::#name { dependency, .. } => dependency.anchor() }
            } else if v.fields.iter().any(|(n, _)| n == "node") {
                quote! { Self::#name { node, .. } => DiagnosticAnchor::Node(*node) }
            } else if v.fields.iter().any(|(n, _)| n == "span") {
                quote! { Self::#name { span, .. } => DiagnosticAnchor::File(span.file) }
            } else if v.fields.iter().any(|(n, _)| n == "package") {
                quote! { Self::#name { package, .. } => DiagnosticAnchor::Package(*package) }
            } else if v.fields.iter().any(|(n, _)| n == "module") {
                quote! { Self::#name { module, .. } => DiagnosticAnchor::Module(*module) }
            } else {
                quote! { Self::#name { .. } => DiagnosticAnchor::Global }
            }
        })
        .collect();

    // generate message match arms
    let message_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;

            if let Some(msg) = &v.message {
                // extract only the fields used in the format string
                let used_fields: Vec<&Ident> = v
                    .fields
                    .iter()
                    .filter(|(n, _)| msg.contains(&format!("{{{n}}}")))
                    .map(|(n, _)| n)
                    .collect();

                let format_expr = generate_format_expr(msg, &v.fields, v.code.span())
                    .unwrap_or_else(|e| e.to_compile_error());
                if used_fields.is_empty() {
                    quote! { Self::#name { .. } => #format_expr }
                } else {
                    quote! { Self::#name { #(#used_fields,)* .. } => #format_expr }
                }
            } else {
                let default_msg = v.name.to_string();
                quote! { Self::#name { .. } => #default_msg.to_string() }
            }
        })
        .collect();

    // find yield variants for TryFrom impl
    let yield_variant = variants
        .iter()
        .find(|v| v.yield_marker == YieldMarker::Yield);
    let yield_failed_variant = variants
        .iter()
        .find(|v| v.yield_marker == YieldMarker::YieldFailed);

    let try_from_impl = if let Some(yv) = yield_variant {
        let yield_name = &yv.name;
        quote! {
            impl TryFrom<#enum_name> for TaskDependency {
                type Error = #enum_name;

                fn try_from(error: #enum_name) -> Result<Self, Self::Error> {
                    match error {
                        #enum_name::#yield_name { dependency } => Ok(dependency),
                        _ => Err(error),
                    }
                }
            }
        }
    } else {
        quote! {
            impl TryFrom<#enum_name> for TaskDependency {
                type Error = #enum_name;

                fn try_from(error: #enum_name) -> Result<Self, Self::Error> {
                    Err(error)
                }
            }
        }
    };

    let from_task_dependency_error_impl = if let (Some(yv), Some(yfv)) =
        (yield_variant, yield_failed_variant)
    {
        let yield_name = &yv.name;
        let yield_failed_name = &yfv.name;
        quote! {
            impl From<TaskDependencyError> for #enum_name {
                fn from(error: TaskDependencyError) -> Self {
                    match error {
                        TaskDependencyError::NotReady { dependency } => Self::#yield_name { dependency },
                        TaskDependencyError::Failed { dependency } => Self::#yield_failed_name { dependency },
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        // Import the DiagnosticFormat trait for field formatting in messages.
        use crate::DiagnosticFormat as _;

        impl #enum_name {
            /// Phase letter for this error type.
            pub const PHASE_LETTER: char = #letter;

            /// All diagnostic definitions for this phase.
            pub const ALL: &'static [DiagnosticDefinition] = &[
                #(#diagnostic_defs),*
            ];

            /// All diagnostic codes for this phase.
            pub const ALL_CODES: &'static [&'static str] = &[
                #(#all_codes),*
            ];

            /// Check if a code string is valid for this error type.
            #[inline]
            pub fn is_valid_code(code: &str) -> bool {
                Self::ALL_CODES.contains(&code)
            }

            /// Get the definition for a code, if valid.
            pub fn def_for_code(code: &str) -> Option<&'static DiagnosticDefinition> {
                Self::ALL.iter().find(|def| def.code == code)
            }

            /// Get the numeric sub-code of the error.
            #[inline]
            pub fn sub_code(&self) -> u8 {
                match self {
                    #(#sub_code_arms),*
                }
            }

            /// Get the full code for this error (e.g., "ER004").
            #[inline]
            pub fn code(&self) -> &'static str {
                match self {
                    #(#code_arms),*
                }
            }

            /// Get the anchor for this error.
            pub fn anchor(&self) -> DiagnosticAnchor {
                match self {
                    #(#anchor_arms),*
                }
            }

            /// Get the message for this error.
            #[allow(unused_variables)]
            pub fn message(&self, program: &Program) -> String {
                match self {
                    #(#message_arms),*
                }
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.code())
            }
        }

        impl From<#enum_name> for TaskError {
            #[inline]
            fn from(error: #enum_name) -> Self {
                TaskError::#phase(error)
            }
        }

        #try_from_impl

        #from_task_dependency_error_impl

        /// Result type for this phase.
        pub type #result_name<T> = Result<T, #enum_name>;
    })
}
