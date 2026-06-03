# zed

Zed extension for Destack language support.
This extension provides `.ds`, `.d.ds`, and `.mir` language configuration and Tree-sitter highlighting.
It launches `destack lsp` for Destack, JavaScript, TypeScript, and TSX buffers.

## Grammar Source

The grammar sources are `language/grammar/destack/destack` for Destack and `language/grammar/mir` for Destack MIR.
`extension.toml` points at those repository paths.

## Development

Install it in Zed with `Install Dev Extension` and select `bridge/zed`.
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

## Release

The extension version in `extension.toml` must match the repository `VERSION.txt`.
`destack dev version` updates both files.

You can prepare a dry-run registry update from the repository root.

```sh
just bridge/publish-zed --dry-run
```

Live mode pushes a branch to your fork of `zed-industries/extensions` and opens or reuses a PR.

```sh
GH_TOKEN=<token> DESTACK_ZED_REGISTRY_PUSH_TO=<owner>/extensions just bridge/publish-zed ""
```

## LSP Coverage

The extension registers `destack-lsp` for these Zed languages:

1. `Destack` (`destack` language id)
2. `JavaScript` (`javascript` language id)
3. `TypeScript` (`typescript` language id)
4. `TSX` (`typescriptreact` language id)

## Testing

Run these from the repository root.

```sh
just bridge/check-quick
just bridge/check-full
```
