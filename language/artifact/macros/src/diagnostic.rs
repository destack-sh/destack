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

/// One named enum variant field.
struct DiagnosticField {
    /// The field name.
    name: Ident,
    /// The field type.
    ty: Type,
}

impl DiagnosticField {
    /// Return whether this field has the given name.
    fn is_named(&self, name: &str) -> bool {
        self.name == name
    }

    /// Return whether this field has type DiagnosticAnchor.
    fn is_diagnostic_anchor(&self) -> bool {
        let Type::Path(ty) = &self.ty else {
            return false;
        };
        let Some(segment) = ty.path.segments.last() else {
            return false;
        };

        segment.ident == "DiagnosticAnchor"
    }
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
    /// The named fields carried by the variant.
    fields: Vec<DiagnosticField>,
}

impl DiagnosticVariant {
    /// Return the field with the given name when present.
    fn field(&self, name: &str) -> Option<&DiagnosticField> {
        self.fields.iter().find(|field| field.is_named(name))
    }
}

/// Validated diagnostic enum data.
struct DiagnosticEnum {
    /// The enum name.
    name: Ident,
    /// The derive options.
    options: DiagnosticDeriveOptions,
    /// The severity name.
    severity_name: String,
    /// The phase name.
    phase: String,
    /// The phase prefix.
    phase_letter: char,
    /// The diagnostic variants.
    variants: Vec<DiagnosticVariant>,
}

impl DiagnosticEnum {
    /// Return whether this diagnostic enum is an error family.
    fn is_error(&self) -> bool {
        self.severity_name == "Error"
    }
}

/// Map a compiler phase name to its single-letter code.
fn phase_letter(phase: &Ident) -> Result<char> {
    let letter = match phase.to_string().as_str() {
        "Declare" => 'D',
        "Import" => 'I',
        "Expand" => 'X',
        "Export" => 'T',
        "Check" => 'C',
        "Elaborate" => 'E',
        "Materialize" => 'M',
        "Lower" => 'L',
        "Verify" => 'V',
        "Optimize" => 'O',
        "Generate" => 'G',
        "Link" => 'K',
        "Lint" => 'L',
        other => {
            return Err(Error::new(
                phase.span(),
                format!("unsupported diagnostic phase `{other}`"),
            ));
        }
    };

    Ok(letter)
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

/// Parsed diagnostic message template.
struct MessageTemplate {
    /// The Rust format string.
    format: String,
    /// The referenced diagnostic fields.
    fields: Vec<Ident>,
}

/// Push a unique field name into a generated binding list.
fn push_unique_field(fields: &mut Vec<Ident>, field: &Ident) {
    if !fields.iter().any(|existing| existing == field) {
        fields.push(field.clone());
    }
}

/// Parse one diagnostic message template.
fn parse_message_template(
    template: &str,
    fields: &[DiagnosticField],
    span: Span,
) -> Result<MessageTemplate> {
    let mut used_fields = Vec::new();
    let mut format = String::new();
    let mut chars = template.chars().peekable();

    // scan format text and field placeholders
    while let Some(character) = chars.next() {
        if character == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                format.push_str("{{");
                continue;
            }

            let mut field_name = String::new();
            let mut found_end = false;
            while let Some(&character) = chars.peek() {
                if character == '}' {
                    chars.next();
                    found_end = true;
                    break;
                }

                if let Some(character) = chars.next() {
                    field_name.push(character);
                }
            }

            if !found_end {
                return Err(Error::new(span, "unmatched `{` in diagnostic message"));
            }

            if field_name.is_empty() {
                return Err(Error::new(span, "empty diagnostic message placeholder"));
            }

            let Some(field) = fields.iter().find(|field| field.is_named(&field_name)) else {
                return Err(Error::new(
                    span,
                    format!("unknown field `{field_name}` in format string"),
                ));
            };

            format.push_str("{}");
            push_unique_field(&mut used_fields, &field.name);
        }
        // escape a literal closing brace
        else if character == '}' && chars.peek() == Some(&'}') {
            chars.next();
            format.push_str("}}");
        } else if character == '}' {
            return Err(Error::new(span, "unmatched `}` in diagnostic message"));
        } else {
            format.push(character);
        }
    }

    Ok(MessageTemplate {
        format,
        fields: used_fields,
    })
}

/// Generate the formatting expression for one parsed message template.
fn message_format_expr(template: &MessageTemplate, formatter_name: &Ident) -> TokenStream2 {
    let format = &template.format;
    let format_args: Vec<TokenStream2> = template
        .fields
        .iter()
        .map(|field| format_field_expr(field, formatter_name))
        .collect();

    quote! { format!(#format, #(#format_args),*) }
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
fn parse_variant(variant: &syn::Variant, severity: char, phase: char) -> Result<DiagnosticVariant> {
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
    // parse code and message
    attribute.parse_nested_meta(|meta| {
        if meta.path.is_ident("code") {
            code = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("message") {
            let value: LitStr = meta.value()?.parse()?;
            message = Some(value.value());
        } else {
            return Err(meta.error("unknown diagnostic variant option"));
        }

        Ok(())
    })?;

    let code = code.ok_or_else(|| Error::new(name.span(), "missing diagnostic code"))?;
    let sub_code = parse_diagnostic_code(&code, severity, phase)?;
    let fields = match &variant.fields {
        Fields::Named(named) => {
            let mut fields = Vec::new();
            for field in &named.named {
                let Some(field_name) = field.ident.clone() else {
                    return Err(Error::new(name.span(), "named field without identifier"));
                };
                fields.push(DiagnosticField {
                    name: field_name,
                    ty: field.ty.clone(),
                });
            }
            fields
        }
        Fields::Unnamed(_) => return Err(Error::new(name.span(), "tuple variants not supported")),
        Fields::Unit => Vec::new(),
    };

    Ok(DiagnosticVariant {
        name,
        code,
        description,
        sub_code,
        message,
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

/// Generate one primary anchor match arm.
fn primary_anchor_match_arm(variant: &DiagnosticVariant) -> Result<TokenStream2> {
    let name = &variant.name;

    let Some(anchor) = variant.field("anchor") else {
        return Err(Error::new(
            variant.name.span(),
            "diagnostic variant must have an `anchor: DiagnosticAnchor` field",
        ));
    };

    if !anchor.is_diagnostic_anchor() {
        return Err(Error::new(
            anchor.name.span(),
            "diagnostic `anchor` field must have type `DiagnosticAnchor`",
        ));
    }

    Ok(quote! {
        Self::#name { anchor, .. } => {
            anchor.clone()
        }
    })
}

/// Generate one message match arm.
fn message_arm(variant: &DiagnosticVariant, formatter_name: &Ident) -> Result<TokenStream2> {
    let name = &variant.name;

    if let Some(message) = &variant.message {
        let template = parse_message_template(message, &variant.fields, variant.code.span())?;
        let format_expr = message_format_expr(&template, formatter_name);

        if template.fields.is_empty() {
            let pattern = variant_pattern(variant);
            Ok(quote! { #pattern => #format_expr })
        } else {
            let used_fields = &template.fields;

            Ok(quote! { Self::#name { #(#used_fields,)* .. } => #format_expr })
        }
    } else {
        Err(Error::new(
            variant.code.span(),
            "missing diagnostic message",
        ))
    }
}

/// Generate one diagnostic definition expression.
fn definition_expr(variant: &DiagnosticVariant) -> TokenStream2 {
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
}

/// Generate one diagnostic code match arm.
fn code_arm(variant: &DiagnosticVariant) -> TokenStream2 {
    let code = variant.code.value();
    let pattern = variant_pattern(variant);

    quote! { #pattern => #code }
}

/// Generate one diagnostic sub-code match arm.
fn sub_code_arm(variant: &DiagnosticVariant) -> TokenStream2 {
    let sub_code = variant.sub_code;
    let pattern = variant_pattern(variant);

    quote! { #pattern => #sub_code }
}

/// Generate the result alias for error diagnostics.
fn result_alias(enum_name: &Ident, result_name: &Ident, is_error: bool) -> TokenStream2 {
    if !is_error {
        return quote! {};
    }

    quote! {
        /// Result type for this phase.
        pub type #result_name<T> = Result<T, #enum_name>;
    }
}

/// Generate the aggregate enum conversion when requested.
fn aggregate_impl(
    enum_name: &Ident,
    options: &DiagnosticDeriveOptions,
    is_error: bool,
) -> TokenStream2 {
    let Some(into) = &options.into else {
        return quote! {};
    };
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
}

/// Parse and validate one diagnostic enum derive input.
fn parse_diagnostic_enum(input: DeriveInput) -> Result<DiagnosticEnum> {
    let name = input.ident.clone();
    let options = parse_options(&input)?;
    let severity = severity_prefix(&options.severity)?;
    let severity_name = options.severity.to_string();
    let phase = options.phase.to_string();
    let phase_letter = phase_letter(&options.phase)?;
    let expected_name = format!("{phase}{severity_name}");

    // validate enum naming convention
    if name != expected_name {
        return Err(Error::new(
            name.span(),
            format!("enum name must be `{expected_name}` for phase `{phase}`"),
        ));
    }

    let data = match input.data {
        Data::Enum(data) => data,
        _ => return Err(Error::new(name.span(), "Diagnostic only works on enums")),
    };

    let mut variants = Vec::new();
    let mut codes = HashSet::new();

    // parse variants and validate stable codes
    for variant in &data.variants {
        let variant = parse_variant(variant, severity, phase_letter)?;
        let code = variant.code.value();
        if !codes.insert(code.clone()) {
            return Err(Error::new(
                variant.code.span(),
                format!("duplicate diagnostic code \"{code}\""),
            ));
        }

        variants.push(variant);
    }

    Ok(DiagnosticEnum {
        name,
        options,
        severity_name,
        phase,
        phase_letter,
        variants,
    })
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
    let diagnostic = parse_diagnostic_enum(input)?;
    let enum_name = &diagnostic.name;
    let options = &diagnostic.options;
    let variants = &diagnostic.variants;

    let result_name = format_ident!("{}Result", diagnostic.phase);
    let is_error = diagnostic.is_error();
    let phase_letter = diagnostic.phase_letter;
    let severity_ident = &options.severity;
    let context_name = format_ident!("__diagnostic_context");
    let formatter_name = format_ident!("__diagnostic_formatter");

    let all_codes: Vec<String> = variants
        .iter()
        .map(|variant| variant.code.value())
        .collect();
    let diagnostic_defs: Vec<TokenStream2> = variants.iter().map(definition_expr).collect();

    let code_arms: Vec<TokenStream2> = variants.iter().map(code_arm).collect();
    let sub_code_arms: Vec<TokenStream2> = variants.iter().map(sub_code_arm).collect();
    let primary_anchor_arms: Vec<TokenStream2> = variants
        .iter()
        .map(primary_anchor_match_arm)
        .collect::<Result<Vec<_>>>()?;
    let message_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| message_arm(variant, &formatter_name))
        .collect::<Result<Vec<_>>>()?;

    let sub_code_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#sub_code_arms),*
            }
        }
    };
    let code_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#code_arms),*
            }
        }
    };
    let anchor_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#primary_anchor_arms),*
            }
        }
    };
    let message_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            let message = match self {
                #(#message_arms),*
            };

            Ok(message)
        }
    };
    let diagnostic_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            let primary_anchor = self.anchor();
            let #formatter_name =
                destack_artifact::DiagnosticFormatter::new(#context_name);
            let message = self.message(&#formatter_name)?;
            let primary_label = #context_name.label(&primary_anchor, Some(message.clone()))?;

            let __diagnostic = destack_source::Diagnostic::new(
                self.code(),
                destack_source::DiagnosticSeverity::#severity_ident,
                message.clone(),
                primary_label,
            );

            Ok(__diagnostic)
        }
    };

    let result_alias = result_alias(enum_name, &result_name, is_error);
    let into_impl = aggregate_impl(enum_name, options, is_error);

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

            /// Return whether a code string is valid for this diagnostic type.
            #[inline]
            pub fn is_valid_code(code: &str) -> bool {
                Self::ALL_CODES.contains(&code)
            }

            /// Return the definition for a code, if valid.
            pub fn definition(
                code: &str,
            ) -> Option<&'static destack_artifact::DiagnosticDefinition> {
                Self::ALL.iter().find(|def| def.code == code)
            }

            /// Return the numeric sub-code of the diagnostic.
            #[inline]
            pub fn sub_code(&self) -> u16 {
                #sub_code_body
            }

            /// Return the full diagnostic code.
            #[inline]
            pub fn code(&self) -> &'static str {
                #code_body
            }

            /// Return the anchor for this diagnostic.
            pub fn anchor(
                &self,
            ) -> destack_artifact::DiagnosticAnchor {
                #anchor_body
            }

            /// Return the message for this diagnostic.
            #[allow(unused_variables)]
            pub fn message(
                &self,
                #formatter_name: &destack_artifact::DiagnosticFormatter<'_>,
            ) -> Result<String, destack_artifact::DiagnosticError>
            {
                #message_body
            }

            /// Return the source diagnostic for this provider diagnostic.
            pub fn diagnostic(
                &self,
                #context_name: &dyn destack_artifact::DiagnosticContext,
            ) -> Result<destack_source::Diagnostic, destack_artifact::DiagnosticError>
            {
                #diagnostic_body
            }

            /// Start a decorated diagnostic builder.
            pub fn builder(self) -> destack_artifact::DiagnosticBuilder<Self> {
                destack_artifact::DiagnosticBuilder::new(self)
            }

            /// Add one secondary source label.
            pub fn label(
                self,
                anchor: impl Into<destack_artifact::DiagnosticAnchor>,
                message: impl Into<String>,
            ) -> destack_artifact::DiagnosticBuilder<Self> {
                self.builder().label(anchor, message)
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

        impl destack_artifact::ToDiagnostic for #enum_name {
            fn to_diagnostic(
                &self,
                #context_name: &dyn destack_artifact::DiagnosticContext,
            ) -> Result<destack_source::Diagnostic, destack_artifact::DiagnosticError> {
                self.diagnostic(#context_name)
            }
        }

        #result_alias
    })
}
