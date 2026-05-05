# Import Types

## import type queries

### import type accesses exported types

```ds:main.ds
type Alias = import("./mod.ds").User;

const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:mod.ds
export type User = { name: string };
```

### import type accesses default exports

```ds:main.ds
type Default = import("./mod.ds").default;

const value: Default = { name: "Ada" };
value.name satisfies string;
```

```ds:mod.ds
export default interface User {
    name: string;
}
```

### import type accesses re-exported type aliases

```ds:main.ds
type Alias = import("./index.ds").User;

const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:user.ds
export type User = { name: string };
```

```ds:index.ds
export type { User } from "./user.ds";
```

### import type accesses exported generic aliases

```ds:main.ds
type Alias = import("./mod.ds").Box<string>;

const value: Alias = { value: "Ada" };
value.value satisfies string;
```

```ds:mod.ds
export type Box<T> = { value: T };
```

### import type resolves local and re-exported members
```ds:main.ds
type LocalAlias = import("./index.ds").Local;
type UserAlias = import("./index.ds").User;

const local: LocalAlias = { id: 1 };
local.id satisfies number;

const user: UserAlias = { name: "Ada" };
user.name satisfies string;
```

```ds:user.ds
export type User = { name: string };
```

```ds:index.ds
export type Local = { id: number };
export type { User } from "./user.ds";
```

### import type rejects missing members

```ds:main.ds
type Alias = import("./mod.ds").Missing;
```

```ds:mod.ds
export type User = { name: string };
```

- contains: does not exist
