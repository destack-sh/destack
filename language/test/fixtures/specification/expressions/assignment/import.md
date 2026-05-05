# Import

Imported bindings are immutable.

## imports

### imported bindings are immutable

> Imported bindings cannot be reassigned.

```ds:counter.ds
export let counter: number = 0;
```

```ds:main.ds
import { counter } from "./counter.ds";

counter = 1;
```

- contains: immutable binding

### imported aliases are immutable

> Renaming an import does not make it assignable.

```ds:counter.ds
export let counter: number = 0;
```

```ds:main.ds
import { counter as localCounter } from "./counter.ds";

localCounter = 1;
```

- contains: immutable binding

### imported namespace bindings are immutable

> Namespace imports cannot be reassigned.

```ds:counter.ds
export let counter: number = 0;
```

```ds:main.ds
import * as counter from "./counter.ds";

counter = { counter: 1 };
```

- contains: immutable binding

### imported namespace members are immutable

> Namespace import members are immutable aliases of exported bindings.

```ds:counter.ds
export let counter: number = 0;
```

```ds:main.ds
import * as namespaceCounter from "./counter.ds";

namespaceCounter.counter = 1;
```

- contains: immutable binding

### imported bindings reject updates

> Imported bindings cannot be updated.

```ds:counter.ds
export let counter: number = 0;
```

```ds:main.ds
import { counter } from "./counter.ds";

counter++;
```

- contains: immutable binding
