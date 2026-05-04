# Namespace Reexports

## export star namespaces

### export star namespace exposes type aliases

A namespace produced by `export * as` exposes exported type aliases.

```ts:types.ts
export type Segment<T extends string> = T extends `id:${infer U}` ? U : never;
```

```ts:namespace.ts
export * as api from "./types";
```

```ts:main.ts
import { api } from "./namespace";

declare const segment: api.Segment<"id:users">;
segment satisfies "users";
```

### renamed namespace reexport exposes values

Renaming an exported namespace keeps its value exports available.

```ts:values.ts
export const version = 1 as const;
```

```ts:namespace.ts
export * as api from "./values";
```

```ts:index.ts
export { api as renamed } from "./namespace";
```

```ts:main.ts
import { renamed } from "./index";

renamed.version satisfies 1;
```

### type-only reexports do not create values

A `type` re-export must stay type-only and must not synthesize runtime namespace values.

```ts:types.ts
export type Item = { id: string };
```

```ts:index.ts
export type { Item } from "./types";
```

```ts:main.ts
import { Item } from "./index";

Item;
```

- contains: type-only symbol cannot be used as a value

### namespace aliases keep type checks

Template-literal constraints still reject non-matching inputs through namespace imports.

```ts:types.ts
export type Segment<T extends string> = T extends `id:${infer U}` ? U : never;
```

```ts:namespace.ts
export * as api from "./types";
```

```ts:main.ts
import { api } from "./namespace";

declare const segment: api.Segment<"users">;
segment satisfies "users";
```

- contains: not assignable

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

## namespace imports

### namespace imports keep value shapes

Namespace imports keep imported value shapes.

```ts:exports.ts
export const value = { ok: true };
```

```ts:module-b.ts
import * as mod from "./exports";

export const forwarded = mod.value;
```

```ts:main.ts
import { forwarded } from "./module-b";

forwarded.ok satisfies boolean;
```
