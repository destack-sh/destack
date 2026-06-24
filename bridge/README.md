# Bridges

Destack bridges into host tools and editor environments.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`vscode`](vscode/README.md) | Experimental | VS Code extension and language support |
| [`zed`](zed/README.md) | Experimental | Zed extension integration |

## Commands

Run these commands from the repository root.

```sh
just bridge/format
just bridge/format-check
just bridge/lint
just bridge/build
just bridge/test
just bridge/check-quick
just bridge/check-full
just bridge/install-toolchain
just bridge/doctor-toolchain
just bridge/ensure-toolchain
just bridge/publish --dry-run
just bridge/publish-zed --dry-run
```
