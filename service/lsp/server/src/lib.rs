#![allow(missing_debug_implementations)]
#![allow(dead_code)]
#![allow(elided_lifetimes_in_paths)]
#![allow(unreachable_pub)]

pub use destack_lsp_types;

pub use self::server::LanguageServer;
pub use self::service::progress::{
    Bounded, Cancellable, NotCancellable, OngoingProgress, Progress, Unbounded,
};
pub use self::service::{Client, ClientSocket, ExitedError, LspService, LspServiceBuilder};
pub use self::transport::{Loopback, Server};
pub use self::uri_ext::UriExt;

pub mod jsonrpc;

mod codec;
mod server;
mod service;
mod transport;
mod uri_ext;
