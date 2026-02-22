# Modules

> NOTE: mdtest module fixtures model ES module semantics unless a fixture explicitly targets CommonJS interop

ES module imports and exports.

## Coverage

- **Imports**: Named, default, namespace, side-effect
- **Exports**: Named, default, re-exports, inference, circularity
- **Type-only**: `import type`, `export type`
- **Namespace values**: `import * as` shapes and `export *` merging
- **Resolution**: Module path resolution
- **CommonJS interop**: Static `module.exports` and `exports` forms
- **import.meta**: Module metadata and environment values

Destack uses the same module system as TypeScript/JavaScript.
