# Goto Declaration

## Local Bindings

### Declaration for a local value

Goto declaration should jump from a local use site to its binding declaration.

```ds:main.ds
const /*declare_def*/value = 1;
const output = /*declare_use*/value;
```

### Declaration for an imported value

Goto declaration should stop at the local import binding for imported values.

```ds:lib.ds
export function ping(): void {}
```

```ds:main.ds
import { /*declare_def*/ping } from "./lib.ds";

const output = /*declare_use*/ping;
```

### Declaration for an aliased import

Goto declaration should stop at the local alias when the import uses `as`.

```ds:lib.ds
export function ping(): void {}
```

```ds:main.ds
import { ping as /*declare_def*/localPing } from "./lib.ds";

const output = /*declare_use*/localPing;
```

### Declaration for a type-only import

Goto declaration should stop at the local type-only import binding.

```ds:types.ds
export struct Point {
    value: int32
}
```

```ds:main.ds
import type { /*declare_def*/Point } from "./types.ds";

const current: /*declare_use*/Point = Point { value: 1 };
```

### Declaration for a namespace import

Goto declaration should stop at the namespace alias in the importing file.

```ds:lib.ds
export function ping(): void {}
```

```ds:main.ds
import * as /*declare_def*/api from "./lib.ds";

/*declare_use*/api.ping();
```
