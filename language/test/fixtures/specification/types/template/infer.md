# Template Infer

## inference

### template literal infer extracts span from string literal

> Conditional infer can extract spans from template literals.

```ds
type Segment<T> = T extends `/${infer Name}` ? Name : never;

let ok: Segment<"/api"> = "api";
```

### template literal infer rejects mismatched inferred type

> Inferred spans must satisfy their resulting type.

```ds
type Segment<T> = T extends `/${infer Name}` ? Name : never;

let bad: Segment<"/api"> = 1;
```

- type 1 is not assignable to type Segment<"/api">

### template literal infer extracts span from template literal type

> Conditional infer can extract spans from template literal types.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : never;
type Result = Strip<`prefix-${string}`>;

let ok: Result = "value";
```

### template literal infer falls back for string

> Non literal `string` does not match template literal patterns.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : "no";

let ok: Strip<string> = "no";
```

### template literal infer rejects non else branch for string

> Non literal `string` rejects the true branch.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : "no";

let bad: Strip<string> = "value";
```

- type "value" is not assignable to type Strip<string>

### template literal infer falls back for unknown

> `unknown` does not match template literal patterns.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : "no";

let ok: Strip<unknown> = "no";
```

### template literal infer rejects non else branch for unknown

> `unknown` rejects the true branch.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : "no";

let bad: Strip<unknown> = "value";
```

- type "value" is not assignable to type Strip<unknown>

### template literal infer distributes over union templates

> Conditional infer distributes over union template literals.

```ds
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `foo-b`>;

let ok: Result = "a";
let ok2: Result = "b";
```

### template literal infer extracts union span members

> Inference preserves union spans inside template literals.

```ds
type Extract<T> = T extends `id-${infer A}` ? A : never;
type Result = Extract<`id-${"a" | "b"}`>;

let ok: Result = "a";
let ok2: Result = "b";
```

### template literal infer rejects non member from union span

> Union spans reject values outside the inferred union.

```ds
type Extract<T> = T extends `id-${infer A}` ? A : never;
type Result = Extract<`id-${"a" | "b"}`>;

let bad: Result = "c";
```

- type "c" is not assignable to type result

### template literal infer merges repeated spans

> Repeated `infer` bindings merge inferred candidates.

```ds
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";

let ok: Repeat<"foo-foo"> = "foo";
```

### template literal infer honors constrained spans

> Constrained inference uses the else branch when constraints fail.

```ds
type Extract<T> = T extends `id-${infer A extends "a" | "b"}` ? A : "no";

let ok: Extract<"id-a"> = "a";
let ok2: Extract<"id-c"> = "no";
```

### template literal infer rejects constrained span mismatch

> Constrained inference rejects values outside the constraint.

```ds
type Extract<T> = T extends `id-${infer A extends "a" | "b"}` ? A : "no";

let bad: Extract<"id-c"> = "c";
```

- type "c" is not assignable to type Extract<"id-c">

### template literal infer falls back for mismatched repeated spans

> Repeated spans fall back to the else branch when they differ.

```ds
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";

let ok: Repeat<"foo-bar"> = "no";
```

### template literal infer rejects mismatched repeated spans

> Repeated spans must match the same substring.

```ds
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";

let bad: Repeat<"foo-bar"> = "foo";
```

- type "foo" is not assignable to type Repeat<"foo-bar">

### template literal infer rejects non matching union member

> Unmatched union branches do not contribute to the inferred type.

```ds
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `bar-b`>;

let bad: Result = "b";
```

- type "b" is not assignable to type result

### template literal infer rejects non string result

> Conditional infer rejects values outside the inferred span.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : never;
type Result = Strip<"prefix-hello">;

let bad: Result = 1;
```

- type 1 is not assignable to type result

### template literal infer splits on first literal

> Inference splits on the earliest matching literal.

```ds
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;

let ok: Pair<"foo-bar-baz"> = ("foo", "bar-baz");
```

### template literal infer rejects later split

> Later literal splits do not satisfy the inferred tuple.

```ds
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;

let bad: Pair<"foo-bar-baz"> = ("foo-bar", "baz");
```

- type ("foo-bar", "baz") is not assignable to type Pair<"foo-bar-baz">

### template literal infer requires non empty spans

> Adjacent spans capture at least one character when possible.

```ds
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;

let ok: Split<"a"> = ("a", "");
```

### template literal infer allows empty spans with literal boundary

> Literal boundaries allow empty captures.

```ds
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;

let ok: Split<"a"> = ("", "");
```

### template literal infer rejects empty first span

> Adjacent spans reject empty leading matches.

```ds
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;

let bad: Split<"a"> = ("", "a");
```

- type ("", "a") is not assignable to type Split<"a">
