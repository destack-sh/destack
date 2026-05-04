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

### import type accesses re-exported type aliases

> Import types can resolve type aliases through re-export modules.

```ts:main.ts
type Alias = import("./index").User;

const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ts:user.ts
export type User = { name: string };
```

```ts:index.ts
export type { User } from "./user";
```

### import type accesses exported generic aliases

> Import types can instantiate generic aliases from imported modules.

```ts:main.ts
type Alias = import("./mod").Box<string>;

const value: Alias = { value: "Ada" };
value.value satisfies string;
```

```ts:mod.ts
export type Box<T> = { value: T };
```

### import type handles mixed local and re-exported members from one target

> Import type member lookups are order independent when one target mixes local exports and re-exported exports.
> Reusing the same import target must still resolve transitive re-export members.

```ts:main.ts
type LocalAlias = import("./index").Local;
type UserAlias = import("./index").User;

const local: LocalAlias = { id: 1 };
local.id satisfies number;

const user: UserAlias = { name: "Ada" };
user.name satisfies string;
```

```ts:user.ts
export type User = { name: string };
```

```ts:index.ts
export type Local = { id: number };
export type { User } from "./user";
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
