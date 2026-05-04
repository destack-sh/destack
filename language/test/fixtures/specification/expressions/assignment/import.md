# Import

Imported bindings are immutable.

## imports

### imported bindings are immutable

> Imported bindings cannot be reassigned.

```ts:counter.ts
export let counter: number = 0;
```

```ts:main.ts
import { counter } from "./counter";

counter = 1;
```

- contains: immutable binding

### imported bindings are immutable with aliases

> Imported bindings remain immutable when renamed.

```ts:counter.ts
export let counter: number = 0;
```

```ts:main.ts
import { counter as localCounter } from "./counter";

localCounter = 1;
```

- contains: immutable binding

### imported namespace bindings are immutable

> Namespace imports cannot be reassigned.

```ts:counter.ts
export let counter: number = 0;
```

```ts:main.ts
import * as counter from "./counter";

counter = { counter: 1 };
```

- contains: immutable binding

### imported namespace members are immutable

> Namespace import members are immutable aliases of exported bindings.

```ts:counter.ts
export let counter: number = 0;
```

```ts:main.ts
import * as namespaceCounter from "./counter";

namespaceCounter.counter = 1;
```

- contains: immutable binding

### imported bindings reject update expressions

> Imported bindings cannot be mutated through update expressions.

```ts:counter.ts
export let counter: number = 0;
```

```ts:main.ts
import { counter } from "./counter";

counter++;
```

- contains: immutable binding
