# VSCode Bridge Tests

This suite validates VSCode adapter scaffolding only.
Canonical applied-LSP correctness belongs in `language/test`, not in the VSCode bridge.

## Scope

This suite checks activation, command registration, test API exports, and one definition-provider roundtrip.
It does not own semantic capability matrices, fixture expansion, or multi-step LSP behavior coverage.

## Testing

Run bridge smoke tests.

```sh
bun run test:bridge
```
