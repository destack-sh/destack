# lsp-types

LSP type definitions for the Destack language server.
Provides Rust types for the Language Server Protocol specification.

## Overview

This crate contains serializable Rust structs and enums for LSP messages, requests, and notifications.
Originally derived from the `lsp-types` crate, now maintained in-tree for Destack-specific customization.

## Key Types

- `Position`, `Range`, `Location` — text document positions
- `Diagnostic`, `CodeAction` — editor feedback
- `CompletionItem`, `Hover`, `SignatureHelp` — intellisense features
- `SemanticToken` — syntax highlighting
- `Uri` — document identifiers

All types implement `Serialize` and `Deserialize` for JSON-RPC transport.
