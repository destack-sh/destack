# resolver

Module resolution for Destack, TypeScript, and JavaScript.
Resolves import specifiers (like `"./utils"` or `"lodash"`) to actual file paths.

## Background

Node.js module resolution is surprisingly complex with `package.json` exports, tsconfig paths, browser field substitutions, symlinks, etc.
To get started quickly and get a good baseline (and tests!), we started from a port of [oxc-resolver](https://github.com/oxc-project/oxc-resolver) (itself a port of webpack's [enhanced-resolve](https://github.com/webpack/enhanced-resolve)).

Unfortunately, `oxc-resolver` didn't fit well with our model and design philosophy, so it was significantly refactored and rewritten. 
Nonetheless, it was a great starting point, and we adapted it to integrate with our `Program` model and added Destack-specific resolution (`dsconfig.json`).

## What It Resolves

Fundamentally, the job of the resolver is to figure out where a module is located based on some contextual specifier (like `import { foo } from "./utils"` or `import { bar } from "@scope/pkg"`).

```
import { foo } from "./utils"        // relative path
import { bar } from "@scope/pkg"     // node_modules package
import { baz } from "#internal"      // package.json imports field
```

The resolver handles:
- **Relative/absolute paths**: `./foo`, `../bar`, `/absolute`
- **Node modules**: walks up `node_modules` directories
- **Package exports**: the `exports` field in `package.json`
- **Package imports**: the `imports` field (`#` prefix)
- **TypeScript paths**: `tsconfig.json` `paths` and `baseUrl`
- **Destack config**: `dsconfig.json` for Destack-specific settings
- **Browser field**: substitutions for browser builds
- **Aliases**: custom module aliases