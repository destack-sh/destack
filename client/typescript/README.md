# @destack/language

Destack language client for TypeScript and JavaScript.
This package opens language workspaces across Node-API and WebAssembly environments.

## Installation

```sh
npm install @destack/language
```

## API

```ts
import { openWorkspace } from "@destack/language";

const workspace = await openWorkspace({
    workspace: "/workspace",
});

console.log(await workspace.revision());
```

## Explicit Backends

```ts
import { openNapiWorkspace } from "@destack/language/napi";
import { openWasmWorkspace } from "@destack/language/wasm";

const nativeWorkspace = await openNapiWorkspace({ workspace: "." });
const webWorkspace = await openWasmWorkspace({ workspace: "/workspace" });
```

## Testing

Run these from the repository root.

```sh
just client/test
just client/check-quick
just client/check-full
```
