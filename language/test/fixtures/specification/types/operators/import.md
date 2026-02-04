# Import Types

## import type queries

### import type accesses exported types

> Import types can reference exported type aliases.

```ts:main.ts
type Alias = import("./mod").User;

const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ts:mod.ts
export type User = { name: string };
```

### import type accesses default exports

> Import types can access default exports via `.default`.

```ts:main.ts
type Default = import("./mod").default;

const value: Default = { name: "Ada" };
value.name satisfies string;
```

```ts:mod.ts
export default interface User {
    name: string;
}
```

### import type rejects missing members

> Missing members in import types are rejected.

```ts:main.ts
type Alias = import("./mod").Missing;
```

```ts:mod.ts
export type User = { name: string };
```

- contains: does not exist
