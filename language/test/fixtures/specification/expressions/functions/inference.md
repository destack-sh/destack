# Inference

## defaults

Parameter and return type inference.

### default values infer parameters

> Parameters are inferred from default values.

```ds
function greet(name = "hi") {
    return name
}
greet satisfies (name: string) => string;
```

## contextual typing

### contextual lambda from annotation

> Lambda parameter types are inferred from annotations.

```ds
const add: (a: number, b: number) => number = (a, b) => a + b
add satisfies (a: number, b: number) => number;
```

### contextual lambda from annotation mismatch

> Lambda return type must satisfy the contextual return type.

```ds
const add: (a: number, b: number) => number = (a, b) => "hi"
```

- type "hi" is not assignable to type number

### contextual lambda from argument

> Lambda parameter types are inferred from parameter types.

```ds
function apply(transform: (value: number) => number) {
    return transform(1)
}
apply((value) => value + 1)
```

### contextual lambda from argument mismatch

> Lambda return type must satisfy the contextual return type.

```ds
function apply(transform: (value: number) => number) {
    return transform(1)
}
apply((value) => "hi")
```

- type "hi" is not assignable to type number

### contextual object argument

> Object literals use parameter types for contextual typing.

```ds
function use_point(point: { x: number, y: number }) {
    return point.x
}
use_point({ x: 1, y: 2 })
```

### contextual object argument mismatch

> Object literal properties must satisfy contextual field types.

```ds
function use_point(point: { x: number, y: number }) {
    return point.x
}
use_point({ x: 1, y: "hi" })
```

- type { x: number, y: "hi" } is not assignable to type { x: number, y: number }

### contextual tuple argument

> Tuple literals use parameter types for contextual typing.

```ds
function sum(pair: (number, number)) {
    return pair
}
sum((1, 2))
```

### contextual tuple argument mismatch

> Tuple literal elements must satisfy contextual element types.

```ds
function sum(pair: (number, number)) {
    return pair
}
sum((1, "hi"))
```

- type (number, "hi") is not assignable to type (number, number)

### contextual array argument

> Array literals use parameter types for contextual typing.

```ds
function total(values: number[]) {
    return values
}
total([1, 2, 3])
```

### contextual array argument mismatch

> Array literal elements must satisfy contextual element types.

```ds
function total(values: number[]) {
    return values
}
total([1, "hi"])
```

- type (number | "hi")[] is not assignable to type number[]

## object literal callbacks

### arrow properties remain order-insensitive for contextual callback inference

> Arrow properties infer contextual callback types regardless of sibling order.

```ts:main.ts
declare function callIt<T>(obj: {
    produce: (x: number) => T,
    consume: (y: T) => void,
}): void;

callIt({
    consume: y => y.toFixed(),
    produce: (x: number) => x * 2,
});
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### this-less methods do not infer from sibling members when flipped

> Method syntax does not infer parameter types from sibling members.

```ts:main.ts
declare function callIt<T>(obj: {
    produce: (x: number) => T,
    consume: (y: T) => void,
}): void;

callIt({
    consume(y) { return y.toFixed(); },
    produce(x: number) { return x * 2; },
});
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- contains: unknown

## nested object callbacks

### this-less arrows infer across sibling ordering with nested object literals

> Arrow properties contextuallies infer generic payloads regardless of sibling ordering.

```ts:main.ts
declare function build<T>(spec: {
    payload: () => T,
    consume: (value: T) => string,
}): string;

const output = build({
    consume: value => value.toUpperCase(),
    payload: () => "ready",
});

output satisfies string;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### this-less methods do not contextually infer sibling generic payloads

> Method syntax does not gain arrow-style sibling contextual inference in this-less object literals.

```ts:main.ts
declare function build<T>(spec: {
    payload: () => T,
    consume: (value: T) => string,
}): string;

build({
    consume(value) { return value.toUpperCase(); },
    payload() { return "ready"; },
});
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- contains: unknown

### this-less arrow callbacks preserve inference through renamed re-exports

> Renamed re-exports do not affect this-less arrow contextual inference.

```ts:api.ts
export declare function build<T>(spec: {
    payload: () => T,
    consume: (value: T) => string,
}): string;
```

```ts:index.ts
export { build as make } from "./api";
```

```ts:main.ts
import { make } from "./index";

const output = make({
    consume: value => value.toUpperCase(),
    payload: () => "ready",
});

output satisfies string;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

## generic callback precision

### generic callback inference preserves const tuple literal element types

> Generic callback inference preserves tuple literal precision from const tuple arguments.

```ts:main.ts
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

const tuple = [1, 2] as const;
const head = mapOne(tuple, input => input[0]);

head satisfies 1;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### generic callback inference widens mutable array element types

> Generic callback inference widens mutable array element reads to their primitive element type.

```ts:main.ts
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

let values = [1, 2];
const head = mapOne(values, input => input[0]);

head satisfies number;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### generic callback inference does not keep mutable array literal elements

> Generic callback inference does not preserve mutable array element literal types.

```ts:main.ts
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

let values = [1, 2];
const head = mapOne(values, input => input[0]);

head satisfies 1;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- contains: not assignable

### generic callback inference stays precise through renamed re-exports

> Generic callback inference preserves const tuple precision across renamed re-export paths.

```ts:api.ts
export declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;
```

```ts:index.ts
export { mapOne as runOne } from "./api";
```

```ts:main.ts
import { runOne } from "./index";

const tuple = [1, 2] as const;
const head = runOne(tuple, input => input[0]);

head satisfies 1;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

## nested contextual inference

### nested callback inference threads outer generic payloads

> Nested callbacks preserve contextual generic payload types through outer callback positions.

```ts:main.ts
declare function withValue<T>(
    value: T,
    callback: (read: () => T) => string,
): string;

const output = withValue("ready", read => read());
output satisfies string;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### nested callback inference rejects mismatched payload usage

> Nested callbacks reject payload usage that is incompatible with the inferred contextual generic type.

```ts:main.ts
declare function withValue<T>(
    value: T,
    callback: (read: () => T) => string,
): string;

withValue("ready", read => read().toFixed());
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- contains: tofixed

### nested this-less arrows remain order-insensitive in contextual object inference

> Nested this-less arrow properties remain order-insensitive for contextual generic object inference.

```ts:main.ts
declare function wire<T>(spec: {
    make: () => { value: T },
    use: (input: { value: T }) => string,
}): string;

const output = wire({
    use: input => {
        input.value satisfies string;
        return input.value;
    },
    make: () => ({ value: "ready" }),
});

output satisfies string;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### nested this-less methods do not infer sibling payloads

> Nested this-less method syntax does not gain arrow-style sibling contextual inference.

```ts:main.ts
declare function wire<T>(spec: {
    make: () => { value: T },
    use: (input: { value: T }) => string,
}): string;

wire({
    use(input) { return input.value.toUpperCase(); },
    make() { return { value: "ready" }; },
});
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- contains: unknown
