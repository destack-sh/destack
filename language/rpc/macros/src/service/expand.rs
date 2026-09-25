use proc_macro::TokenStream;

use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{ItemTrait, parse_macro_input};

use super::{Idempotency, Method, Service};

/// Expand one typed RPC service declaration.
pub(crate) fn expand(attribute: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemTrait);
    let attribute = proc_macro2::TokenStream::from(attribute);

    match Service::parse(attribute, item).map(expand_service) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Generate one complete service trait, client, and server.
fn expand_service(service: Service) -> Tokens {
    let Service {
        attributes,
        trait_name,
        client_name,
        server_name,
        methods_name,
        visibility,
        name,
        methods,
    } = service;
    let trait_methods = methods.iter().map(expand_trait_method);
    let arc_methods = methods
        .iter()
        .map(|method| expand_arc_method(&trait_name, method));
    let schema_methods = methods
        .iter()
        .map(|method| expand_schema_method(&name, method));
    let method_fields = methods.iter().map(expand_method_field);
    let method_names = methods.iter().map(|method| &method.rust_name);
    let client_methods = methods.iter().map(expand_client_method);
    let server_methods = methods.iter().map(expand_server_method);

    quote! {
        #(#attributes)*
        #visibility trait #trait_name: ::std::fmt::Debug + Send + Sync + 'static {
            #(#trait_methods)*
        }

        impl<T: #trait_name + ?Sized> #trait_name for ::std::sync::Arc<T> {
            #(#arc_methods)*
        }

        /// Exact typed method descriptors for the declared RPC service.
        #[derive(Debug, Clone, Copy)]
        struct #methods_name {
            #(#method_fields)*
        }

        impl #methods_name {
            /// Build this service's canonical schema and method descriptors.
            fn build() -> Result<
                (::tspp_rpc::ServiceSchema, Self),
                ::tspp_rpc::ServiceSchemaError,
            > {
                let mut types = ::tspp_serde::Schema::default();
                let mut methods = Vec::new();
                let service = ::tspp_rpc::ServiceId::for_name(#name);

                #(#schema_methods)*

                let schema = ::tspp_rpc::ServiceSchema::new(#name, methods, types)?;
                let methods = Self {
                    #(#method_names,)*
                };

                Ok((schema, methods))
            }
        }

        /// Typed client for the declared RPC service.
        #[derive(Debug, Clone)]
        #visibility struct #client_name {
            /// Negotiated RPC connection.
            connection: ::std::sync::Arc<::tspp_rpc::Connection>,
            /// Exact client service schema.
            schema: ::std::sync::Arc<::tspp_rpc::ServiceSchema>,
            /// Exact typed method descriptors.
            methods: #methods_name,
        }

        impl #client_name {
            /// Connect this typed client through one RPC transport.
            pub fn connect(
                transport: ::std::sync::Arc<dyn ::tspp_rpc::Transport>,
                options: ::tspp_rpc::ConnectionOptions,
            ) -> Result<Self, ::tspp_rpc::ConnectError> {
                let (schema, methods) = #methods_name::build()?;
                let schema = ::std::sync::Arc::new(schema);
                let connection = ::tspp_rpc::Connection::connect(
                    transport,
                    options,
                    vec![schema.id()],
                )?;
                connection.bind(&schema)?;

                Ok(Self {
                    connection,
                    schema,
                    methods,
                })
            }

            /// Create this typed client from one negotiated RPC connection.
            pub fn new(
                connection: ::std::sync::Arc<::tspp_rpc::Connection>,
            ) -> Result<Self, ::tspp_rpc::ConnectError> {
                let (schema, methods) = #methods_name::build()?;
                let schema = ::std::sync::Arc::new(schema);
                connection.bind(&schema)?;

                Ok(Self {
                    connection,
                    schema,
                    methods,
                })
            }

            /// Return this client's exact service schema.
            pub fn schema(&self) -> &::tspp_rpc::ServiceSchema {
                &self.schema
            }

            /// Build this client's canonical service schema.
            pub fn service_schema() -> Result<
                ::tspp_rpc::ServiceSchema,
                ::tspp_rpc::ServiceSchemaError,
            > {
                #methods_name::build().map(|(schema, _)| schema)
            }

            #(#client_methods)*
        }

        /// Typed server for the declared RPC service.
        #[derive(Debug)]
        #visibility struct #server_name<T> {
            /// Service implementation.
            service: T,
            /// Exact server service schema.
            schema: ::tspp_rpc::ServiceSchema,
            /// Exact typed method descriptors.
            methods: #methods_name,
        }

        impl<T: #trait_name> #server_name<T> {
            /// Create one typed service server.
            pub fn new(service: T) -> Result<Self, ::tspp_rpc::ServiceSchemaError> {
                let (schema, methods) = #methods_name::build()?;

                Ok(Self {
                    service,
                    schema,
                    methods,
                })
            }

            /// Return the implemented service.
            pub const fn get_ref(&self) -> &T {
                &self.service
            }

            /// Consume this server and return the implemented service.
            pub fn into_inner(self) -> T {
                self.service
            }
        }

        impl<T: #trait_name> ::tspp_rpc::Service for #server_name<T> {
            /// Return this server's canonical service schema.
            fn schema(&self) -> &::tspp_rpc::ServiceSchema {
                &self.schema
            }

            /// Dispatch one typed service request.
            fn call(
                &self,
                call: ::tspp_rpc::ServerCall,
            ) -> ::tspp_rpc::ServiceFuture<'_> {
                ::std::boxed::Box::pin(async move {
                    let mut call = call;

                    match call.method() {
                        #(#server_methods)*
                        _ => Err(::tspp_rpc::Status::new(
                            ::tspp_rpc::Code::Unimplemented,
                            "unknown RPC method",
                        ).into()),
                    }
                })
            }
        }
    }
}

/// Generate one service method forwarded through Arc ownership.
fn expand_arc_method(trait_name: &syn::Ident, method: &Method) -> Tokens {
    let name = &method.rust_name;
    let request_argument = method.request_stream.as_ref().map(|_| quote!(, requests));
    let response_argument = method.response_stream.as_ref().map(|_| quote!(, responses));
    let implementation = quote! {
        {
            <T as #trait_name>::#name(
                self.as_ref(),
                request
                #request_argument
                #response_argument
            )
        }
    };

    expand_method(method, implementation)
}

/// Generate one declared service trait method.
fn expand_trait_method(method: &Method) -> Tokens {
    expand_method(method, quote!(;))
}

/// Generate one service method with the given implementation.
fn expand_method(method: &Method, implementation: Tokens) -> Tokens {
    let attributes = &method.attributes;
    let name = &method.rust_name;
    let request = &method.request;
    let response = &method.response;
    let request_stream = method
        .request_stream
        .as_ref()
        .map(|stream| quote!(, requests: ::tspp_rpc::RequestStream<#stream>));
    let response_stream = method
        .response_stream
        .as_ref()
        .map(|stream| quote!(, responses: ::tspp_rpc::ResponseSender<#stream>));

    quote! {
        #(#attributes)*
        fn #name(
            &self,
            request: ::tspp_rpc::Request<#request>
            #request_stream
            #response_stream
        ) -> impl ::std::future::Future<
            Output = Result<::tspp_rpc::Response<#response>, ::tspp_rpc::Status>
        > + Send
        #implementation
    }
}

/// Generate one method schema registration.
fn expand_schema_method(service: &syn::LitStr, method: &Method) -> Tokens {
    let rust_name = &method.rust_name;
    let name = &method.name;
    let request = &method.request;
    let response = &method.response;
    let idempotency = expand_idempotency(method.idempotency);
    let method_constructor = expand_method_constructor(method);
    let request_schema = quote!(types.register::<#request>());
    let response_schema = quote!(types.register::<#response>());

    let constructor = match (&method.request_stream, &method.response_stream) {
        (None, None) => quote! {
            ::tspp_rpc::MethodSchema::unary(
                #service,
                #name,
                #request_schema,
                #response_schema,
                &types,
            )?
        },
        (None, Some(output)) => quote! {
            ::tspp_rpc::MethodSchema::server_streaming(
                #service,
                #name,
                #request_schema,
                #response_schema,
                types.register::<#output>(),
                &types,
            )?
        },
        (Some(input), None) => quote! {
            ::tspp_rpc::MethodSchema::client_streaming(
                #service,
                #name,
                #request_schema,
                #response_schema,
                types.register::<#input>(),
                &types,
            )?
        },
        (Some(input), Some(output)) => quote! {
            ::tspp_rpc::MethodSchema::bidirectional_streaming(
                #service,
                #name,
                #request_schema,
                #response_schema,
                types.register::<#input>(),
                types.register::<#output>(),
                &types,
            )?
        },
    };

    quote! {
        let #rust_name = {
            let schema = #constructor.with_idempotency(#idempotency, &types)?;
            let method = #method_constructor;
            methods.push(schema);

            method
        };
    }
}

/// Generate one cached typed method descriptor field.
fn expand_method_field(method: &Method) -> Tokens {
    let name = &method.rust_name;
    let request = &method.request;
    let response = &method.response;
    let input = method.request_stream.as_ref().map_or_else(
        || quote!(::std::convert::Infallible),
        |stream| quote!(#stream),
    );
    let output = method.response_stream.as_ref().map_or_else(
        || quote!(::std::convert::Infallible),
        |stream| quote!(#stream),
    );

    quote! {
        /// Exact typed descriptor for this RPC method.
        #name: ::tspp_rpc::Method<#request, #response, #input, #output>,
    }
}

/// Generate one typed client method.
fn expand_client_method(method: &Method) -> Tokens {
    let attributes = &method.attributes;
    let rust_name = &method.rust_name;
    let request = &method.request;
    let response = &method.response;
    let input = method.request_stream.as_ref().map_or_else(
        || quote!(::std::convert::Infallible),
        |stream| quote!(#stream),
    );
    let output = method.response_stream.as_ref().map_or_else(
        || quote!(::std::convert::Infallible),
        |stream| quote!(#stream),
    );
    let return_type = if method.request_stream.is_none() && method.response_stream.is_none() {
        quote!(::tspp_rpc::Response<#response>)
    } else {
        quote!(::tspp_rpc::Call<#response, #input, #output>)
    };
    let invoke = if method.request_stream.is_none() && method.response_stream.is_none() {
        quote!(self.connection.call(method, request))
    } else {
        quote!(self.connection.start(method, request))
    };

    quote! {
        #(#attributes)*
        pub fn #rust_name(
            &self,
            request: impl ::tspp_rpc::IntoRequest<#request>,
        ) -> Result<#return_type, ::tspp_rpc::CallError> {
            let method = self.methods.#rust_name;

            #invoke
        }
    }
}

/// Generate one server dispatch branch.
fn expand_server_method(method: &Method) -> Tokens {
    let rust_name = &method.rust_name;
    let request = &method.request;
    let request_stream = method.request_stream.as_ref().map(|stream| {
        quote! {
            let requests = call.request_stream::<#stream>()?;
        }
    });
    let response_stream = method.response_stream.as_ref().map(|stream| {
        quote! {
            let responses = call.response_sender::<#stream>()?;
        }
    });
    let request_argument = method.request_stream.as_ref().map(|_| quote!(, requests));
    let response_argument = method.response_stream.as_ref().map(|_| quote!(, responses));

    quote! {
        method if method == self.methods.#rust_name.id() => {
            let request = call.decode_request::<#request>()?;
            #request_stream
            #response_stream
            let response = self.service.#rust_name(
                request
                #request_argument
                #response_argument
            ).await;

            match response {
                Ok(response) => call.respond(response),
                Err(status) => call.fail(status),
            }
        }
    }
}

/// Generate one statically typed method constructor.
fn expand_method_constructor(method: &Method) -> Tokens {
    match (&method.request_stream, &method.response_stream) {
        (None, None) => quote! {
            ::tspp_rpc::Method::unary(service, schema.id(), schema.fingerprint())
        },
        (None, Some(_)) => quote! {
            ::tspp_rpc::Method::server_streaming(service, schema.id(), schema.fingerprint())
        },
        (Some(_), None) => quote! {
            ::tspp_rpc::Method::client_streaming(service, schema.id(), schema.fingerprint())
        },
        (Some(_), Some(_)) => quote! {
            ::tspp_rpc::Method::bidirectional_streaming(
                service,
                schema.id(),
                schema.fingerprint(),
            )
        },
    }
}

/// Generate one method idempotency value.
fn expand_idempotency(idempotency: Idempotency) -> Tokens {
    match idempotency {
        Idempotency::Unknown => quote!(::tspp_rpc::Idempotency::Unknown),
        Idempotency::Idempotent => quote!(::tspp_rpc::Idempotency::Idempotent),
        Idempotency::NoSideEffects => quote!(::tspp_rpc::Idempotency::NoSideEffects),
    }
}
