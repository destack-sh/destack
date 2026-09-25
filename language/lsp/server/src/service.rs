//! Service abstraction for language servers.

pub use self::client::{Client, ClientSocket, LogRecord, RequestStream, ResponseSink, progress};

pub use self::pending::Pending;
pub use self::state::{ServerState, State};

use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::future::{self, BoxFuture, FutureExt};
use tower::Service;
use tspp_lsp_types::{LSPAny, MessageType, TraceValue};

use crate::LanguageServer;
use crate::jsonrpc::{
    Error, ErrorCode, FromParams, IntoResponse, Method, Request, Response, Router,
};

pub mod layers;

mod client;
mod pending;
mod state;

/// Error that occurs when attempting to call the language server after it has already exited.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExitedError(());

impl std::error::Error for ExitedError {}

impl Display for ExitedError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("language server has exited")
    }
}

/// Service abstraction for the Language Server Protocol.
///
/// This service takes an incoming JSON-RPC message as input and produces an outgoing message as
/// output. If the incoming message is a server notification or a client response, then the
/// corresponding response will be `None`.
///
/// This implements [`tower::Service`] in order to remain independent from the underlying transport
/// and to facilitate further abstraction with middleware.
///
/// Pending requests can be canceled by issuing a [`$/cancelRequest`] notification.
///
/// [`$/cancelRequest`]: https://microsoft.github.io/language-server-protocol/specification#cancelRequest
///
/// The service shuts down and stops serving requests after the [`exit`] notification is received.
///
/// [`exit`]: https://microsoft.github.io/language-server-protocol/specification#exit
#[derive(Debug)]
pub struct LspService<S> {
    inner: Router<S, ExitedError>,
    state: Arc<ServerState>,
    client: Client,
}

/// One structured protocol trace record.
pub(super) struct TraceRecord {
    /// The compact record fields.
    message: String,
    /// The verbose message body.
    verbose: Option<String>,
}

impl TraceRecord {
    /// Render an incoming request or notification.
    fn received(request: &Request, is_verbose: bool) -> Self {
        Self::request("lsp", request, is_verbose)
    }

    /// Render an outgoing request or notification.
    pub(super) fn sent(request: &Request, is_verbose: bool) -> Self {
        Self::request("lsp.client", request, is_verbose)
    }

    /// Render an outgoing response.
    fn response_sent(
        response: &Response,
        method: &str,
        duration: Duration,
        is_verbose: bool,
    ) -> Self {
        Self::response("lsp.response", response, method, duration, is_verbose)
    }

    /// Render an incoming response.
    pub(super) fn response_received(
        response: &Response,
        method: &str,
        duration: Duration,
        is_verbose: bool,
    ) -> Self {
        Self::response(
            "lsp.client.response",
            response,
            method,
            duration,
            is_verbose,
        )
    }

    /// Render one completed incoming notification.
    fn notification_finished(method: &str, duration: Duration) -> Self {
        let message = LogRecord::new("lsp.notification.finished")
            .field("method", method)
            .field("duration_us", duration.as_micros())
            .to_string();

        Self {
            message,
            verbose: None,
        }
    }

    /// Return the compact record fields.
    pub(super) fn message(&self) -> &str {
        &self.message
    }

    /// Split the record into standard LSP trace fields.
    pub(super) fn into_parts(self) -> (String, Option<String>) {
        (self.message, self.verbose)
    }

    /// Render one request-shaped message.
    fn request(namespace: &str, request: &Request, is_verbose: bool) -> Self {
        let event = match request.id() {
            Some(_) => format!("{namespace}.request"),
            None => format!("{namespace}.notification"),
        };
        let mut record = LogRecord::new(&event).field("method", request.method());
        if let Some(request_id) = request.id() {
            record = record.field("request_id", request_id);
        }

        // retain complete parameters for verbose protocol traces
        let verbose = if is_verbose {
            request.params().map(|params| format!("params={params}"))
        } else {
            None
        };

        Self {
            message: record.to_string(),
            verbose,
        }
    }

    /// Render one response-shaped message.
    fn response(
        event: &str,
        response: &Response,
        method: &str,
        duration: Duration,
        is_verbose: bool,
    ) -> Self {
        let duration_us = duration.as_micros();
        let status = match response.error().map(|error| error.code) {
            None => "ok",
            Some(ErrorCode::RequestCancelled) => "cancelled",
            Some(ErrorCode::ContentModified) => "stale",
            Some(_) => "error",
        };
        let mut record = LogRecord::new(event)
            .field("method", method)
            .field("request_id", response.id())
            .field("status", status)
            .field("duration_us", duration_us);

        // include exact failures in the compact record
        if let Some(error) = response.error() {
            let reason = error.reason();
            record = record.field("code", error.code).field("error", reason);
        }

        // include complete results only in verbose traces
        let verbose = if is_verbose {
            response
                .result()
                .map(|result| format!("result={result}"))
                .or_else(|| {
                    response
                        .error()
                        .and_then(|error| error.data.as_ref())
                        .map(|data| format!("error_data={data}"))
                })
        } else {
            None
        };

        Self {
            message: record.to_string(),
            verbose,
        }
    }
}

impl<S: LanguageServer> LspService<S> {
    /// Creates a new `LspService` with the given server backend, also returning a channel for
    /// server-to-client communication.
    pub fn new<F>(init: F) -> (Self, ClientSocket)
    where
        F: FnOnce(Client) -> S,
    {
        Self::build(init).finish()
    }

    /// Starts building a new `LspService`.
    ///
    /// Returns an `LspServiceBuilder`, which allows adding custom JSON-RPC methods to the server.
    pub fn build<F>(init: F) -> LspServiceBuilder<S>
    where
        F: FnOnce(Client) -> S,
    {
        let state = Arc::new(ServerState::new());

        let (client, socket) = Client::new(state.clone());
        let inner = Router::new(init(client.clone()));
        let pending = Arc::new(Pending::new());

        LspServiceBuilder {
            inner: crate::server::generated::register_lsp_methods(
                inner,
                state.clone(),
                pending.clone(),
                client.clone(),
            ),
            state,
            pending,
            client,
            socket,
        }
    }

    /// Returns a reference to the inner server.
    #[must_use]
    pub fn inner(&self) -> &S {
        self.inner.inner()
    }
}

impl<S: LanguageServer> Service<Request> for LspService<S> {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.state.get() {
            State::Initializing => Poll::Pending,
            State::Exited => Poll::Ready(Err(ExitedError(()))),
            _ => self.inner.poll_ready(cx),
        }
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if self.state.get() == State::Exited {
            return future::err(ExitedError(())).boxed();
        }

        let trace = self.client.trace_level();
        let is_verbose = trace == TraceValue::Verbose;
        let method = req.method().to_string();
        let request_trace =
            (trace != TraceValue::Off).then(|| TraceRecord::received(&req, is_verbose));
        let client = self.client.clone();
        let fut = self.inner.call(req);

        Box::pin(async move {
            if let Some(request_trace) = request_trace {
                client.log_trace_record(request_trace);
            }

            // time request handling independently from trace publication
            let started = Instant::now();
            let response = fut.await?;

            let response = match response.as_ref().and_then(|res| res.error()) {
                Some(Error {
                    code: ErrorCode::MethodNotFound,
                    data: Some(LSPAny::String(m)),
                    ..
                }) if m.starts_with("$/") => None,
                _ => response,
            };

            // report failures and traced responses with the exact handler duration
            if let Some(response) = response.as_ref() {
                let is_error = response.error().is_some_and(|error| {
                    !matches!(
                        error.code,
                        ErrorCode::RequestCancelled | ErrorCode::ContentModified
                    )
                });
                if is_error || trace != TraceValue::Off {
                    let response_trace = TraceRecord::response_sent(
                        response,
                        &method,
                        started.elapsed(),
                        is_verbose,
                    );
                    if is_error {
                        client
                            .log_message(MessageType::ERROR, response_trace.message())
                            .await;
                    } else {
                        client.log_trace_record(response_trace);
                    }
                }
            }
            // report traced notification completion
            else if trace != TraceValue::Off {
                let notification_trace =
                    TraceRecord::notification_finished(&method, started.elapsed());
                client.log_trace_record(notification_trace);
            }

            Ok(response)
        })
    }
}

/// A builder to customize the properties of an `LspService`.
///
/// To construct an `LspServiceBuilder`, refer to [`LspService::build`].
pub struct LspServiceBuilder<S> {
    inner: Router<S, ExitedError>,
    state: Arc<ServerState>,
    pending: Arc<Pending>,
    client: Client,
    socket: ClientSocket,
}

impl<S: LanguageServer> LspServiceBuilder<S> {
    /// Defines a custom JSON-RPC request or notification with the given method `name` and handler.
    ///
    /// # Handler varieties
    ///
    /// Fundamentally, any inherent `async fn(&self)` method defined directly on the language
    /// server backend could be considered a valid method handler.
    ///
    /// Handlers may optionally include a single `params` argument. This argument may be of any
    /// type that implements [`Serialize`](serde::Serialize).
    ///
    /// Handlers which return `()` are treated as **notifications**, while those which return
    /// [`jsonrpc::Result<T>`](crate::jsonrpc::Result) are treated as **requests**.
    ///
    /// Similar to the `params` argument, the `T` in the `Result<T>` return values may be of any
    /// type which implements [`DeserializeOwned`](serde::de::DeserializeOwned). Additionally, this
    /// type _must_ be convertible into a [`serde_json::Value`] using [`serde_json::to_value`]. If
    /// this latter constraint is not met, the client will receive a JSON-RPC error response with
    /// code `-32603` (Internal Error) instead of the expected response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use serde_json::{json, Value};
    /// use tower_lsp_server::jsonrpc::Result;
    /// use tower_lsp_server::tspp_lsp_types::*;
    /// use tower_lsp_server::{LanguageServer, LspService};
    ///
    /// struct Mock;
    ///
    /// // Implementation of `LanguageServer` omitted...
    /// # impl LanguageServer for Mock {
    /// #     async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    /// #         Ok(InitializeResult::default())
    /// #     }
    /// #
    /// #     async fn shutdown(&self) -> Result<()> {
    /// #         Ok(())
    /// #     }
    /// # }
    ///
    /// impl Mock {
    ///     async fn request(&self) -> Result<i32> {
    ///         Ok(123)
    ///     }
    ///
    ///     async fn request_params(&self, params: Vec<String>) -> Result<Value> {
    ///         Ok(json!({"num_elems":params.len()}))
    ///     }
    ///
    ///     async fn notification(&self) {
    ///         // ...
    ///     }
    ///
    ///     async fn notification_params(&self, params: Value) {
    ///         // ...
    /// #       let _ = params;
    ///     }
    /// }
    ///
    /// let (service, socket) = LspService::build(|_| Mock)
    ///     .custom_method("custom/request", Mock::request)
    ///     .custom_method("custom/requestParams", Mock::request_params)
    ///     .custom_method("custom/notification", Mock::notification)
    ///     .custom_method("custom/notificationParams", Mock::notification_params)
    ///     .finish();
    /// ```
    #[must_use]
    pub fn custom_method<P, R, F>(mut self, name: &'static str, callback: F) -> Self
    where
        P: FromParams,
        R: IntoResponse,
        F: for<'a> Method<&'a S, P, R> + Clone + Send + Sync + 'static,
    {
        let layer = layers::Normal::new(self.state.clone(), self.pending.clone());
        self.inner.method(name, callback, layer);
        self
    }

    /// Constructs the `LspService` and returns it, along with a channel for server-to-client
    /// communication.
    #[must_use]
    pub fn finish(self) -> (LspService<S>, ClientSocket) {
        let Self {
            inner,
            state,
            client,
            socket,
            ..
        } = self;

        (
            LspService {
                inner,
                state,
                client,
            },
            socket,
        )
    }
}

impl<S: Debug> Debug for LspServiceBuilder<S> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("LspServiceBuilder")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tower::ServiceExt;
    use tspp_lsp_types::*;

    use super::*;
    use crate::jsonrpc::Result;

    #[derive(Debug)]
    struct Mock;

    impl LanguageServer for Mock {
        async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
            Ok(InitializeResult::default())
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        // this handler should never resolve
        async fn code_action_resolve(&self, _: CodeAction) -> Result<CodeAction> {
            future::pending().await
        }
    }

    #[expect(clippy::unused_async)]
    impl Mock {
        async fn custom_request(&self, params: i32) -> Result<i32> {
            Ok(params)
        }
    }

    fn initialize_request(id: i64) -> Request {
        Request::build("initialize")
            .params(json!({"capabilities":{}}))
            .id(id)
            .finish()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn initializes_only_once() {
        let (mut service, _) = LspService::new(|_| Mock);

        let request = initialize_request(1);

        let response = service.ready().await.unwrap().call(request.clone()).await;
        let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
        assert_eq!(response, Ok(Some(ok)));

        let response = service.ready().await.unwrap().call(request).await;
        let err = Response::from_error(1.into(), Error::invalid_request());
        assert_eq!(response, Ok(Some(err)));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn refuses_requests_after_shutdown() {
        let (mut service, _) = LspService::new(|_| Mock);

        let initialize = initialize_request(1);
        let response = service.ready().await.unwrap().call(initialize).await;
        let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
        assert_eq!(response, Ok(Some(ok)));

        let shutdown = Request::build("shutdown").id(1).finish();
        let response = service.ready().await.unwrap().call(shutdown.clone()).await;
        let ok = Response::from_ok(1.into(), json!(null));
        assert_eq!(response, Ok(Some(ok)));

        let response = service.ready().await.unwrap().call(shutdown).await;
        let err = Response::from_error(1.into(), Error::invalid_request());
        assert_eq!(response, Ok(Some(err)));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exit_notification() {
        let (mut service, _) = LspService::new(|_| Mock);

        let exit = Request::build("exit").finish();
        let response = service.ready().await.unwrap().call(exit.clone()).await;
        assert_eq!(response, Ok(None));

        let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
        assert_eq!(ready, Err(ExitedError(())));
        assert_eq!(service.call(exit).await, Err(ExitedError(())));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancels_pending_requests() {
        let (mut service, _) = LspService::new(|_| Mock);

        let initialize = initialize_request(1);
        let response = service.ready().await.unwrap().call(initialize).await;
        let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
        assert_eq!(response, Ok(Some(ok)));

        let pending_request = Request::build("codeAction/resolve")
            .params(json!({"title":""}))
            .id(1)
            .finish();

        let cancel_request = Request::build("$/cancelRequest")
            .params(json!({"id":1i32}))
            .finish();

        let pending_fut = service.ready().await.unwrap().call(pending_request);
        let cancel_fut = service.ready().await.unwrap().call(cancel_request);
        let (pending_response, cancel_response) = futures::join!(pending_fut, cancel_fut);

        let canceled = Response::from_error(1.into(), Error::request_cancelled());
        assert_eq!(pending_response, Ok(Some(canceled)));
        assert_eq!(cancel_response, Ok(None));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn serves_custom_requests() {
        let (mut service, _) = LspService::build(|_| Mock)
            .custom_method("custom", Mock::custom_request)
            .finish();

        let initialize = initialize_request(1);
        let response = service.ready().await.unwrap().call(initialize).await;
        let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
        assert_eq!(response, Ok(Some(ok)));

        let custom = Request::build("custom").params(123i32).id(1).finish();
        let response = service.ready().await.unwrap().call(custom).await;
        let ok = Response::from_ok(1.into(), json!(123i32));
        assert_eq!(response, Ok(Some(ok)));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn get_inner() {
        let (service, _) = LspService::build(|_| Mock).finish();

        service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .unwrap();
    }
}
