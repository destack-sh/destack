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

### resolves inferred module without extension

> Extensionless specifiers resolve `.ds` modules with inferred exports.

```ds:main.ds
import { value } from "./mod";

value satisfies number;
```

```ds:mod.ds
export const value = 1;
```

### resolves declaration modules without extension

> Extensionless specifiers resolve `.ds` modules for type usage.

```ds:main.ds
import type { User } from "./types";

type Alias = User;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:types.ds
export interface User {
    name: string;
}
```

### resolves declaration value exports

> Extensionless specifiers resolve declared values from `.ds` modules.

```ds:main.ds
import { version } from "./types";

version satisfies string;
```

```ds:types.ds
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
