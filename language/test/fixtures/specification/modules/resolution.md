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
