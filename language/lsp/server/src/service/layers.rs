//! Assorted middleware that implements LSP server semantics.

use std::marker::PhantomData;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::{self, BoxFuture, FutureExt};
use tower::{Layer, Service};

use super::ExitedError;
use crate::jsonrpc::{Error, Id, Request, Response, not_initialized_error};

use super::client::Client;
use super::pending::Pending;
use super::state::{ServerState, State};

/// Middleware which implements `initialize` request semantics.
///
/// # Specification
///
/// <https://microsoft.github.io/language-server-protocol/specification#initialize>
pub struct Initialize {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
}

impl Initialize {
    pub const fn new(state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Self { state, pending }
    }
}

impl<S> Layer<S> for Initialize {
    type Service = InitializeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        InitializeService {
            inner: Cancellable::new(inner, self.pending.clone()),
            state: self.state.clone(),
        }
    }
}

/// Service created from [`Initialize`] layer.
pub struct InitializeService<S> {
    inner: Cancellable<S>,
    state: Arc<ServerState>,
}

impl<S> Service<Request> for InitializeService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if self.state.get() == State::Uninitialized {
            let state = self.state.clone();
            let fut = self.inner.call(req);

            Box::pin(async move {
                let response = fut.await?;

                match &response {
                    Some(response) if response.is_ok() => state.set(State::Initialized),
                    _ => state.set(State::Uninitialized),
                }

                Ok(response)
            })
        } else {
            let (_, id, _) = req.into_parts();
            future::ok(id.map(|id| Response::from_error(id, Error::invalid_request()))).boxed()
        }
    }
}

/// Middleware which implements `shutdown` request semantics.
///
/// # Specification
///
/// <https://microsoft.github.io/language-server-protocol/specification#shutdown>
pub struct Shutdown {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
}

impl Shutdown {
    pub const fn new(state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Self { state, pending }
    }
}

impl<S> Layer<S> for Shutdown {
    type Service = ShutdownService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ShutdownService {
            inner: Cancellable::new(inner, self.pending.clone()),
            state: self.state.clone(),
        }
    }
}

/// Service created from [`Shutdown`] layer.
pub struct ShutdownService<S> {
    inner: Cancellable<S>,
    state: Arc<ServerState>,
}

impl<S> Service<Request> for ShutdownService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Into<BoxFuture<'static, Result<Option<Response>, S::Error>>> + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match self.state.get() {
            State::Initialized => {
                self.state.set(State::ShutDown);
                self.inner.call(req)
            }
            cur_state => {
                let (_, id, _) = req.into_parts();
                future::ok(not_initialized_response(id, cur_state)).boxed()
            }
        }
    }
}

/// Middleware which implements `exit` notification semantics.
///
/// # Specification
///
/// <https://microsoft.github.io/language-server-protocol/specification#exit>
pub struct Exit {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
    client: Client,
}

impl Exit {
    pub const fn new(state: Arc<ServerState>, pending: Arc<Pending>, client: Client) -> Self {
        Self {
            state,
            pending,
            client,
        }
    }
}

impl<S> Layer<S> for Exit {
    type Service = ExitService<S>;

    fn layer(&self, _: S) -> Self::Service {
        ExitService {
            state: self.state.clone(),
            pending: self.pending.clone(),
            client: self.client.clone(),
            _marker: PhantomData,
        }
    }
}

/// Service created from [`Exit`] layer.
pub struct ExitService<S> {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
    client: Client,
    _marker: PhantomData<S>,
}

impl<S> Service<Request> for ExitService<S> {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.state.get() == State::Exited {
            Poll::Ready(Err(ExitedError(())))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&mut self, _: Request) -> Self::Future {
        self.state.set(State::Exited);
        self.pending.cancel_all();
        self.client.close();
        future::ok(None)
    }
}

/// Middleware which implements LSP semantics for all other kinds of requests.
pub struct Normal {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
}

impl Normal {
    pub const fn new(state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Self { state, pending }
    }
}

impl<S> Layer<S> for Normal {
    type Service = NormalService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NormalService {
            inner: Cancellable::new(inner, self.pending.clone()),
            state: self.state.clone(),
        }
    }
}

/// Service created from [`Normal`] layer.
pub struct NormalService<S> {
    inner: Cancellable<S>,
    state: Arc<ServerState>,
}

impl<S> Service<Request> for NormalService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Into<BoxFuture<'static, Result<Option<Response>, S::Error>>> + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match self.state.get() {
            State::Initialized => self.inner.call(req),
            cur_state => {
                let (_, id, _) = req.into_parts();
                future::ok(not_initialized_response(id, cur_state)).boxed()
            }
        }
    }
}

/// Wraps an inner service `S` and implements `$/cancelRequest` semantics for all requests.
///
/// # Specification
///
/// <https://microsoft.github.io/language-server-protocol/specification#cancelRequest>
struct Cancellable<S> {
    inner: S,
    pending: Arc<Pending>,
}

impl<S> Cancellable<S> {
    const fn new(inner: S, pending: Arc<Pending>) -> Self {
        Self { inner, pending }
    }
}

impl<S> Service<Request> for Cancellable<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req.id().cloned() {
            Some(id) => self.pending.execute(id, self.inner.call(req)).boxed(),
            None => self.inner.call(req).boxed(),
        }
    }
}

fn not_initialized_response(id: Option<Id>, server_state: State) -> Option<Response> {
    let id = id?;
    let error = match server_state {
        State::Uninitialized | State::Initializing => not_initialized_error(),
        _ => Error::invalid_request(),
    };

    Some(Response::from_error(id, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsonrpc::{Id, Request, Response};
    use serde_json::json;

    /// Mock service that returns a successful response.
    struct MockOkService;

    impl Service<Request> for MockOkService {
        type Response = Option<Response>;
        type Error = ExitedError;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request) -> Self::Future {
            let (_, id, _) = req.into_parts();
            let response = id.map(|id| Response::from_ok(id, json!({"result": "ok"})));
            future::ok(response).boxed()
        }
    }

    fn make_request(method: &str, id: Option<i64>) -> Request {
        let mut builder = Request::build(method.to_string());
        if let Some(id) = id {
            builder = builder.id(Id::Number(id));
        }
        builder.finish()
    }

    #[tokio::test]
    async fn test_initialize_first_request_succeeds() {
        let state = Arc::new(ServerState::new());
        let pending = Arc::new(Pending::new());
        let layer = Initialize::new(state.clone(), pending);
        let mut service = layer.layer(MockOkService);

        assert_eq!(state.get(), State::Uninitialized);

        let request = make_request("initialize", Some(1));
        let response = service.call(request).await.unwrap();

        assert!(response.is_some());
        assert!(response.unwrap().is_ok());
        assert_eq!(state.get(), State::Initialized);
    }

    #[tokio::test]
    async fn test_initialize_duplicate_request_rejected() {
        let state = Arc::new(ServerState::new());
        state.set(State::Initialized);
        let pending = Arc::new(Pending::new());
        let layer = Initialize::new(state.clone(), pending);
        let mut service = layer.layer(MockOkService);

        let request = make_request("initialize", Some(2));
        let response = service.call(request).await.unwrap();

        assert!(response.is_some());
        assert!(!response.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_when_initialized() {
        let state = Arc::new(ServerState::new());
        state.set(State::Initialized);
        let pending = Arc::new(Pending::new());
        let layer = Shutdown::new(state.clone(), pending);
        let mut service = layer.layer(MockOkService);

        let request = make_request("shutdown", Some(1));
        let response = service.call(request).await.unwrap();

        assert!(response.is_some());
        assert_eq!(state.get(), State::ShutDown);
    }

    #[tokio::test]
    async fn test_shutdown_when_not_initialized() {
        let state = Arc::new(ServerState::new());
        let pending = Arc::new(Pending::new());
        let layer = Shutdown::new(state.clone(), pending);
        let mut service = layer.layer(MockOkService);

        let request = make_request("shutdown", Some(1));
        let response = service.call(request).await.unwrap();

        // returns not initialized error
        assert!(response.is_some());
        assert!(!response.unwrap().is_ok());
        assert_eq!(state.get(), State::Uninitialized);
    }

    #[tokio::test]
    async fn test_normal_when_initialized() {
        let state = Arc::new(ServerState::new());
        state.set(State::Initialized);
        let pending = Arc::new(Pending::new());
        let layer = Normal::new(state, pending);
        let mut service = layer.layer(MockOkService);

        let request = make_request("textDocument/hover", Some(1));
        let response = service.call(request).await.unwrap();

        assert!(response.is_some());
        assert!(response.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_normal_when_not_initialized() {
        let state = Arc::new(ServerState::new());
        let pending = Arc::new(Pending::new());
        let layer = Normal::new(state, pending);
        let mut service = layer.layer(MockOkService);

        let request = make_request("textDocument/hover", Some(1));
        let response = service.call(request).await.unwrap();

        // returns not initialized error
        assert!(response.is_some());
        assert!(!response.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_exit_sets_state() {
        let state = Arc::new(ServerState::new());
        state.set(State::ShutDown);
        let pending = Arc::new(Pending::new());
        let (client, _) = Client::new(state.clone());
        let layer = Exit::new(state.clone(), pending, client);
        let mut service = layer.layer(MockOkService);

        let request = make_request("exit", None);
        let response = service.call(request).await.unwrap();

        assert!(response.is_none());
        assert_eq!(state.get(), State::Exited);
    }
}
