# Reexports

Reexports expose the binding shape from the module that declares it.

## re-exports

### named reexports keep value shapes

Reexport chains keep the original value shape.

```ds:a.ds
export const value = { ok: true };
```

```ds:b.ds
export { value } from "./a.ds";
```

```ds:main.ds
import { value } from "./b.ds";

value.ok satisfies boolean;
```

### multi-hop reexports keep value shapes

Multi-hop reexports keep the original value shape.

```ds:a.ds
export const value = { ok: true };
```

```ds:b.ds
export { value } from "./a.ds";
```

```ds:c.ds
export { value } from "./b.ds";
```

```ds:main.ds
import { value } from "./c.ds";

value.ok satisfies boolean;
```

### export star preserves type-only exports

> Export star reexports types for type positions without runtime values.

```ds:types.ds
export type User = { name: string };
export const value = 1;
```

```ds:mod.ds
export * from "./types.ds";
```

```ds:main.ds
import { value, User } from "./mod.ds";

value satisfies number;
type Alias = User;
User;
```

- contains: value

### renamed reexports keep value shapes

Renamed reexports keep the original value shape.

```ds:a.ds
export const value = { ok: true };
```

```ds:b.ds
export { value as renamed } from "./a.ds";
```

```ds:main.ds
import { renamed } from "./b.ds";

renamed.ok satisfies boolean;
```

### export star keeps value shapes

Export star keeps exported value shapes.

```ds:a.ds
export const value = { ok: true };
```

```ds:b.ds
export * from "./a.ds";
```

```ds:main.ds
import { value } from "./b.ds";

value.ok satisfies boolean;
```

### export type reexports remain type-only

> Export type reexports do not create runtime values.

```ds:types.ds
export type User = { name: string };
```

```ds:mod.ds
export type { User } from "./types.ds";
```

```ds:main.ds
import { User } from "./mod.ds";

type Alias = User;
User;
```

- contains: value

### export type specifiers remain type-only

> Named export type specifiers do not produce runtime values.

```ds:types.ds
export type User = { name: string };
```

```ds:mod.ds
export { type User } from "./types.ds";
```

```ds:main.ds
import { User } from "./mod.ds";

type Alias = User;
User;
```

- contains: value

### export type star reexports types only

> Export type star reexports types without runtime values.

```ds:types.ds
export type User = { name: string };
```

```ds:mod.ds
export type * from "./types.ds";
```

```ds:main.ds
import { User } from "./mod.ds";

type Alias = User;
User;
```

- contains: value

### export type star rejects runtime value usage

> Value usage from export type star imports stays type-only.

```ds:types.ds
export interface User {
    name: string;
}
```

```ds:mod.ds
export type * from "./types.ds";
```

```ds:main.ds
import { User } from "./mod.ds";

const value = User;
```

- contains: value

### namespace reexports keep value shapes

Namespace reexports keep exported value shapes.

```ds:a.ds
export const value = { ok: true };
```

```ds:b.ds
export * as ns from "./a.ds";
```

```ds:main.ds
import { ns } from "./b.ds";

ns.value.ok satisfies boolean;
```

### default reexports keep value shapes

Default reexports keep exported value shapes.

```ds:defaults.ds
export default function make() {
    return { ok: true };
}
```

```ds:reexport.ds
export { default as make } from "./defaults.ds";
```

```ds:main.ds
import { make } from "./reexport.ds";

make().ok satisfies boolean;
```

