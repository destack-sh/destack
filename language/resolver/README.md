# resolver

Module resolution for Destack, TypeScript, and JavaScript.
Resolves import specifiers (like `"./utils"` or `"lodash"`) to actual file paths.

## Background

JavaScript module resolution is surprisingly complex with `package.json` exports, tsconfig paths, browser field substitutions, symlinks, and some more fun stuff.
Destack's module resolver began as a port of [oxc-resolver](https://github.com/oxc-project/oxc-resolver), which itself is a port of webpack's [enhanced-resolve](https://github.com/webpack/enhanced-resolve).
Ultimately, `oxc-resolver` did not fit well with our model and design philosophy, so it was significantly refactored and rewritten to integrate with our `Program` model and we added some Destack-specific features (like resolving `dsconfig.json`).

## What It Resolves

The basic task of the resolver is to figure out where a module is located based on some contextual specifier (like `import { foo } from "./utils"` or `import { bar } from "@scope/pkg"`).

```ts
import { foo } from "./utils";        // relative path
import { bar } from "@scope/pkg";     // node_modules package
import { baz } from "#internal";      // package.json imports field
```

The resolver handles:
- **Relative/absolute paths**: `./foo`, `../bar`, `/absolute`
- **Node modules**: walks up `node_modules` directories
- **Package exports**: the `exports` field in `package.json`
- **Package self references**: bare imports that match the current package name
- **Package imports**: the `imports` field (`#` prefix)
- **TypeScript paths**: `tsconfig.json` `paths` and `baseUrl`
- **Destack config**: `dsconfig.json` for Destack-specific settings
- **Browser field**: substitutions for browser builds
- **Aliases**: custom module aliases

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_resolver
just language/test-resolver
just language/test-query

# clean gate
just language/quick

# exhaustive gate
just language/full
```
