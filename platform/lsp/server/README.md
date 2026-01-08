# lsp/server

LSP server framework for the Destack language server.
Handles JSON-RPC transport, request routing, and client communication.

## Overview

This crate provides the infrastructure for implementing an LSP server.
Originally derived from `tower-lsp-server`, now maintained in-tree.

## Key Components

- `Server` — main server loop handling stdio/TCP transport
- `LanguageServer` trait — implement this for your language server
- `Client` — send notifications and requests to the editor
- `LspService` — tower service wrapper for the server

## Usage

```rust
use destack_lsp_server::{Client, LanguageServer, LspService, Server};
use destack_lsp_types::*;

struct Backend { client: Client }

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        // ...
    }
    // ... implement other methods
}

let (service, socket) = LspService::new(|client| Backend { client });
Server::new(stdin, stdout, socket).serve(service).await;
```
