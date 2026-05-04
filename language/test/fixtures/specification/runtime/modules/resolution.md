# Module Resolution

Extensionless and index-based module resolution.

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

> Extensionless specifiers resolve `.ts` modules from `.ts` sources.

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

