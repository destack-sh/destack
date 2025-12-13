use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::HashSet;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, Ident, Lit, LitStr, Meta, Result, Type,
    parse_macro_input,
};

/// Parsed warning variant data.
struct WarningVariant {
    name: Ident,
    code: LitStr,
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
        return Err(Error::new(
            code.span(),
            format!(
                "warning code must be exactly 5 characters (e.g., \"W{expected_letter}001\"), got \"{code_str}\""
            ),
        ));
    }

    // first character must be 'W'
    if chars[0] != 'W' {
        return Err(Error::new(
            code.span(),
            format!("warning code must start with 'W', got '{}'", chars[0]),
        ));
    }

    // second character must match the phase letter
    if chars[1] != expected_letter {
        return Err(Error::new(
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
        _ => Err(Error::new(
            code.span(),
            format!("warning code must end with 3 digits (000-999), got \"{number_str}\""),
        )),
    }
}

/// Get the simple type name from a Type.
fn type_name(ty: &Type) -> Option<String> {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

/// Generate the formatting expression for a field based on its type.
fn format_field_expr(field_name: &Ident, ty: &Type) -> TokenStream2 {
    let type_name = type_name(ty).unwrap_or_default();
    match type_name.as_str() {
        "StringId" => quote! { program.strings.get(*#field_name).as_str() },
        "StaticKey" => quote! { #field_name.debug_string(&program.strings) },
        "ModuleId" => quote! { program.modules.get(*#field_name).read().uri.to_string() },
        "GlobalNodeIdAny" => quote! { #field_name.local_id.ty.name() },
        _ => quote! { #field_name },
    }
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
                if let Some((name, ty)) = field {
                    result_format.push_str("{}");
                    format_args.push(format_field_expr(name, ty));
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

/// Implementation of the `DefineWarning` derive macro.
pub(crate) fn define_warning_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match define_warning_inner(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn define_warning_inner(input: DeriveInput) -> Result<TokenStream2> {
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

    // check for #[standalone] attribute - if present, don't generate From impl
    let is_standalone = input
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("standalone"));

    let phase_str = phase.to_string();
    let letter = phase_letter(&phase_str);

    // verify enum name matches phase
    let expected_name = format!("{phase_str}Warning");
    if enum_name != &expected_name {
        return Err(Error::new(
            enum_name.span(),
            format!("enum name must be `{expected_name}` for phase `{phase_str}`"),
        ));
    }

    // extract variants
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(Error::new(
                Span::call_site(),
                "DefineWarning only works on enums",
            ));
        }
    };

    let mut variants: Vec<WarningVariant> = Vec::new();
    let mut seen_codes: HashSet<String> = HashSet::new();

    for variant in &data.variants {
        let name = variant.ident.clone();

        // parse #[warning(...)] attribute
        let warning_attr = variant
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("warning"))
            .ok_or_else(|| {
                Error::new(name.span(), "missing #[warning(code = \"...\")] attribute")
            })?;

        let mut code: Option<LitStr> = None;
        let mut message: Option<String> = None;

        warning_attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("code") {
                let value = meta.value()?;
                code = Some(value.parse()?);
            } else if meta.path.is_ident("message") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                message = Some(lit.value());
            } else {
                return Err(meta.error("unknown attribute"));
            }
            Ok(())
        })?;

        let code =
            code.ok_or_else(|| Error::new(name.span(), "missing `code` in #[warning(...)]"))?;

        // validate code
        let _code_number = validate_warning_code(&code, letter)?;
        let code_str = code.value();
        if !seen_codes.insert(code_str.clone()) {
            return Err(Error::new(
                code.span(),
                format!("duplicate warning code \"{code_str}\""),
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

        variants.push(WarningVariant {
            name,
            code,
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
            if v.fields.iter().any(|(n, _)| n == "node") {
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

    let mut output = quote! {
        impl #enum_name {
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

            /// Get the full code for this warning (e.g., "WR001").
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

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.code())
            }
        }
    };

    // conditionally generate From impl
    if !is_standalone {
        output.extend(quote! {
            impl From<#enum_name> for TaskWarning {
                fn from(warning: #enum_name) -> Self {
                    TaskWarning::#phase(warning)
                }
            }
        });
    }

    Ok(output)
}
