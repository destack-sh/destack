use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
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
    /// Whether source controls may select this diagnostic family.
    is_controllable: bool,
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

    /// Return whether this field has type Option<T>.
    fn is_option(&self) -> bool {
        let Type::Path(ty) = &self.ty else {
            return false;
        };
        let Some(segment) = ty.path.segments.last() else {
            return false;
        };

        segment.ident == "Option"
    }
}

/// One validated diagnostic variant.
struct DiagnosticVariant {
    /// The variant name.
    name: Ident,
    /// The canonical diagnostic id.
    id: LitStr,
    /// The documentation description.
    description: String,
    /// The diagnostic message template.
    message: Option<String>,
    /// The optional diagnostic message template.
    optional_message: Option<String>,
    /// The static help template.
    help: Option<String>,
    /// The named fields carried by the variant.
    fields: Vec<DiagnosticField>,
}

impl DiagnosticVariant {
    /// Return the field with the given name when present.
    fn field(&self, name: &str) -> Option<&DiagnosticField> {
        self.fields.iter().find(|field| field.is_named(name))
    }
}

/// One validated diagnostic enum.
struct DiagnosticEnum {
    /// The enum name.
    name: Ident,
    /// The derive options.
    options: DiagnosticDeriveOptions,
    /// The diagnostic variants.
    variants: Vec<DiagnosticVariant>,
}

impl DiagnosticEnum {
    /// Return whether this diagnostic enum is an error family.
    fn is_error(&self) -> bool {
        self.options.severity == "Error"
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
    fields: Vec<MessageField>,
}

/// One diagnostic message template field.
#[derive(Clone)]
struct MessageField {
    /// The referenced diagnostic field.
    name: Ident,
    /// Whether the referenced field is optional.
    is_optional: bool,
}

/// Push a unique field name into a generated binding list.
fn push_unique_field(fields: &mut Vec<MessageField>, field: &DiagnosticField) {
    if !fields.iter().any(|existing| existing.name == field.name) {
        fields.push(MessageField {
            name: field.name.clone(),
            is_optional: field.is_option(),
        });
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
            push_unique_field(&mut used_fields, field);
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
        .map(|field| format_field_expr(&field.name, formatter_name))
        .collect();

    quote! { format!(#format, #(#format_args),*) }
}

/// Generate one match arm returning a variant's static help.
fn help_arm(variant: &DiagnosticVariant, formatter_name: &Ident) -> Result<TokenStream2> {
    let name = &variant.name;
    let Some(help) = &variant.help else {
        let pattern = variant_pattern(variant);
        return Ok(quote! { #pattern => None });
    };
    let template = parse_message_template(help, &variant.fields, variant.id.span())?;
    let format_expr = message_format_expr(&template, formatter_name);
    let used_fields = template
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect::<Vec<_>>();
    if used_fields.is_empty() {
        let pattern = variant_pattern(variant);
        return Ok(quote! { #pattern => Some(#format_expr) });
    }

    Ok(quote! { Self::#name { #(#used_fields,)* .. } => Some(#format_expr) })
}

/// Generate the formatting expression for one optional message template.
fn optional_message_format_expr(
    template: &MessageTemplate,
    formatter_name: &Ident,
    span: Span,
) -> Result<TokenStream2> {
    let format = &template.format;
    let option_fields = template
        .fields
        .iter()
        .filter(|field| field.is_optional)
        .collect::<Vec<_>>();
    if option_fields.is_empty() {
        return Err(Error::new(
            span,
            "optional diagnostic message must reference at least one Option field",
        ));
    }

    let option_names = option_fields
        .iter()
        .map(|field| &field.name)
        .collect::<Vec<_>>();
    let option_bindings = option_names
        .iter()
        .map(|field| format_ident!("__diagnostic_optional_{field}"))
        .collect::<Vec<_>>();
    let option_refs = option_names
        .iter()
        .map(|field| quote! { #field.as_ref() })
        .collect::<Vec<_>>();
    let format_args = template
        .fields
        .iter()
        .map(|field| {
            let field_name = &field.name;
            if field.is_optional {
                let binding = format_ident!("__diagnostic_optional_{field_name}");

                quote! { #binding.format_diagnostic(#formatter_name)? }
            } else {
                format_field_expr(field_name, formatter_name)
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        if let (#(Some(#option_bindings),)*) = (#(#option_refs,)*) {
            Some(format!(#format, #(#format_args),*))
        } else {
            None
        }
    })
}

/// Extract the diagnostic description from the first documentation line.
fn extract_description(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if attr.path().is_ident("doc")
            && let Meta::NameValue(value) = &attr.meta
            && let Expr::Lit(expression) = &value.value
            && let Lit::Str(string) = &expression.lit
        {
            let line = string.value();
            let line = line.trim().to_string();
            if !line.is_empty() {
                return Some(line);
            }
        }

        None
    })
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
    let mut is_controllable = false;
    let mut into = None;

    // parse severity, phase, and optional aggregate target
    attribute.parse_nested_meta(|meta| {
        if meta.path.is_ident("severity") {
            severity = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("phase") {
            phase = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("controllable") {
            is_controllable = true;
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
        is_controllable,
        into,
    })
}

/// Parse one variant-level diagnostic attribute.
fn parse_variant(variant: &syn::Variant) -> Result<DiagnosticVariant> {
    let name = variant.ident.clone();
    let description = extract_description(&variant.attrs).ok_or_else(|| {
        Error::new(
            name.span(),
            "diagnostic variant must have a documentation description",
        )
    })?;
    let attribute = variant
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("diagnostic"))
        .ok_or_else(|| Error::new(name.span(), "missing #[diagnostic(id = \"...\")] attribute"))?;

    let mut id = None;
    let mut message = None;
    let mut optional_message = None;
    let mut help = None;

    // parse the id and messages
    attribute.parse_nested_meta(|meta| {
        if meta.path.is_ident("id") {
            id = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("message") {
            let value: LitStr = meta.value()?.parse()?;
            message = Some(value.value());
        } else if meta.path.is_ident("optional_message") {
            let value: LitStr = meta.value()?.parse()?;
            optional_message = Some(value.value());
        } else if meta.path.is_ident("help") {
            let value: LitStr = meta.value()?.parse()?;
            help = Some(value.value());
        } else {
            return Err(meta.error("unknown diagnostic variant option"));
        }

        Ok(())
    })?;

    let id = id.ok_or_else(|| Error::new(name.span(), "missing diagnostic id"))?;
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
        id,
        description,
        message,
        optional_message,
        help,
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
        let template = parse_message_template(message, &variant.fields, variant.id.span())?;
        let format_expr = message_format_expr(&template, formatter_name);
        let mut used_fields = template
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>();
        let optional_format_expr = match &variant.optional_message {
            Some(optional_message) => {
                let optional_template =
                    parse_message_template(optional_message, &variant.fields, variant.id.span())?;
                for field in &optional_template.fields {
                    if !used_fields.iter().any(|existing| existing == &field.name) {
                        used_fields.push(field.name.clone());
                    }
                }
                let optional_format_expr = optional_message_format_expr(
                    &optional_template,
                    formatter_name,
                    variant.id.span(),
                )?;

                quote! {
                    let mut message = #format_expr;
                    if let Some(optional_message) = #optional_format_expr {
                        message.push_str(&optional_message);
                    }

                    message
                }
            }
            None => format_expr,
        };

        if used_fields.is_empty() {
            let pattern = variant_pattern(variant);
            Ok(quote! { #pattern => { #optional_format_expr } })
        } else {
            Ok(quote! { Self::#name { #(#used_fields,)* .. } => { #optional_format_expr } })
        }
    } else {
        Err(Error::new(variant.id.span(), "missing diagnostic message"))
    }
}

/// Generate one diagnostic definition.
fn diagnostic_definition(
    variant: &DiagnosticVariant,
    is_error: bool,
    is_controllable: bool,
) -> TokenStream2 {
    let id = variant.id.value();
    let description = &variant.description;
    let constructor = if is_error {
        format_ident!("error")
    } else if is_controllable {
        format_ident!("controllable_warning")
    } else {
        format_ident!("warning")
    };

    quote! {
        tspp_source::DiagnosticDefinition::#constructor(
            #id,
            #description,
        )
    }
}

/// Generate one diagnostic id match arm.
fn diagnostic_id_arm(variant: &DiagnosticVariant) -> TokenStream2 {
    let id = variant.id.value();
    let pattern = variant_pattern(variant);

    quote! { #pattern => #id }
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
    let severity_name = options.severity.to_string();
    let phase = options.phase.to_string();
    let expected_name = format!("{phase}{severity_name}");

    // accept only final diagnostic severities
    if severity_name != "Error" && severity_name != "Warning" {
        return Err(Error::new(
            options.severity.span(),
            format!("unsupported diagnostic severity `{severity_name}`"),
        ));
    }

    // only warnings may be controlled
    if options.is_controllable && severity_name != "Warning" {
        return Err(Error::new(
            options.severity.span(),
            "only warning diagnostics may be controllable",
        ));
    }

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
    let mut ids = HashSet::new();

    // parse variants and validate unique ids
    for variant in &data.variants {
        let variant = parse_variant(variant)?;
        let id = variant.id.value();
        if !ids.insert(id.clone()) {
            return Err(Error::new(
                variant.id.span(),
                format!("duplicate diagnostic id \"{id}\""),
            ));
        }

        variants.push(variant);
    }

    Ok(DiagnosticEnum {
        name,
        options,
        variants,
    })
}

/// Derive one provider diagnostic enum.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_diagnostic(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derive one provider diagnostic enum.
fn expand_diagnostic(input: DeriveInput) -> Result<TokenStream2> {
    let diagnostic = parse_diagnostic_enum(input)?;
    let enum_name = &diagnostic.name;
    let options = &diagnostic.options;
    let variants = &diagnostic.variants;

    let result_name = format_ident!("{}Result", options.phase);
    let is_error = diagnostic.is_error();
    let severity_ident = &options.severity;
    let context_name = format_ident!("__diagnostic_context");
    let formatter_name = format_ident!("__diagnostic_formatter");

    let diagnostic_definitions: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| diagnostic_definition(variant, is_error, options.is_controllable))
        .collect();

    let diagnostic_id_arms: Vec<TokenStream2> = variants.iter().map(diagnostic_id_arm).collect();
    let primary_anchor_arms: Vec<TokenStream2> = variants
        .iter()
        .map(primary_anchor_match_arm)
        .collect::<Result<Vec<_>>>()?;
    let message_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| message_arm(variant, &formatter_name))
        .collect::<Result<Vec<_>>>()?;

    let id_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            match self {
                #(#diagnostic_id_arms),*
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
    let help_arms = variants
        .iter()
        .map(|variant| help_arm(variant, &formatter_name))
        .collect::<Result<Vec<_>>>()?;
    let help_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            let help = match self {
                #(#help_arms),*
            };

            Ok(help)
        }
    };
    let diagnostic_body = if variants.is_empty() {
        quote! { match *self {} }
    } else {
        quote! {
            let primary_anchor = self.anchor();
            let #formatter_name =
                tspp_artifact::DiagnosticFormatter::new(#context_name);
            let message = self.message(&#formatter_name)?;
            // the header carries the message, so the primary span stays bare
            let primary_label = #context_name.label(&primary_anchor, None)?;

            let __diagnostic = tspp_source::Diagnostic::new(
                self.id(),
                tspp_source::DiagnosticSeverity::#severity_ident,
                message,
                primary_label,
            );
            let __diagnostic = match self.help_message(&#formatter_name)? {
                Some(help) => __diagnostic.help(help),
                None => __diagnostic,
            };

            Ok(__diagnostic)
        }
    };

    let result_alias = result_alias(enum_name, &result_name, is_error);
    let into_impl = aggregate_impl(enum_name, options, is_error);

    Ok(quote! {
        use tspp_artifact::DiagnosticFormat as _;

        impl #enum_name {
            /// All diagnostic definitions for this phase.
            pub const ALL: &'static [tspp_source::DiagnosticDefinition] = &[
                #(#diagnostic_definitions),*
            ];

            /// Return the canonical diagnostic id.
            #[inline]
            pub fn id(&self) -> &'static str {
                #id_body
            }

            /// Return the anchor for this diagnostic.
            pub fn anchor(
                &self,
            ) -> tspp_artifact::DiagnosticAnchor {
                #anchor_body
            }

            /// Return the message for this diagnostic.
            #[allow(unused_variables)]
            pub fn message(
                &self,
                #formatter_name: &tspp_artifact::DiagnosticFormatter<'_>,
            ) -> Result<String, tspp_artifact::DiagnosticError>
            {
                #message_body
            }

            /// Return the static help for this diagnostic.
            #[allow(unused_variables)]
            pub fn help_message(
                &self,
                #formatter_name: &tspp_artifact::DiagnosticFormatter<'_>,
            ) -> Result<Option<String>, tspp_artifact::DiagnosticError>
            {
                #help_body
            }

            /// Return the source diagnostic for this provider diagnostic.
            pub fn diagnostic(
                &self,
                #context_name: &dyn tspp_artifact::DiagnosticContext,
            ) -> Result<tspp_source::Diagnostic, tspp_artifact::DiagnosticError>
            {
                #diagnostic_body
            }

            /// Start a decorated diagnostic builder.
            pub fn builder(self) -> tspp_artifact::DiagnosticBuilder<Self> {
                tspp_artifact::DiagnosticBuilder::new(self)
            }

            /// Add one secondary source label.
            pub fn label(
                self,
                anchor: impl Into<tspp_artifact::DiagnosticAnchor>,
                message: impl Into<String>,
            ) -> tspp_artifact::DiagnosticBuilder<Self> {
                self.builder().label(anchor, message)
            }

            /// Add one note.
            pub fn note(
                self,
                note: impl Into<tspp_source::DiagnosticNote>,
            ) -> tspp_artifact::DiagnosticBuilder<Self> {
                self.builder().note(note)
            }

            /// Add one help message.
            pub fn help(
                self,
                help: impl Into<tspp_source::DiagnosticHelp>,
            ) -> tspp_artifact::DiagnosticBuilder<Self> {
                self.builder().help(help)
            }

            /// Add one source edit suggestion.
            pub fn suggestion(
                self,
                suggestion: tspp_source::DiagnosticSuggestion,
            ) -> tspp_artifact::DiagnosticBuilder<Self> {
                self.builder().suggestion(suggestion)
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.id())
            }
        }

        #into_impl

        impl tspp_artifact::ToDiagnostic for #enum_name {
            fn to_diagnostic(
                &self,
                #context_name: &dyn tspp_artifact::DiagnosticContext,
            ) -> Result<tspp_source::Diagnostic, tspp_artifact::DiagnosticError> {
                self.diagnostic(#context_name)
            }
        }

        #result_alias
    })
}
