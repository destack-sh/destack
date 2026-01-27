# Assignment

Assignment tests cover binding mutability and assignment validity.

## Immutable bindings

### const bindings reject assignment

> Const bindings cannot be reassigned.

```ds
const value: number = 1;
value = 2;
```

- contains: immutable binding

### const bindings reject compound assignment

> Const bindings cannot use compound assignment operators.

```ds
const value: number = 1;
value += 1;
```

- contains: immutable binding

### const destructuring rejects assignment

> Const bindings created from destructuring are immutable.

```ds
const { count }: { count: number } = { count: 0 };
count = 1;
```

- contains: immutable binding

### const bindings allow member assignment

> Const bindings do not freeze object members.

```ds
const state: { count: number } = { count: 0 };
state.count = 1;
state.count satisfies number;
```

## Imported bindings

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

## Mutable bindings

### let bindings allow assignment

> Let bindings may be reassigned.

```ds
let value: number = 1;
value = 2;
value satisfies number;
```
