# Module Routing

## export star and namespace bridges

### export star as namespace preserves nested type aliases

A namespace produced by `export * as` should retain nested type aliases through the re-export boundary.

```ts:types.ts
export type Segment<T extends string> = T extends `id:${infer U}` ? U : never;
```

```ts:bridge.ts
export * as api from "./types";
```

```ts:main.ts
import { api } from "./bridge";

declare const segment: api.Segment<"id:users">;
segment satisfies "users";
```

### renamed export star namespace preserves value exports

Renaming an `export * as` namespace should still preserve value-side members when imported.

```ts:values.ts
export const version = 1 as const;
```

```ts:bridge.ts
export * as api from "./values";
```

```ts:index.ts
export { api as renamed } from "./bridge";
```

```ts:main.ts
import { renamed } from "./index";

renamed.version satisfies 1;
```

### type only re exports do not leak runtime values

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

### namespace bridges preserve conditional type rejection on non matches

Template-conditional constraints routed through namespace bridges should reject inputs that miss literal structure.

```ts:types.ts
export type Segment<T extends string> = T extends `id:${infer U}` ? U : never;
```

```ts:bridge.ts
export * as api from "./types";
```

```ts:main.ts
import { api } from "./bridge";

declare const segment: api.Segment<"users">;
segment satisfies "users";
```

- contains: not assignable

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```
