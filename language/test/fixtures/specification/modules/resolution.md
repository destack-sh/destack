# Module Resolution

Tests for extensionless and index-based module resolution.

## extensionless specifiers

### resolves ds module without extension

> Extensionless specifiers resolve `.ds` modules.

```ds:main.ds
import { value } from "./mod";

value satisfies int32;
```

```ds:mod.ds
export const value: int32 = 1;
```

### resolves ts module without extension

> Extensionless specifiers resolve `.ts` modules from TypeScript sources.

```ts:main.ts
import { value } from "./mod";

value satisfies number;
```

```ts:mod.ts
export const value = 1;
```

### resolves declaration modules without extension

> Extensionless specifiers resolve `.d.ts` modules for type usage.

```ts:main.ts
import type { User } from "./types";

type Alias = User;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ts:types.d.ts
export interface User {
    name: string;
}
```

### resolves declaration value exports

> Extensionless specifiers resolve declared values from `.d.ts` modules.

```ts:main.ts
import { version } from "./types";

version satisfies string;
```

```ts:types.d.ts
export const version: string;
```

## index modules

### resolves directory index module

> Directory specifiers resolve `index.ds` modules.

```ds:main.ds
import { value } from "./dir";

value satisfies string;
```

```ds:dir/index.ds
export const value: string = "ok";
```

## package metadata

### resolves package types entry

> Package types entries resolve type-only imports for bare specifiers.

```ts:main.ts
import type { User } from "spec";

const value: User = { name: "Ada" };
value.name satisfies string;
```

```ts:types.d.ts
export interface User {
    name: string;
}
```

```json:package.json
{
  "name": "spec",
  "types": "./types.d.ts"
}
```

### resolves package exports types entry

> Package exports map types entries for subpath imports.

```ts:main.ts
import { value } from "spec/feature";

value satisfies number;
```

```ts:feature.d.ts
export const value: number;
```

```json:package.json
{
  "name": "spec",
  "exports": {
    "./feature": {
      "types": "./feature.d.ts",
      "default": "./feature.js"
    }
  }
}
```

## package self references

### rejects self package bare import without exports

> Bare self package imports require explicit `exports` and do not fallback to `main`.

```ts:main.ts
import { value } from "spec";
```

```ts:index.ts
export const value = 1;
```

```json:package.json
{
  "name": "spec",
  "main": "./index.ts"
}
```

- contains: unresolved module 'spec'

### rejects self package subpath import without exports

> Subpath self imports require explicit `exports` mappings.

```ts:main.ts
import { feature } from "spec/feature";
```

```ts:feature.ts
export const feature = "ok";
```

```json:package.json
{
  "name": "spec"
}
```

- contains: unresolved module 'spec/feature'

### resolves self package bare import through exports when present

> Packages with exports resolve self package root imports through `exports`.

```ts:main.ts
import { value } from "spec";

value satisfies number;
```

```ts:index.ts
export const value = 0;
```

```ts:public.ts
export const value = 1;
```

```json:package.json
{
  "name": "spec",
  "main": "./index.ts",
  "exports": {
    ".": "./public.ts"
  }
}
```

### does not fallback to package main when exports are present

> Packages with exports keep exports constraints and do not fallback to package main for root imports.

```ts:main.ts
import "spec";
```

```ts:index.ts
export const value = 1;
```

```ts:feature.ts
export const feature = 1;
```

```json:package.json
{
  "name": "spec",
  "main": "./index.ts",
  "exports": {
    "./feature": "./feature.ts"
  }
}
```

- contains: unresolved module 'spec'
