# Template Literal Inference

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

- contains: type 1 is not assignable to type segment<<type>>

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

- contains: type "value" is not assignable to type strip<<type>>

### template literal infer from any yields union

> `any` produces the union of both branches.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : "no";

let ok: Strip<any> = "no";
let ok2: Strip<any> = "value";
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

### template literal infer rejects non string from any

> `any` results still require string values.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : "no";

let bad: Strip<any> = 1;
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

- contains: type 1 is not assignable to type strip<<type>>

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

- contains: type "value" is not assignable to type strip<<type>>

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

- contains: type "c" is not assignable to type result

### template literal infer merges repeated spans

> Repeated `infer` bindings merge inferred candidates.

```ds
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";

let ok: Repeat<"foo-foo"> = "foo";
```

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

- contains: type "foo" is not assignable to type repeat<<type>>

### template literal infer rejects non matching union member

> Unmatched union branches do not contribute to the inferred type.

```ds
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `bar-b`>;

let bad: Result = "b";
```

- contains: type "b" is not assignable to type result

### template literal infer rejects non string result

> Conditional infer rejects values outside the inferred span.

```ds
type Strip<T> = T extends `prefix-${infer A}` ? A : never;
type Result = Strip<`prefix-${string}`>;

let bad: Result = 1;
```

- contains: type 1 is not assignable to type result

### template literal infers from call arguments

> Generic inference can flow through template literal parameters.

```ds
declare function take<T>(value: `${T}`): T;

let value = take("hello");
value satisfies "hello";
```

### template literal infers constrained number literal

> Numeric spans infer literal numbers when canonical.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let ok = parse("42");
ok satisfies 42;
```

### template literal infers constrained bigint literal

> Bigint spans infer literal bigints when canonical.

```ds
declare function parse<T extends bigint>(value: `${T}`): T;

let ok = parse("42");
ok satisfies 42n;
```

### template literal rejects invalid bigint string for bigint span

> Bigint spans reject non literal strings.

```ds
declare function parse<T extends bigint>(value: `${T}`): T;

let bad = parse("+1");
```

- contains: type "+1" is not assignable to type `${bigint}`

### template literal infers constrained int literal

> Fixed width int spans infer literal ints when canonical.

```ds
declare function parse<T extends int32>(value: `${T}`): T;

let ok = parse("42");
ok satisfies 42;
```

### template literal rejects out of range int span

> Fixed width int spans reject out of range strings.

```ds
declare function parse<T extends int8>(value: `${T}`): T;

let bad = parse("128");
```

- contains: type "128" is not assignable to type `${int8}`

### template literal rejects non numeric string for number span

> Numeric spans reject strings that do not parse as numbers.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let bad = parse("no");
```

- contains: type "no" is not assignable to type `${number}`

### template literal infers non canonical number span as number

> Non canonical numeric strings infer to the number primitive.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let nonCanonical = parse("1e3");
let ok: number = nonCanonical;
```

### template literal infers non canonical number span rejects literal assignment

> Non canonical numeric strings are not inferred as literals.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let nonCanonical = parse("1e3");
let bad: 1000 = nonCanonical;
```

- contains: not assignable to type 1000

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

- contains: type ("foo-bar", "baz") is not assignable to type pair<<type>>

### template literal infer requires non empty spans

> Adjacent spans capture at least one character when possible.

```ds
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;

let ok: Split<"a"> = ("a", "");
```

### template literal infer rejects empty first span

> Adjacent spans reject empty leading matches.

```ds
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;

let bad: Split<"a"> = ("", "a");
```

- contains: type ("", "a") is not assignable to type split<<type>>

### template literal infers from template literal arguments

> Template literal arguments can infer span types.

```ds
declare function take<T extends string>(value: `prefix-${T}`): T;

declare let value: `prefix-${"a" | "b"}`;
let result = take(value);
result satisfies "a" | "b";
```

### template literal infers from template literal arguments rejects narrowed result

> Inferred spans preserve union members.

```ds
declare function take<T extends string>(value: `prefix-${T}`): T;

declare let value: `prefix-${"a" | "b"}`;
let result = take(value);
let bad: "a" = result;
```

- contains: not assignable to type "a"

### template literal infers from template literal parameters

> Template literal arguments flow into generic spans.

```ds
declare function takeAny<T extends string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
result satisfies `prefix-${"a"}`;
```

### template literal infers from template literal parameters rejects narrowed result

> Inference preserves the full template literal shape.

```ds
declare function takeAny<T extends string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
let bad: "a" = result;
```

- contains: not assignable to type "a"

### template literal infers constrained bigint literal

> Bigint spans infer literal bigints when canonical.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let ok = parseBig("-1");
ok satisfies -1n;
```

### template literal infers non canonical bigint as bigint

> Non canonical bigint strings infer to the bigint primitive.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let nonCanonical = parseBig("0x1");
let ok: bigint = nonCanonical;
```

### template literal infers non canonical bigint rejects literal assignment

> Non canonical bigint strings are not inferred as literals.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let nonCanonical = parseBig("0x1");
let bad: 1n = nonCanonical;
```

- contains: not assignable to type 1n

### template literal rejects invalid bigint string

> Invalid bigint strings reject inference.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let bad = parseBig("01");
let bad2 = parseBig("+1");
```

### template literal rejects invalid bigint string leading zeros

> Leading zeros are rejected for bigint inference.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let bad = parseBig("01");
```

- contains: type "01" is not assignable to type `${bigint}`

### template literal rejects invalid bigint string plus sign

> Plus signs are rejected for bigint inference.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let bad = parseBig("+1");
```

- contains: type "+1" is not assignable to type `${bigint}`
