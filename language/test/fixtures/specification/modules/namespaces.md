# Namespace Reexports

Namespace re-exports group a module's surface under one name.

## export star namespaces

### export star namespace exposes type aliases

A namespace produced by `export * as` exposes exported type aliases.

```ds:types.ds
export type Segment<T: string> = T extends `id:${infer U}` ? U : never;
```

```ds:namespace.ds
export * as api from "./types.ds";
```

```ds:main.ds
import { api } from "./namespace.ds";

declare const segment: api.Segment<"id:users">;
segment satisfies "users";
```

### renamed namespace reexport exposes values

Renaming an exported namespace keeps its value exports available.

```ds:values.ds
export const version = 1 as const;
```

```ds:namespace.ds
export * as api from "./values.ds";
```

```ds:index.ds
export { api as renamed } from "./namespace.ds";
```

```ds:main.ds
import { renamed } from "./index.ds";

renamed.version satisfies 1;
```

### type-only reexports do not create values

A `type` re-export must stay type-only and must not synthesize runtime namespace values.

```ds:types.ds
export type Item = { id: string };
```

```ds:index.ds
export type { Item } from "./types.ds";
```

```ds:main.ds
import { Item } from "./index.ds";

Item;
```

- contains: type-only symbol cannot be used as a value

### namespace aliases keep type checks

Template-literal constraints still reject non-matching inputs through namespace imports.

```ds:types.ds
export type Segment<T: string> = T extends `id:${infer U}` ? U : never;
```

```ds:namespace.ds
export * as api from "./types.ds";
```

```ds:main.ds
import { api } from "./namespace.ds";

declare const segment: api.Segment<"users">;
segment satisfies "users";
```

- contains: not assignable

## namespace imports

### namespace imports keep value shapes

Namespace imports keep imported value shapes.

```ds:exports.ds
export const value = { ok: true };
```

```ds:module-b.ds
import * as mod from "./exports.ds";

export const forwarded = mod.value;
```

```ds:main.ds
import { forwarded } from "./module-b.ds";

forwarded.ok satisfies boolean;
```
