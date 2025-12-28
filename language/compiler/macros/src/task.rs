use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{
    Data, DeriveInput, Error, Fields, Ident, LitInt, LitStr, Result, Type, parse_macro_input,
};

struct TaskVariant {
    name: Ident,
    code: LitInt,
    trace: Option<String>,
    fields: Vec<(Ident, Type)>,
}

/// Map a compiler phase name to its single letter code.
fn phase_letter(phase: &str) -> char {
    match phase {
        "Import" => 'I',
        "Bind" => 'B',
        "Resolve" => 'R',
        "Analyze" => 'A',
        "Elaborate" => 'E',
        "Lower" => 'M',
        "Verify" => 'V',
        "Execute" => 'X',
        "Optimize" => 'O',
        "Generate" => 'G',
        "Link" => 'K',
        "Emit" => 'W',
        "Lint" => 'L',
        _ => '?',
    }
}

/// Convert a PascalCase name to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Derive the task name from the variant name by stripping the phase prefix.
fn derive_task_name(variant_name: &str, phase: &str) -> String {
    // strip phase prefix if present: "ImportModule" -> "Module" for phase "Import"
    let stripped = variant_name.strip_prefix(phase).unwrap_or(variant_name);
    to_snake_case(stripped)
}

/// Generate the formatting expression for a field using DiagnosticFormat.
fn format_field_expr(field_name: &Ident) -> TokenStream2 {
    quote! { #field_name.diagnostic_fmt(program) }
}

/// Parse a trace format string and generate code that formats it at runtime.
fn generate_format_expr(
    format_str: &str,
    fields: &[(Ident, Type)],
    span: Span,
) -> Result<TokenStream2> {
    let mut result_format = String::new();
    let mut format_args: Vec<TokenStream2> = Vec::new();
    let mut chars = format_str.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                // escaped opening brace
                chars.next();
                result_format.push_str("{{");
            }
            '{' => {
                // placeholder: extract field name until closing brace
                let mut field_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        break;
                    }
                    field_name.push(chars.next().unwrap());
                }

                // look up field and generate format arg
                let field = fields.iter().find(|(name, _)| name == &field_name);
                if let Some((name, _)) = field {
                    result_format.push_str("{}");
                    format_args.push(format_field_expr(name));
                } else {
                    return Err(Error::new(
                        span,
                        format!("unknown field `{field_name}` in format string"),
                    ));
                }
            }
            '}' if chars.peek() == Some(&'}') => {
                // escaped closing brace
                chars.next();
                result_format.push_str("}}");
            }
            _ => {
                result_format.push(c);
            }
        }
    }

    Ok(quote! { format!(#result_format, #(#format_args),*) })
}

/// Entry point for the DefineTask derive macro.
pub(crate) fn define_task_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match define_task_inner(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn define_task_inner(input: DeriveInput) -> Result<TokenStream2> {
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
    let expected_name = format!("{phase_str}Task");
    if enum_name != &expected_name {
        return Err(Error::new(
            enum_name.span(),
            format!("enum name must be `{expected_name}` for phase `{phase_str}`"),
        ));
    }

    // extract enum data
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(Error::new(
                Span::call_site(),
                "DefineTask only works on enums",
            ));
        }
    };

    // parse each variant
    let mut variants: Vec<TaskVariant> = Vec::new();
    let mut seen_codes: HashSet<u8> = HashSet::new();

    for variant in &data.variants {
        let name = variant.ident.clone();

        // find #[task(...)] attribute
        let task_attr = variant
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("task"))
            .ok_or_else(|| Error::new(name.span(), "missing #[task(code = N)] attribute"))?;

        // parse code and trace from attribute
        let mut code: Option<LitInt> = None;
        let mut trace: Option<String> = None;

        task_attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("code") {
                let value = meta.value()?;
                code = Some(value.parse()?);
            } else if meta.path.is_ident("trace") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                trace = Some(lit.value());
            } else {
                return Err(meta.error("unknown attribute"));
            }
            Ok(())
        })?;

        let code = code.ok_or_else(|| Error::new(name.span(), "missing `code` in #[task(...)]"))?;

        // validate code is unique u8
        let code_value: u8 = code
            .base10_parse()
            .map_err(|_| Error::new(code.span(), "task code must be a u8 (0-255)"))?;

        if !seen_codes.insert(code_value) {
            return Err(Error::new(
                code.span(),
                format!("duplicate task code {code_value}"),
            ));
        }

        // extract named fields
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

        variants.push(TaskVariant {
            name,
            code,
            trace,
            fields,
        });
    }

    // generate match arms for sub_code()
    let sub_code_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;
            let code = &v.code;
            quote! { Self::#name { .. } => #code }
        })
        .collect();

    // generate match arms for name(), derived from variant name
    let name_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;
            let derived_name = derive_task_name(&v.name.to_string(), &phase_str);
            quote! { Self::#name { .. } => #derived_name }
        })
        .collect();

    // generate match arms for trace_args()
    let trace_args_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;

            if let Some(trace) = &v.trace {
                // only destructure fields actually used in the format string
                let used_fields: Vec<&Ident> = v
                    .fields
                    .iter()
                    .filter(|(n, _)| trace.contains(&format!("{{{n}}}")))
                    .map(|(n, _)| n)
                    .collect();

                let format_expr = generate_format_expr(trace, &v.fields, v.code.span())
                    .unwrap_or_else(|e| e.to_compile_error());

                if used_fields.is_empty() {
                    quote! { Self::#name { .. } => #format_expr }
                } else {
                    quote! { Self::#name { #(#used_fields,)* .. } => #format_expr }
                }
            } else {
                quote! { Self::#name { .. } => String::new() }
            }
        })
        .collect();

    // generate match arms for anchor()
    let anchor_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let name = &v.name;

            // check for optional node with module fallback
            let has_module = v.fields.iter().any(|(n, _)| n == "module");
            let node_is_option = v
                .fields
                .iter()
                .any(|(n, ty)| n == "node" && quote!(#ty).to_string().starts_with("Option"));

            if has_module && node_is_option {
                // both module and optional node: use node if present, otherwise module
                quote! {
                    Self::#name { node, module, .. } => match node {
                        Some(n) => crate::DiagnosticAnchor::Node(*n),
                        None => crate::DiagnosticAnchor::Module(*module),
                    }
                }
            } else if v.fields.iter().any(|(n, _)| n == "node") {
                quote! { Self::#name { node, .. } => crate::DiagnosticAnchor::Node(*node) }
            } else if v.fields.iter().any(|(n, _)| n == "module") {
                quote! { Self::#name { module, .. } => crate::DiagnosticAnchor::Module(*module) }
            } else if v.fields.iter().any(|(n, _)| n == "package") {
                quote! { Self::#name { package, .. } => crate::DiagnosticAnchor::Package(*package) }
            } else {
                quote! { Self::#name { .. } => crate::DiagnosticAnchor::Global }
            }
        })
        .collect();

    let task_variant = format_ident!("{}", phase_str);

    Ok(quote! {
        use crate::DiagnosticFormat as _;

        impl #enum_name {
            /// Phase letter for this task type.
            pub const PHASE_LETTER: char = #letter;

            /// Get the numeric sub-code of the task.
            #[inline]
            pub fn sub_code(&self) -> u8 {
                match self {
                    #(#sub_code_arms),*
                }
            }

            /// Get the diagnostic anchor for this task.
            #[inline]
            pub fn anchor(&self) -> crate::DiagnosticAnchor {
                match self {
                    #(#anchor_arms),*
                }
            }
        }

        impl crate::TaskDebug for #enum_name {
            fn name(&self) -> &'static str {
                match self {
                    #(#name_arms),*
                }
            }

            #[allow(unused_variables)]
            fn trace_args(&self, program: &destack_workspace::Program) -> String {
                match self {
                    #(#trace_args_arms),*
                }
            }
        }

        impl From<#enum_name> for crate::Task {
            #[inline]
            fn from(task: #enum_name) -> Self {
                crate::Task::#task_variant(task)
            }
        }
    })
}
