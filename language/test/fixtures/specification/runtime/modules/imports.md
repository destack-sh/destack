# Imports

Imports bind module exports into the local scope.

## type-only imports

### type-only imports cannot mix default and named bindings

Type-only imports must not mix default and named bindings.

```ds:main.ds
import type Foo, { Bar } from "./mod.ds";
```

```ds:mod.ds
export default class Foo {}
export type Bar = string;
```

- contains: type-only imports cannot mix default and named bindings

### type-only imports cannot be used as values

Type-only imports are erased and cannot be used as runtime values.

```ds:main.ds
import type { Foo } from "./mod.ds";

Foo;
```

```ds:mod.ds
export type Foo = { name: string };
```

- contains: type-only

### named type-only imports are allowed

Named type-only specifiers are permitted in value imports.

```ds:main.ds
import { type Foo } from "./mod.ds";

type Alias = Foo;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:mod.ds
export type Foo = { name: string };
```

### named type-only imports cannot be used as values

Type-only specifiers are still erased at runtime.

```ds:main.ds
import { type Foo } from "./mod.ds";

Foo;
```

```ds:mod.ds
export type Foo = { name: string };
```

- contains: type-only

### default type-only imports are allowed

Default type-only imports can be used in type positions.

```ds:main.ds
import type Foo from "./mod.ds";

type Alias = Foo;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:mod.ds
export default interface Foo {
    name: string;
}
```

### default type-only imports cannot be used as values

Default type-only imports are erased at runtime.

```ds:main.ds
import type Foo from "./mod.ds";

Foo;
```

```ds:mod.ds
export default interface Foo {
    name: string;
}
```

- contains: type-only

### namespace type-only imports are allowed

Namespace type-only imports expose types only.

```ds:main.ds
import type * as Types from "./mod.ds";

type Alias = Types.User;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:mod.ds
export interface User {
    name: string;
}
```

### namespace type-only imports are not runtime values

Namespace type-only imports cannot be used as values.

```ds:main.ds
import type * as Types from "./mod.ds";

Types;
```

```ds:mod.ds
export interface User {
    name: string;
}
```

- contains: type-only

### value imports allow type-only exports in type positions

Value imports may reference type-only exports in type positions.

```ds:main.ds
import { Foo } from "./mod.ds";

type Alias = Foo;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ds:mod.ds
export type Foo = { name: string };
```

### value imports of type-only exports are not runtime values

Imported types cannot be referenced as values even without import type.

```ds:main.ds
import { Foo } from "./mod.ds";

Foo;
```

```ds:mod.ds
export type Foo = { name: string };
```

- contains: value

### imported generic constraints are enforced at call sites

Imported generic function constraints remain active at consumer call sites.

```ds:lib.ds
export function readName<T: { name: string }>(value: T): string {
    return value.name;
}
```

```ds:main.ds
import { readName } from "./lib.ds";

readName({ name: "ok" });
readName({ name: 1 });
```

- contains: not assignable
