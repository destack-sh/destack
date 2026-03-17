# resolver

Module resolution for Destack, TypeScript, and JavaScript.Resolves import specifiers (like `"./utils"` or `"lodash"`) to actual file paths.
The Destack resolver was originally based on [oxc-resolver](https://github.com/oxc-project/oxc-resolver), which in turn is the Rust port of webpack's [enhanced-resolve](https://github.com/webpack/enhanced-resolve), which is also where we get most of the resolver test fixtures from.

## Rewrite

The "annoying" part of module resolution is that specifiers mean different things depending on the context, i.e., they are "rewritten" before resolution according to some specification (`tsconfig.json`, `package.json`, etc.).

| Source | Example | Effect |
| --- | --- | --- |
| `tsconfig.json` `paths` and `baseUrl` | `@/util` | Rewrite app-local specifiers before package lookup. |
| aliases and fallback aliases | `~shared/button` | Rewrite directly from resolver options. |
| `package.json` `imports` | `#config` | Rewrite private package-internal specifiers. |
| package self references | `my-app/server` | Rewrite a package importing itself by name. |
| `package.json` `exports` | `pkg/feature` | Map published subpaths to concrete targets. |
| browser field rewrites | `./platform` | Swap package-internal targets for browser builds. |
| extension aliases | `./widget.js` | Probe alternate file extensions such as `.js -> .ts`. |

`dsconfig.json` is separate from ordinary module resolution.
It is package or workspace metadata that some callers want alongside resolution, not part of the core path-selection algorithm.

## Examples

### `tsconfig.json` paths and `baseUrl`

This is the common app-local rewrite.

```ts
// src/main.ts
import { formatUser } from "@/format/user";
```

```json
{
  "compiler": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}
```

The resolver rewrites `@/format/user` to `src/format/user`, then probes extensions and lands on `src/format/user.ts`.

### `package.json` imports inside one package

This is the package-internal form.

```ds
// src/runtime/app.ds
import { settings } from "#config";
```

```json
{
  "imports": {
    "#config": "./src/config/runtime.ds"
  }
}
```

The resolver rewrites `#config` through `package.json#imports`, then resolves `./src/config/runtime.ds`.

### `package.json` exports from another package

This is the published package form.

```ts
// src/main.ts
import { createClient } from "acme/client";
```

```json
{
  "name": "acme",
  "exports": {
    "./client": "./src/client.ts"
  }
}
```

The resolver finds the `acme` package, applies `exports["./client"]`, then resolves `acme/src/client.ts`.

### browser field rewrites

This is the package-internal browser swap.

```ts
// src/entry.ts
import { platformName } from "./platform";
```

```json
{
  "browser": {
    "./src/platform.ts": "./src/platform.browser.ts"
  }
}
```

If browser rewrites are enabled and `./platform` resolves to `./src/platform.ts`, the resolver swaps that target to `./src/platform.browser.ts`.

## Testing

Run these from the repository root:

```sh
cargo test -p destack_resolver
just language/test-resolver
just language/test-query
just language/quick
just language/full
```
