# Optional

Optional chaining propagates `undefined` through property access, element access, and calls.

## optional property access

### optional property access returns undefined for nullish bases

> Optional property access yields undefined when the base is nullish.

```ds
type User = { name: string };

declare const user: User | null;

const name = user?.name;
name satisfies string | undefined;
```

### optional property access requires known properties

> Optional chaining still enforces property existence.

```ds
type User = { name: string };

declare const user: User | null;

user?.missing;
```

- contains: does not exist

## optional element access

### optional element access returns undefined for nullish bases

> Optional element access yields undefined when the base is nullish.

```ds
type Bag = { [key: string]: number };

declare const bag: Bag | undefined;

const value = bag?.["count"];
value satisfies number | undefined;
```

## optional call

### optional call returns undefined for nullish functions

> Optional calls yield undefined when the callee is nullish.

```ds
declare const handler: ((value: number) => string) | undefined;

const result = handler?.(1);
result satisfies string | undefined;
```

### optional call validates argument types

> Optional calls still enforce parameter types.

```ds
declare const handler: ((value: number) => string) | undefined;

handler?.("bad");
```

- contains: not assignable

### optional call rejects non callable values

> Optional calls still require callable targets.

```ds
declare const value: { name: string } | undefined;

value?.();
```

- contains: calling non-callable

## optional chain composition

### optional chains propagate undefined

> Chained optional access preserves undefined in the result.

```ds
type User = { name?: { length: number } };

declare const user: User | null;

const length = user?.name?.length;
length satisfies number | undefined;
```
