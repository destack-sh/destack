# Conditional Types

Conditional types are static type expressions.

## branches

### matching types choose the true branch

`extends` picks the true branch on a match.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<string> = "ok";
```

### non-matching types choose the false branch

A failed match picks the false branch.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<int32> = 1;
```

### true branches reject false branch values

The selected branch is the only branch.

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<string> = 1;
```

- contains: not assignable

### false branches reject true branch values

Selection works in both directions.

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<int32> = "no";
```

- contains: not assignable

## distribution

### naked type parameters distribute over unions

A bare parameter checks each union arm separately.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let ok: OnlyStrings<string | int32> = "ok";
```

### distribution filters rejected union members

Arms that fail the check disappear.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let bad: OnlyStrings<string | int32> = 1;
```

- contains: not assignable

### tuple wrapping disables distribution

Wrapping the parameter checks the union as one type.

```ds
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";

let ok: Wrapped<string | int32> = "no";
```

### never distributes to never

Distribution over nothing produces nothing.

```ds
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let bad: Result = "no";
```

- contains: not assignable

### unknown chooses the false branch for concrete targets

`unknown` only extends `unknown`.

```ds
type Select<T> = T extends string ? "yes" : "no";

let ok: Select<unknown> = "no";
```

## infer

### infer extracts matching members

`infer` binds the matched component.

```ds
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const ok: Unbox<Box<"ready">> = "ready";
```

### infer rejects unrelated extracted values

The extracted type is exact.

```ds
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const bad: Unbox<Box<"ready">> = "no";
```

- contains: not assignable

### infer distributes over unions

Each distributed arm infers its own binding.

```ds
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const first: Unbox<Box<"a"> | Box<"b">> = "a";
const second: Unbox<Box<"a"> | Box<"b">> = "b";
```

### infer extracts function arguments

Parameter positions infer across overloaded shapes.

```ds
type Argument<T> = T extends (value: infer A) => unknown ? A : never;

type Input = Argument<((value: string) => void) | ((value: number) => void)>;

const first: Input = "ok";
const second: Input = 1;
```

### inferred argument unions reject unrelated values

The inferred union stays closed.

```ds
type Argument<T> = T extends (value: infer A) => unknown ? A : never;

type Input = Argument<((value: string) => void) | ((value: number) => void)>;

const bad: Input = false;
```

- contains: not assignable
