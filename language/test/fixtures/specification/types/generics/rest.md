# Rest Inference

## partial application

### partial application preserves tail tuple order

> Variadic partial application infers the remaining tail tuple in original parameter order.

```ds
declare function partial<T extends readonly unknown[], U extends readonly unknown[], R>(
    fn: (...args: [...T, ...U]) => R,
    ...head: T
): (...tail: U) => R;

declare function join(a: "a", b: 1, c: true): "ok";

const tail = partial(join, "a");
const value = tail(1, true);
value satisfies "ok";
```

### partial application rejects reordered tail arguments

> A partially applied function rejects calls that provide the inferred tail arguments in the wrong order.

```ds
declare function partial<T extends readonly unknown[], U extends readonly unknown[], R>(
    fn: (...args: [...T, ...U]) => R,
    ...head: T
): (...tail: U) => R;

declare function join(a: "a", b: 1, c: true): "ok";

const tail = partial(join, "a");
tail(true, 1);
```

- contains: not assignable

## rest inference

### const rest inference preserves literal tuple elements

> Rest inference from const tuple inputs keeps literal element types.

```ds
declare function collect<const T extends readonly unknown[]>(...values: T): T;

const value = collect("x", 1, true);
value[0] satisfies "x";
value[1] satisfies 1;
value[2] satisfies true;
```

### mutable rest inference widens scalar tuple elements

> Rest inference from mutable arrays widens scalar literals to their mutable primitive counterparts.

```ds
declare function collect<T extends readonly unknown[]>(...values: T): T;

let first = "x";
const value = collect(first, 1, true);
value[0] satisfies "x";
```

- contains: not assignable
