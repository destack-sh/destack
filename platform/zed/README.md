# zed

Zed extension for Destack language support.
This extension provides `.ds` and `.d.ds` language configuration, Tree-sitter highlighting, and LSP wiring.
The language server is registered for Destack plus Zed JavaScript, TypeScript, and TSX buffers.

## Grammar Source

The grammar source is the canonical fork at `language/grammar/destack`.
`extension.toml` points at the Destack repository path `language/grammar/destack/destack`.
The current grammar ref is `main` and should be pinned to a commit for release branches.

## Development

Build and run extension tests with Cargo.

```sh
cd platform/zed
cargo test --release
```

Install it in Zed with `Install Dev Extension` and select `platform/zed`.
Start Zed from a terminal with `zed --foreground` to inspect extension logs.

## Local Binary Path

When developing across multiple worktrees, set a shared Destack binary path in Zed settings.
This avoids requiring `target/debug/destack` or `target/release/destack` in every worktree.

```json
{
  "lsp": {
    "destack-lsp": {
      "binary": {
        "path": "/absolute/path/to/destack",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## LSP Coverage

The extension registers `destack-lsp` for these Zed languages:

1. `Destack` (`destack` language id)
2. `JavaScript` (`javascript` language id)
3. `TypeScript` (`typescript` language id)
4. `TSX` (`typescriptreact` language id)
