# Conditional Types

Conditional types are static type expressions.

## branches

### matching types choose the true branch

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<string> = "ok";
```

### non-matching types choose the false branch

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<int32> = 1;
```

### true branches reject false branch values

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<string> = 1;
```

- contains: not assignable

### false branches reject true branch values

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<int32> = "no";
```

- contains: not assignable

## distribution

### naked type parameters distribute over unions

```ds
type OnlyStrings<T> = T extends string ? T : never;

let ok: OnlyStrings<string | int32> = "ok";
```

### distribution filters rejected union members

```ds
type OnlyStrings<T> = T extends string ? T : never;

let bad: OnlyStrings<string | int32> = 1;
```

- contains: not assignable

### tuple wrapping disables distribution

```ds
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";

let ok: Wrapped<string | int32> = "no";
```

### never distributes to never

```ds
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let bad: Result = "no";
```

- contains: not assignable

### unknown chooses the false branch for concrete targets

```ds
type Select<T> = T extends string ? "yes" : "no";

let ok: Select<unknown> = "no";
```

## infer

### infer extracts matching members

```ds
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const ok: Unbox<Box<"ready">> = "ready";
```

### infer rejects unrelated extracted values

```ds
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const bad: Unbox<Box<"ready">> = "no";
```

- contains: not assignable

### infer distributes over unions

```ds
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const first: Unbox<Box<"a"> | Box<"b">> = "a";
const second: Unbox<Box<"a"> | Box<"b">> = "b";
```

### infer extracts function arguments

```ds
type Argument<T> = T extends (value: infer A) => unknown ? A : never;

type Input = Argument<((value: string) => void) | ((value: number) => void)>;

const first: Input = "ok";
const second: Input = 1;
```

### inferred argument unions reject unrelated values

```ds
type Argument<T> = T extends (value: infer A) => unknown ? A : never;

type Input = Argument<((value: string) => void) | ((value: number) => void)>;

const bad: Input = false;
```

- contains: not assignable
