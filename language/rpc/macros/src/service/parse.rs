use proc_macro2::TokenStream;
use quote::format_ident;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, ExprLit, FnArg, ItemTrait, Lit, LitStr, Meta, ReturnType, Token, TraitItem,
    TraitItemFn, Type, Visibility,
};

/// One parsed service declaration.
pub(crate) struct Service {
    /// Service documentation and visibility.
    pub(crate) attributes: Vec<Attribute>,
    /// Rust service trait name.
    pub(crate) trait_name: syn::Ident,
    /// Generated client name.
    pub(crate) client_name: syn::Ident,
    /// Generated server name.
    pub(crate) server_name: syn::Ident,
    /// Generated method descriptor table name.
    pub(crate) methods_name: syn::Ident,
    /// Service visibility.
    pub(crate) visibility: Visibility,
    /// Canonical qualified service name.
    pub(crate) name: LitStr,
    /// Declared service methods.
    pub(crate) methods: Vec<Method>,
}

/// One parsed service method declaration.
pub(crate) struct Method {
    /// Method documentation and non-RPC attributes.
    pub(crate) attributes: Vec<Attribute>,
    /// Rust method name.
    pub(crate) rust_name: syn::Ident,
    /// Canonical stable method name.
    pub(crate) name: LitStr,
    /// Initial request value type.
    pub(crate) request: Type,
    /// Terminal response value type.
    pub(crate) response: Type,
    /// Caller-to-service stream item type.
    pub(crate) request_stream: Option<Type>,
    /// Service-to-caller stream item type.
    pub(crate) response_stream: Option<Type>,
    /// Repeated-call behavior.
    pub(crate) idempotency: Idempotency,
}

/// Parsed method idempotency.
#[derive(Clone, Copy)]
pub(crate) enum Idempotency {
    /// No repeated-call guarantee.
    Unknown,
    /// Repeated calls have one intended effect.
    Idempotent,
    /// Calls do not change observable application state.
    NoSideEffects,
}

impl Service {
    /// Parse one service declaration and its macro options.
    pub(crate) fn parse(attribute: TokenStream, item: ItemTrait) -> syn::Result<Self> {
        let options = parse_options(attribute)?;
        let name = required_string(&options, "name", item.trait_token)?;

        // require one concrete trait declaration
        if !item.generics.params.is_empty() || !item.supertraits.is_empty() {
            return Err(syn::Error::new_spanned(
                &item.generics,
                "RPC service declarations cannot have generics or supertraits",
            ));
        }

        // parse every declared RPC method
        let methods = item
            .items
            .into_iter()
            .map(|item| match item {
                TraitItem::Fn(method) => Method::parse(method),
                other => Err(syn::Error::new_spanned(
                    other,
                    "RPC services may contain only methods",
                )),
            })
            .collect::<syn::Result<Vec<_>>>()?;
        if methods.is_empty() {
            return Err(syn::Error::new_spanned(
                item.ident,
                "RPC services must declare at least one method",
            ));
        }

        // derive symmetrical generated type names
        let trait_name = item.ident;
        let trait_text = trait_name.to_string();
        let Some(base) = trait_text.strip_suffix("Service") else {
            return Err(syn::Error::new_spanned(
                trait_name,
                "RPC service trait names must end in `Service`",
            ));
        };
        let client_name = format_ident!("{base}Client");
        let server_name = format_ident!("{base}Server");
        let methods_name = format_ident!("{base}Methods");

        Ok(Self {
            attributes: item.attrs,
            trait_name,
            client_name,
            server_name,
            methods_name,
            visibility: item.vis,
            name,
            methods,
        })
    }
}

impl Method {
    /// Parse one service method declaration.
    fn parse(method: TraitItemFn) -> syn::Result<Self> {
        let Some(attribute) = method
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("rpc"))
        else {
            return Err(syn::Error::new_spanned(
                method.sig.ident,
                "RPC service methods require `#[rpc(...)]`",
            ));
        };
        let options = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        let name = required_string(&options, "name", attribute)?;

        // require one initial request value
        if method.sig.asyncness.is_some()
            || method.sig.constness.is_some()
            || method.sig.unsafety.is_some()
            || method.sig.abi.is_some()
            || !method.sig.generics.params.is_empty()
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "RPC method declarations must be plain non-generic functions",
            ));
        }
        let mut inputs = method.sig.inputs.into_iter();
        let Some(FnArg::Typed(request)) = inputs.next() else {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "RPC methods require one request value",
            ));
        };
        if inputs.next().is_some() {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "RPC method declarations accept exactly one request value",
            ));
        }
        let ReturnType::Type(_, response) = method.sig.output else {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "RPC methods require one response value",
            ));
        };
        if method.default.is_some() {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "RPC method declarations cannot have implementations",
            ));
        }

        // reserve the generated client operations
        let rust_name = method.sig.ident.to_string();
        if matches!(
            rust_name.as_str(),
            "connect" | "new" | "schema" | "service_schema"
        ) {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "RPC method name conflicts with a generated client operation",
            ));
        }

        // parse optional stream and behavior declarations
        let request_stream = optional_type(&options, "request_stream")?;
        let response_stream = optional_type(&options, "response_stream")?;
        let idempotency = optional_string(&options, "idempotency")?
            .map(|value| Idempotency::parse(&value))
            .transpose()?
            .unwrap_or(Idempotency::Unknown);
        reject_unknown_options(
            &options,
            &["name", "request_stream", "response_stream", "idempotency"],
        )?;

        let attributes = method
            .attrs
            .into_iter()
            .filter(|attribute| !attribute.path().is_ident("rpc"))
            .collect();

        Ok(Self {
            attributes,
            rust_name: method.sig.ident,
            name,
            request: *request.ty,
            response: *response,
            request_stream,
            response_stream,
            idempotency,
        })
    }
}

impl Idempotency {
    /// Parse one canonical idempotency name.
    fn parse(value: &LitStr) -> syn::Result<Self> {
        match value.value().as_str() {
            "unknown" => Ok(Self::Unknown),
            "idempotent" => Ok(Self::Idempotent),
            "no_side_effects" => Ok(Self::NoSideEffects),
            _ => Err(syn::Error::new_spanned(
                value,
                "idempotency must be `unknown`, `idempotent`, or `no_side_effects`",
            )),
        }
    }
}

/// Parse comma-separated macro options.
fn parse_options(tokens: TokenStream) -> syn::Result<Punctuated<Meta, Token![,]>> {
    Punctuated::<Meta, Token![,]>::parse_terminated.parse2(tokens)
}

/// Return one required string option.
fn required_string(
    options: &Punctuated<Meta, Token![,]>,
    name: &str,
    span: impl quote::ToTokens,
) -> syn::Result<LitStr> {
    optional_string(options, name)?
        .ok_or_else(|| syn::Error::new_spanned(span, format!("missing required `{name}` option")))
}

/// Return one optional string option.
fn optional_string(
    options: &Punctuated<Meta, Token![,]>,
    name: &str,
) -> syn::Result<Option<LitStr>> {
    let mut value = None;

    for option in options {
        if !option.path().is_ident(name) {
            continue;
        }
        if value.is_some() {
            return Err(syn::Error::new_spanned(option, "duplicate RPC option"));
        }
        let Meta::NameValue(option) = option else {
            return Err(syn::Error::new_spanned(option, "expected `name = value`"));
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Str(string),
            ..
        }) = &option.value
        else {
            return Err(syn::Error::new_spanned(&option.value, "expected a string"));
        };
        value = Some(string.clone());
    }

    Ok(value)
}

/// Return one optional type option.
fn optional_type(options: &Punctuated<Meta, Token![,]>, name: &str) -> syn::Result<Option<Type>> {
    let mut value = None;

    for option in options {
        if !option.path().is_ident(name) {
            continue;
        }
        if value.is_some() {
            return Err(syn::Error::new_spanned(option, "duplicate RPC option"));
        }
        let Meta::List(option) = option else {
            let message = format!("expected `{name}(Type)`");

            return Err(syn::Error::new_spanned(option, message));
        };
        value = Some(syn::parse2(option.tokens.clone())?);
    }

    Ok(value)
}

/// Reject options outside one exact declaration vocabulary.
fn reject_unknown_options(
    options: &Punctuated<Meta, Token![,]>,
    known: &[&str],
) -> syn::Result<()> {
    for option in options {
        if !known.iter().any(|name| option.path().is_ident(name)) {
            return Err(syn::Error::new_spanned(option, "unsupported RPC option"));
        }
    }

    Ok(())
}
