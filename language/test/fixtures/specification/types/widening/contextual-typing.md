# Contextual Typing and Best Common Type

Widening happens when values are bound, not during type-level evaluation.

## Array and tuple literals

### arrays widen element literals in let bindings

> Let bindings commit array element literals to widened element types.

```ds
let values = [1, 2];

values[0] satisfies number;
```

### arrays do not preserve literal element types in let bindings

> Let bindings do not retain literal element unions for array elements.

```ds
let values = [1, 2];

let union: 1 | 2 = values[0];
```

- contains: not assignable

### arrays widen object literal members in let bindings

> Let bindings widen object literal members in array literals.

```ds
let values = [{ kind: "a" }, { kind: "b" }];

values[0].kind satisfies string;
```

### arrays do not keep object literal members as literals in let bindings

> Let bindings do not retain literal member types for array object literals.

```ds
let values = [{ kind: "a" }, { kind: "b" }];

values[0].kind satisfies "a";
```

- contains: expected "a"

### const arrays still widen elements without const assertions

> Const bindings alone does not preserve literal element types.

```ds
const values = [1, 2];

values[0] satisfies number;
```

### const arrays do not preserve literal element types without const assertions

> Const bindings alone does not retain literal element unions for arrays.

```ds
const values = [1, 2];

let union: 1 | 2 = values[0];
```

- contains: not assignable

### contextual arrays preserve union element types

> Contextual element types prevent widening beyond the context.

```ds
const values: (1 | 2)[] = [1, 2];

values[0] satisfies 1 | 2;
```

### contextual arrays do not narrow to a single literal

> Contextual array element unions do not narrow to a single literal.

```ds
const values: (1 | 2)[] = [1, 2];

values[0] satisfies 1;
```

- contains: expected 1

## Conditional expressions

### let conditionals commit to widened types

> Let bindings widen conditional literal unions when no context constrains them.

```ds
let value = true ? 1 : 2;

value satisfies number;
```

### let conditionals do not preserve literal unions

> Let conditional results do not retain literal unions without context.

```ds
let value = true ? 1 : 2;

let union: 1 | 2 = value;
```

- contains: not assignable

### let conditionals widen object literal members

> Let conditional results widen object literal members without context.

```ds
let value = true ? { mode: "dev" } : { mode: "prod" };

value.mode satisfies string;
```

### let conditionals do not keep object members as literals

> Let conditional results do not keep object member literals.

```ds
let value = true ? { mode: "dev" } : { mode: "prod" };

value.mode satisfies "dev";
```

- contains: expected "dev"

### const conditionals preserve literal unions

> Const bindings retain literal unions for conditional expressions.

```ds
const value = true ? 1 : 2;

value satisfies 1 | 2;
```

## Contextual object literals

### annotations constrain object literal fields

> Annotations contextualize object literal fields without changing the annotation.

```ds
type Mode = "dev" | "prod";

const config: { mode: Mode } = { mode: "dev" };

config.mode satisfies Mode;
```

### annotations do not narrow object literal fields to a single literal

> Annotations do not narrow object literal fields to a specific literal.

```ds
type Mode = "dev" | "prod";

const config: { mode: Mode } = { mode: "dev" };

config.mode satisfies "dev";
```

- contains: expected "dev"

### contextual arrays preserve object literal unions

> Contextual object element types prevent widening of object member unions.

```ds
const values: { kind: "a" | "b" }[] = [{ kind: "a" }, { kind: "b" }];

values[0].kind satisfies "a" | "b";
```

### contextual arrays do not narrow object literal unions

> Contextual object member unions do not narrow to a single literal.

```ds
const values: { kind: "a" | "b" }[] = [{ kind: "a" }, { kind: "b" }];

values[0].kind satisfies "a";
```

- contains: expected "a"

## Function returns

### function bodies do not inherit outer const contexts

> Outer const bindings do not freeze literals inside function bodies.

```ds
const make = () => ({ mode: "dev" });

make().mode satisfies string;
```

### function bodies do not preserve literal members by default

> Function bodies do not preserve literal members without const assertions.

```ds
const make = () => ({ mode: "dev" });

make().mode satisfies "dev";
```

- contains: expected "dev"

### contextual return types constrain object literal members

> Declared return types contextualize object literal members.

```ds
type Mode = "dev" | "prod";

const make = (): { mode: Mode } => ({ mode: "dev" });

make().mode satisfies Mode;
```

### contextual return types do not narrow to a single literal

> Return type annotations do not narrow object members to a single literal.

```ds
type Mode = "dev" | "prod";

const make = (): { mode: Mode } => ({ mode: "dev" });

make().mode satisfies "dev";
```

- contains: expected "dev"

## Assignment freshness boundaries

### fresh object literals enforce excess checks at commitment

> Fresh object literals enforce excess property checks at annotated binding sites.

```ts
type Named = { name: string };

const value: Named = { name: "Ada", extra: true };
```

- contains: excess property

### non-fresh objects skip excess checks at later commitments

> Non-fresh object values do not re-run excess checks at later assignment points.

```ts
type Named = { name: string };

const source = { name: "Ada", extra: true };
const value: Named = source;
```

## Contextual generics

### explicit generic unions constrain object literal members

> Explicit generic unions flow into object literal members.

```ds
function wrap<T>(value: T): { value: T } {
    return { value };
}

const result = wrap<"dev" | "prod">("dev");

result.value satisfies "dev" | "prod";
```

### explicit generic unions do not narrow to a single literal

> Explicit generic unions do not narrow object literal members to a single literal.

```ds
function wrap<T>(value: T): { value: T } {
    return { value };
}

const result = wrap<"dev" | "prod">("dev");

result.value satisfies "dev";
```

- contains: expected "dev"

### method calls keep explicit generic unions

> Method calls keep explicit generic unions.

```ds
type Wrapper = {
    wrap<T>(value: T): { value: T };
};

declare const wrapper: Wrapper;

const result = wrapper.wrap<"dev" | "prod">("dev");

result.value satisfies "dev" | "prod";
```

### explicit generic unions on methods do not narrow

> Explicit generic unions do not narrow object literal members to a single literal.

```ds
type Wrapper = {
    wrap<T>(value: T): { value: T };
};

declare const wrapper: Wrapper;

const result = wrapper.wrap<"dev" | "prod">("dev");

result.value satisfies "dev";
```

- contains: expected "dev"

### return type annotations keep union members

> Return type annotations with unions preserve the union shape.

```ds
type Mode = "dev" | "prod";

function make(mode: Mode): { mode: Mode } {
    return { mode };
}

const config = make("dev");

config.mode satisfies Mode;
```

### nested return annotations preserve unions

> Nested object literals keep return type annotations.

```ds
type Mode = "dev" | "prod";

function make(mode: Mode): { inner: { mode: Mode } } {
    return { inner: { mode } };
}

const config = make("dev");

config.inner.mode satisfies Mode;
```

### nested return annotations do not narrow to a single literal

> Nested return annotations do not narrow to a single literal.

```ds
type Mode = "dev" | "prod";

function make(mode: Mode): { inner: { mode: Mode } } {
    return { inner: { mode } };
}

const config = make("dev");

config.inner.mode satisfies "dev";
```

- contains: expected "dev"

### return type annotations do not narrow to a single literal

> Return type annotations do not narrow object members to a single literal.

```ds
type Mode = "dev" | "prod";

function make(mode: Mode): { mode: Mode } {
    return { mode };
}

const config = make("dev");

config.mode satisfies "dev";
```

- contains: expected "dev"

## Contextual tuples

### contextual tuples preserve literal element types

> Contextual tuple element types preserve literal values.

```ds
const pair: (1, 2) = (1, 2);

pair[0] satisfies 1;
pair[1] satisfies 2;
```

### contextual tuples do not narrow to a single literal when widened

> Contextual tuple elements typed as primitives do not keep literal values.

```ds
const pair: (number, number) = (1, 2);

pair[0] satisfies 1;
```

- contains: expected 1

## Mixed literal arrays

### arrays widen mixed literals to a common union

> Mixed literal arrays widen to a union element type.

```ds
let values = [1, "a"];

values[0] satisfies number | string;
```

### arrays do not narrow mixed literals to a single literal

> Mixed literal arrays are not assignable to a single literal type.

```ds
let values = [1, "a"];

values[0] satisfies 1;
```

- contains: expected 1
