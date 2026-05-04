# Reexports

Reexports expose the binding shape from the module that declares it.

## re-exports

### named reexports keep value shapes

Reexport chains keep the original value shape.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export { value } from "./a";
```

```ts:main.ts
import { value } from "./b";

value.ok satisfies boolean;
```

### multi-hop reexports keep value shapes

Multi-hop reexports keep the original value shape.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export { value } from "./a";
```

```ts:c.ts
export { value } from "./b";
```

```ts:main.ts
import { value } from "./c";

value.ok satisfies boolean;
```

### export star preserves type-only exports

> Export star reexports types for type positions without runtime values.

```ts:types.ts
export type User = { name: string };
export const value = 1;
```

```ts:mod.ts
export * from "./types";
```

```ts:main.ts
import { value, User } from "./mod";

value satisfies number;
type Alias = User;
User;
```

- contains: value

### renamed reexports keep value shapes

Renamed reexports keep the original value shape.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export { value as renamed } from "./a";
```

```ts:main.ts
import { renamed } from "./b";

renamed.ok satisfies boolean;
```

### export star keeps value shapes

Export star keeps exported value shapes.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export * from "./a";
```

```ts:main.ts
import { value } from "./b";

value.ok satisfies boolean;
```

### export type reexports remain type-only

> Export type reexports do not create runtime values.

```ts:types.ts
export type User = { name: string };
```

```ts:mod.ts
export type { User } from "./types";
```

```ts:main.ts
import { User } from "./mod";

type Alias = User;
User;
```

- contains: value

### export type specifiers remain type-only

> Named export type specifiers do not produce runtime values.

```ts:types.ts
export type User = { name: string };
```

```ts:mod.ts
export { type User } from "./types";
```

```ts:main.ts
import { User } from "./mod";

type Alias = User;
User;
```

- contains: value

### export type star reexports types only

> Export type star reexports types without runtime values.

```ts:types.ts
export type User = { name: string };
```

```ts:mod.ts
export type * from "./types";
```

```ts:main.ts
import { User } from "./mod";

type Alias = User;
User;
```

- contains: value

### export type star rejects runtime value usage

> Value usage from export type star imports stays type-only.

```ts:types.ts
export interface User {
    name: string;
}
```

```ts:mod.ts
export type * from "./types";
```

```ts:main.ts
import { User } from "./mod";

const value = User;
```

- contains: value

### namespace reexports keep value shapes

Namespace reexports keep exported value shapes.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export * as ns from "./a";
```

```ts:main.ts
import { ns } from "./b";

ns.value.ok satisfies boolean;
```

### default reexports keep value shapes

Default reexports keep exported value shapes.

```ts:defaults.ts
export default function make() {
    return { ok: true };
}
```

```ts:reexport.ts
export { default as make } from "./defaults";
```

```ts:main.ts
import { make } from "./reexport";

make().ok satisfies boolean;
```

