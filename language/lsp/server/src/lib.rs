#![allow(missing_debug_implementations)]
#![allow(dead_code)]
#![allow(elided_lifetimes_in_paths)]
#![allow(unreachable_pub)]

pub use tspp_lsp_types;

pub use self::server::LanguageServer;
pub use self::service::progress::{
    Bounded, Cancellable, NotCancellable, OngoingProgress, Progress, Unbounded,
};
pub use self::service::{
    Client, ClientSocket, ExitedError, LogRecord, LspService, LspServiceBuilder, RequestStream,
    ResponseSink,
};
pub use self::transport::{Loopback, Server};
pub use self::uri_ext::UriExt;

pub mod jsonrpc;

mod codec;
mod server;
mod service;
mod transport;
mod uri_ext;
