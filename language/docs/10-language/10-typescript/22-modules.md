---
title: Modules
description: Strictly ESM imports and exports.
---

# Modules

- strictly ESM imports and exports
- imports, exports, re-exports, defaults, etc. it's all the same
- import data files
- no async imports / exports
- no CommonJS
- no export type / import type

- nested `module { ... }` block whose decorators configure the module itself

- JSON, TOML, and YAML imports become exact deeply readonly literal values at compile time
- Markdown, CSS, HTML, and plain text import as `string`; images, fonts, Wasm, and other binary files import as `uint8[]`
- `with { type: ... }` overrides the loader with `json`, `toml`, `yaml`, `text`, `binary`, or `base64`

```tspp src/geometry.tspp
export function square(value: float64): float64 {
    value * value
}
```

```tspp src/main.tspp
import { square } from "./geometry.tspp";

square(4.0);
```

- [Globals](23-globals.md)
