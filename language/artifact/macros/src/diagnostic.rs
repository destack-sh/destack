use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, Ident, Lit, LitStr, Meta, Path, Result, Type,
    parse_macro_input,
};

/// Parsed diagnostic derive options.
struct DiagnosticDeriveOptions {
    /// The diagnostic severity.
    severity: Ident,
    /// The provider phase.
    phase: Ident,
    /// The optional aggregate enum to convert into.
    into: Option<Path>,
}

/// Parsed diagnostic variant data.
struct RawDiagnosticVariant {
    /// The variant name.
    name: Ident,
    /// The stable diagnostic code.
    code: LitStr,
    /// The doc-comment description.
    description: String,
    /// The diagnostic message template.
    message: Option<String>,
    /// Whether source directives may control this diagnostic.
    directive: bool,
    /// The named fields carried by the variant.
    fields: Vec<(Ident, Type)>,
}

/// Validated diagnostic variant data.
struct DiagnosticVariant {
    /// The variant name.
    name: Ident,
    /// The stable diagnostic code.
    code: LitStr,
    /// The doc-comment description.
    description: String,
    /// The numeric diagnostic sub-code.
    sub_code: u16,
    /// The diagnostic message template.
    message: Option<String>,
    /// Whether source directives may control this diagnostic.
    directive: bool,
    /// The named fields carried by the variant.
    fields: Vec<(Ident, Type)>,
}

impl RawDiagnosticVariant {
    /// Validate this raw variant against the owning diagnostic family.
    fn into_diagnostic_variant(self, severity: char, phase: char) -> Result<DiagnosticVariant> {
        let sub_code = parse_diagnostic_code(&self.code, severity, phase)?;

        Ok(DiagnosticVariant {
            name: self.name,
            code: self.code,
            description: self.description,
            sub_code,
            message: self.message,
            directive: self.directive,
            fields: self.fields,
        })
    }
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
        "Execute" => 'X',
        "Optimize" => 'O',
        "Generate" => 'G',
        "Link" => 'K',
        "Emit" => 'W',
        "Lint" => 'L',
        _ => '?',
    }
}

/// Return the diagnostic code prefix for one severity name.
fn severity_prefix(severity: &Ident) -> Result<char> {
    match severity.to_string().as_str() {
        "Error" => Ok('E'),
        "Warning" => Ok('W'),
        other => Err(Error::new(
            severity.span(),
            format!("unsupported diagnostic severity `{other}`"),
        )),
    }
}

/// Validate diagnostic code format and extract the numeric part.
fn parse_diagnostic_code(code: &LitStr, severity: char, phase: char) -> Result<u16> {
    let code_str = code.value();
    let chars: Vec<char> = code_str.chars().collect();

    // validate the full code shape
    if chars.len() != 5 {
        return Err(Error::new(
            code.span(),
            format!("diagnostic code must be exactly 5 characters, got \"{code_str}\""),
        ));
    }

    // validate severity prefix
    if chars[0] != severity {
        return Err(Error::new(
            code.span(),
            format!(
                "diagnostic code must start with '{severity}', got '{}'",
                chars[0]
            ),
        ));
    }

    // validate phase prefix
    if chars[1] != phase {
        return Err(Error::new(
            code.span(),
            format!(
                "diagnostic code phase letter must be '{phase}', got '{}'",
                chars[1]
            ),
        ));
    }

    // validate numeric suffix
    let number: String = chars[2..5].iter().collect();
    match number.parse::<u16>() {
        Ok(number) if number < 1000 => Ok(number),
        _ => Err(Error::new(
            code.span(),
            format!("diagnostic code must end with 3 digits, got \"{number}\""),
        )),
    }
}

/// Generate the formatting expression for a field using DiagnosticFormat.
fn format_field_expr(field_name: &Ident, formatter_name: &Ident) -> TokenStream2 {
    quote! { #field_name.format_diagnostic(#formatter_name)? }
}

/// Push a unique field name into a generated binding list.
fn push_unique_field(fields: &mut Vec<Ident>, field: &Ident) {
    if !fields.iter().any(|existing| existing == field) {
        fields.push(field.clone());
    }
}

/// Parse a format string and return the fields it references.
fn format_field_names(
    format_str: &str,
    fields: &[(Ident, Type)],
    span: Span,
) -> Result<Vec<Ident>> {
    let mut used_fields = Vec::new();
    let mut chars = format_str.chars().peekable();

    // scan placeholders and validate field names
    while let Some(character) = chars.next() {
        if character == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                continue;
            }

            let mut field_name = String::new();
            while let Some(&character) = chars.peek() {
                if character == '}' {
                    chars.next();
                    break;
                }

                if let Some(character) = chars.next() {
                    field_name.push(character);
                }
            }

            let Some((name, _ty)) = fields.iter().find(|(name, _)| name == &field_name) else {
                return Err(Error::new(
                    span,
                    format!("unknown field `{field_name}` in format string"),
                ));
            };
            push_unique_field(&mut used_fields, name);
        }
        // skip escaped closing braces
        else if character == '}' && chars.peek() == Some(&'}') {
            chars.next();
        }
    }

    Ok(used_fields)
}

/// Parse a format string and generate the formatting code.
fn generate_format_expr(
    format_str: &str,
    fields: &[(Ident, Type)],
    formatter_name: &Ident,
    span: Span,
) -> Result<TokenStream2> {
    let mut result_format = String::new();
    let mut format_args: Vec<TokenStream2> = Vec::new();
    let mut chars = format_str.chars().peekable();

    // scan the template into a format string and argument list
    while let Some(character) = chars.next() {
        if character == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                result_format.push_str("{{");
            } else {
                let mut field_name = String::new();
                while let Some(&character) = chars.peek() {
                    if character == '}' {
                        chars.next();
                        break;
                    }

                    if let Some(character) = chars.next() {
                        field_name.push(character);
                    }
                }

                let Some((name, _ty)) = fields.iter().find(|(name, _)| name == &field_name) else {
                    return Err(Error::new(
                        span,
                        format!("unknown field `{field_name}` in format string"),
                    ));
                };
                result_format.push_str("{}");
                format_args.push(format_field_expr(name, formatter_name));
            }
        } else if character == '}' {
            if chars.peek() == Some(&'}') {
                chars.next();
                result_format.push_str("}}");
            } else {
                result_format.push(character);
            }
        } else {
            result_format.push(character);
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
                && let Meta::NameValue(value) = &attr.meta
                && let Expr::Lit(expression) = &value.value
                && let Lit::Str(string) = &expression.lit
            {
                return Some(string.value().trim().to_string());
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse the item-level diagnostic attribute.
fn parse_options(input: &DeriveInput) -> Result<DiagnosticDeriveOptions> {
    let attribute = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("diagnostic"))
        .ok_or_else(|| Error::new(input.ident.span(), "missing #[diagnostic(...)] attribute"))?;

    let mut severity = None;
    let mut phase = None;
    let mut into = None;

    // parse severity, phase, and optional aggregate target
    attribute.parse_nested_meta(|meta| {
        if meta.path.is_ident("severity") {
            severity = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("phase") {
            phase = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("into") {
            into = Some(meta.value()?.parse()?);
        } else {
            return Err(meta.error("unknown diagnostic option"));
        }

        Ok(())
    })?;

    let severity =
        severity.ok_or_else(|| Error::new(attribute.span(), "missing diagnostic severity"))?;
    let phase = phase.ok_or_else(|| Error::new(attribute.span(), "missing diagnostic phase"))?;

    Ok(DiagnosticDeriveOptions {
        severity,
        phase,
        into,
    })
}

/// Parse one variant-level diagnostic attribute.
fn parse_variant(variant: &syn::Variant) -> Result<RawDiagnosticVariant> {
    let name = variant.ident.clone();
    let description = extract_doc_comment(&variant.attrs);
    let attribute = variant
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("diagnostic"))
        .ok_or_else(|| {
            Error::new(
                name.span(),
                "missing #[diagnostic(code = \"...\")] attribute",
            )
        })?;

    let mut code = None;
    let mut message = None;
    let mut directive = false;

    // parse code, message, and directive control
    attribute.parse_nested_meta(|meta| {
        if meta.path.is_ident("code") {
            code = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("message") {
            let value: LitStr = meta.value()?.parse()?;
            message = Some(value.value());
        } else if meta.path.is_ident("directive") {
            directive = true;
        } else {
            return Err(meta.error("unknown diagnostic variant option"));
        }

        Ok(())
    })?;

    let code = code.ok_or_else(|| Error::new(name.span(), "missing diagnostic code"))?;
    let fields = match &variant.fields {
        Fields::Named(named) => {
            let mut fields = Vec::new();
            for field in &named.named {
                let Some(field_name) = field.ident.clone() else {
                    return Err(Error::new(name.span(), "named field without identifier"));
                };
                fields.push((field_name, field.ty.clone()));
            }
            fields
        }
        Fields::Unnamed(_) => return Err(Error::new(name.span(), "tuple variants not supported")),
        Fields::Unit => Vec::new(),
    };

    Ok(RawDiagnosticVariant {
        name,
        code,
        description,
        message,
        directive,
        fields,
    })
}

/// Return a pattern that ignores all variant fields.
fn variant_pattern(variant: &DiagnosticVariant) -> TokenStream2 {
    let name = &variant.name;
    if variant.fields.is_empty() {
        quote! { Self::#name }
    } else {
        quote! { Self::#name { .. } }
    }
}

/// Return whether one phase generally anchors MIR nodes.
fn phase_uses_mir(phase: &str) -> bool {
    phase == "Optimize" || phase == "Link"
}

/// Generate one site match arm.
fn site_arm(variant: &DiagnosticVariant, is_mir: bool) -> Result<TokenStream2> {
    let name = &variant.name;
    let has_module = variant.fields.iter().any(|(name, _)| name == "module");
    let node_is_option = variant
        .fields
        .iter()
        .any(|(name, ty)| name == "node" && quote!(#ty).to_string().starts_with("Option"));

    // use explicit diagnostic sites first
    if variant.fields.iter().any(|(name, _)| name == "site") {
        Ok(quote! {
            Self::#name { site, .. } => {
                Ok(site.clone())
            }
        })
    }
    // use explicit diagnostic anchors next
    else if variant.fields.iter().any(|(name, _)| name == "anchor") {
        Ok(quote! {
            Self::#name { anchor, .. } => {
                Ok(destack_artifact::DiagnosticSite::Anchor(anchor.clone()))
            }
        })
    }
    // delegate dependency diagnostics to the dependency type
    else if variant.fields.iter().any(|(name, _)| name == "dependency") {
        Ok(quote! { Self::#name { dependency, .. } => dependency.site() })
    }
    // require optional artifact nodes to be present
    else if has_module && node_is_option {
        if is_mir {
            Ok(quote! {
                Self::#name { node, .. } => match node {
                    Some(node) => destack_artifact::DiagnosticSite::mir_optimized(node.clone()),
                    None => Err(destack_artifact::DiagnosticError::InvalidSite {
                        message: format!(
                            "{} requires a MIR diagnostic node",
                            stringify!(#name),
                        ),
                    }),
                }
            })
        } else {
            Ok(quote! {
                Self::#name { node, .. } => match node {
                    Some(node) => destack_artifact::DiagnosticSite::dir_declared(*node),
                    None => Err(destack_artifact::DiagnosticError::InvalidSite {
                        message: format!(
                            "{} requires a DIR diagnostic node",
                            stringify!(#name),
                        ),
                    }),
                }
            })
        }
    }
    // prefer explicit node anchors
    else if variant.fields.iter().any(|(name, _)| name == "node") {
        if is_mir {
            Ok(quote! {
                Self::#name { node, .. } => {
                    destack_artifact::DiagnosticSite::mir_optimized(node.clone())
                }
            })
        } else {
            Ok(quote! {
                Self::#name { node, .. } => {
                    destack_artifact::DiagnosticSite::dir_declared(*node)
                }
            })
        }
    }
    // use concrete spans when present
    else if variant.fields.iter().any(|(name, _)| name == "span") {
        Ok(quote! {
            Self::#name { span, .. } => {
                Ok(destack_artifact::DiagnosticSite::from(*span))
            }
        })
    }
    // use package anchors when present
    else if variant.fields.iter().any(|(name, _)| name == "package") {
        Ok(quote! {
            Self::#name { package, .. } => {
                Ok(destack_artifact::DiagnosticSite::from(*package))
            }
        })
    }
    // use module anchors when present
    else if has_module {
        Ok(quote! {
            Self::#name { module, .. } => {
                Ok(destack_artifact::DiagnosticSite::from(*module))
            }
        })
    }
    // require every provider diagnostic to have a source home
    else {
        Err(Error::new(
            variant.name.span(),
            "diagnostic variant has no source anchor field",
        ))
    }
}

/// Generate one message match arm.
fn message_arm(variant: &DiagnosticVariant, formatter_name: &Ident) -> Result<TokenStream2> {
    let name = &variant.name;

    if let Some(message) = &variant.message {
        let used_fields = format_field_names(message, &variant.fields, variant.code.span())?;
        let format_expr = generate_format_expr(
            message,
            &variant.fields,
            formatter_name,
            variant.code.span(),
        )?;

        if used_fields.is_empty() {
            let pattern = variant_pattern(variant);
            Ok(quote! { #pattern => #format_expr })
        } else {
            Ok(quote! { Self::#name { #(#used_fields,)* .. } => #format_expr })
        }
    } else {
        Err(Error::new(
            variant.code.span(),
            "missing diagnostic message",
        ))
    }
}

/// Derive one provider diagnostic enum.
pub(crate) fn diagnostic_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match diagnostic_inner(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derive one provider diagnostic enum.
fn diagnostic_inner(input: DeriveInput) -> Result<TokenStream2> {
    let enum_name = &input.ident;
    let options = parse_options(&input)?;
    let severity = severity_prefix(&options.severity)?;
    let phase = options.phase.to_string();
    let phase_letter = phase_letter(&phase);

    // validate the enum shape
    let severity_name = options.severity.to_string();
    let expected_name = format!("{phase}{severity_name}");
    if *enum_name != expected_name {
        return Err(Error::new(
            enum_name.span(),
            format!("enum name must be `{expected_name}` for phase `{phase}`"),
        ));
    }

    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(Error::new(
                enum_name.span(),
                "Diagnostic only works on enums",
            ));
        }
    };

    let mut variants = Vec::new();
    let mut codes = HashSet::new();

    // parse variants and validate stable codes
    for variant in &data.variants {
        let variant = parse_variant(variant)?.into_diagnostic_variant(severity, phase_letter)?;
        let code = variant.code.value();
        if !codes.insert(code.clone()) {
            return Err(Error::new(
                variant.code.span(),
                format!("duplicate diagnostic code \"{code}\""),
            ));
        }

        variants.push(variant);
    }

    let result_name = format_ident!("{phase}Result");
    let is_error = severity_name == "Error";
    let severity_ident = &options.severity;
    let is_mir = phase_uses_mir(&phase);
    let context_name = format_ident!("__diagnostic_context");
    let formatter_name = format_ident!("__diagnostic_formatter");

    let all_codes: Vec<String> = variants
        .iter()
        .map(|variant| variant.code.value())
        .collect();
    let directive_codes: Vec<String> = variants
        .iter()
        .filter(|variant| variant.directive)
        .map(|variant| variant.code.value())
        .collect();
    let diagnostic_defs: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let code = variant.code.value();
            let name = variant.name.to_string();
            let description = &variant.description;
            let sub_code = variant.sub_code;
            quote! {
                destack_artifact::DiagnosticDefinition {
                    code: #code,
                    name: #name,
                    description: #description,
                    sub_code: #sub_code,
                }
            }
        })
        .collect();

    let code_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let code = variant.code.value();
            let pattern = variant_pattern(variant);
            quote! { #pattern => #code }
        })
        .collect();
    let sub_code_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let sub_code = variant.sub_code;
            let pattern = variant_pattern(variant);
            quote! { #pattern => #sub_code }
        })
        .collect();
    let directive_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| {
            let directive = variant.directive;
            let pattern = variant_pattern(variant);
            quote! { #pattern => #directive }
        })
        .collect();
    let site_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| site_arm(variant, is_mir))
        .collect::<Result<Vec<_>>>()?;
    let message_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| message_arm(variant, &formatter_name))
        .collect::<Result<Vec<_>>>()?;

    let result_alias = if is_error {
        quote! {
            /// Result type for this phase.
            pub type #result_name<T> = Result<T, #enum_name>;
        }
    } else {
        quote! {}
    };

    let into_impl = if let Some(into) = &options.into {
        let phase = &options.phase;
        let argument = if is_error {
            quote! { error }
        } else {
            quote! { warning }
        };
        quote! {
            impl From<#enum_name> for #into {
                #[inline]
                fn from(#argument: #enum_name) -> Self {
                    #into::#phase(#argument)
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        use destack_artifact::DiagnosticFormat as _;

        impl #enum_name {
            /// Phase letter for this diagnostic type.
            pub const PHASE_LETTER: char = #phase_letter;

            /// All diagnostic definitions for this phase.
            pub const ALL: &'static [destack_artifact::DiagnosticDefinition] = &[
                #(#diagnostic_defs),*
            ];

            /// All diagnostic codes for this phase.
            pub const ALL_CODES: &'static [&'static str] = &[
                #(#all_codes),*
            ];

            /// All codes that can be controlled by directives.
            pub const DIRECTIVE_CODES: &'static [&'static str] = &[
                #(#directive_codes),*
            ];

            /// Check if a code string is valid for this diagnostic type.
            #[inline]
            pub fn is_valid_code(code: &str) -> bool {
                Self::ALL_CODES.contains(&code)
            }

            /// Check if a code string can be controlled by directives.
            #[inline]
            pub fn is_directive_code(code: &str) -> bool {
                Self::DIRECTIVE_CODES.contains(&code)
            }

            /// Get the definition for a code, if valid.
            pub fn def_for_code(
                code: &str,
            ) -> Option<&'static destack_artifact::DiagnosticDefinition> {
                Self::ALL.iter().find(|def| def.code == code)
            }

            /// Get the numeric sub-code of the diagnostic.
            #[inline]
            pub fn sub_code(&self) -> u16 {
                match self {
                    #(#sub_code_arms),*
                }
            }

            /// Get the full diagnostic code.
            #[inline]
            pub fn code(&self) -> &'static str {
                match self {
                    #(#code_arms),*
                }
            }

            /// Check whether directives are allowed for this diagnostic.
            #[inline]
            pub fn is_directive(&self) -> bool {
                match self {
                    #(#directive_arms),*
                }
            }

            /// Get the site for this diagnostic.
            pub fn site(&self) -> Result<destack_artifact::DiagnosticSite, destack_artifact::DiagnosticError> {
                match self {
                    #(#site_arms),*
                }
            }

            /// Get the message for this diagnostic.
            #[allow(unused_variables)]
            pub fn message<R>(
                &self,
                #formatter_name: &destack_artifact::DiagnosticFormatter<'_, R>,
            ) -> Result<String, destack_artifact::DiagnosticError>
            where
                R: Copy + Eq + std::hash::Hash,
            {
                let message = match self {
                    #(#message_arms),*
                };

                Ok(message)
            }

            /// Build the source diagnostic for this provider diagnostic.
            pub fn diagnostic<R>(
                &self,
                #context_name: &dyn destack_artifact::DiagnosticContext<Revision = R>,
            ) -> Result<destack_source::Diagnostic, destack_artifact::DiagnosticError>
            where
                R: Copy + Eq + std::hash::Hash,
            {
                let primary_site = self.site()?;
                let primary_anchor = #context_name.anchor(&primary_site)?;
                let #formatter_name =
                    destack_artifact::DiagnosticFormatter::new(#context_name, &primary_anchor);
                let message = self.message(&#formatter_name)?;
                let primary_span = #context_name.span(&primary_anchor)?;

                let __diagnostic = destack_source::Diagnostic::new(
                    self.code(),
                    destack_source::DiagnosticSeverity::#severity_ident,
                    message.clone(),
                    destack_source::DiagnosticLabel::message(primary_span, message),
                );

                Ok(__diagnostic)
            }

            /// Start a decorated diagnostic builder.
            pub fn builder(self) -> destack_artifact::DiagnosticBuilder<Self> {
                destack_artifact::DiagnosticBuilder::new(self)
            }

            /// Add one secondary source label.
            pub fn label(
                self,
                site: impl Into<destack_artifact::DiagnosticSite>,
                message: impl Into<String>,
            ) -> destack_artifact::DiagnosticBuilder<Self> {
                self.builder().label(site, message)
            }

            /// Add one note.
            pub fn note(
                self,
                note: impl Into<destack_source::DiagnosticNote>,
            ) -> destack_artifact::DiagnosticBuilder<Self> {
                self.builder().note(note)
            }

            /// Add one help message.
            pub fn help(
                self,
                help: impl Into<destack_source::DiagnosticHelp>,
            ) -> destack_artifact::DiagnosticBuilder<Self> {
                self.builder().help(help)
            }

            /// Add one source edit suggestion.
            pub fn suggestion(
                self,
                suggestion: destack_source::DiagnosticSuggestion,
            ) -> destack_artifact::DiagnosticBuilder<Self> {
                self.builder().suggestion(suggestion)
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.code())
            }
        }

        #into_impl

        impl<R> destack_artifact::IntoDiagnostic<R> for #enum_name
        where
            R: Copy + Eq + std::hash::Hash,
        {
            fn into_diagnostic(
                &self,
                #context_name: &dyn destack_artifact::DiagnosticContext<Revision = R>,
            ) -> Result<destack_source::Diagnostic, destack_artifact::DiagnosticError> {
                self.diagnostic(#context_name)
            }
        }

        impl<R> destack_artifact::DiagnosticDraft<R> for #enum_name
        where
            R: Copy + Eq + std::hash::Hash,
        {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn code(&self) -> &'static str {
                self.code()
            }

            fn severity(&self) -> destack_source::DiagnosticSeverity {
                destack_source::DiagnosticSeverity::#severity_ident
            }

            fn site(&self) -> Result<destack_artifact::DiagnosticSite, destack_artifact::DiagnosticError> {
                self.site()
            }

            fn is_directive(&self) -> bool {
                self.is_directive()
            }
        }

        #result_alias
    })
}
