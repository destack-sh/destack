# Inference

## defaults

Parameter and return type inference.

### default values infer parameters

Parameters are inferred from default values.

```ds
function greet(name = "hi") {
    return name
}
greet satisfies (name: string) => string;
```

## contextual typing

### contextual lambda from annotation

Lambda parameter types are inferred from annotations.

```ds
const add: (a: number, b: number) => number = (a, b) => a + b
add satisfies (a: number, b: number) => number;
```

### contextual lambda from annotation mismatch

Lambda return type must satisfy the contextual return type.

```ds
const add: (a: number, b: number) => number = (a, b) => "hi"
```

- contains: not assignable

### contextual lambda from argument

Lambda parameter types are inferred from parameter types.

```ds
function apply(transform: (value: number) => number) {
    return transform(1)
}
apply((value) => value + 1)
```

### contextual lambda from argument mismatch

Lambda return type must satisfy the contextual return type.

```ds
function apply(transform: (value: number) => number) {
    return transform(1)
}
apply((value) => "hi")
```

- contains: not assignable

### contextual object argument

Object literals use parameter types for contextual typing.

```ds
function use_point(point: { x: number, y: number }) {
    return point.x
}
use_point({ x: 1, y: 2 })
```

### contextual object argument mismatch

Object literal properties must satisfy contextual field types.

```ds
function use_point(point: { x: number, y: number }) {
    return point.x
}
use_point({ x: 1, y: "hi" })
```

- contains: not assignable

### contextual tuple argument

Tuple literals use parameter types for contextual typing.

```ds
function sum(pair: (number, number)) {
    return pair
}
sum((1, 2))
```

### contextual tuple argument mismatch

Tuple literal elements must satisfy contextual element types.

```ds
function sum(pair: (number, number)) {
    return pair
}
sum((1, "hi"))
```

- contains: not assignable

### contextual array argument

Array literals use parameter types for contextual typing.

```ds
function total(values: number[]) {
    return values
}
total([1, 2, 3])
```

### contextual array argument mismatch

Array literal elements must satisfy contextual element types.

```ds
function total(values: number[]) {
    return values
}
total([1, "hi"])
```

- contains: not assignable

## object literal callbacks

### arrow properties remain order-insensitive for contextual callback inference

Arrow properties infer contextual callback types regardless of sibling order.

```ds:main.ds
declare function callIt<T>(obj: {
    produce: (x: number) => T,
    consume: (y: T) => void,
}): void;

callIt({
    consume: y => y.toFixed(),
    produce: (x: number) => x * 2,
});
```

### object methods do not infer from sibling members when flipped

Method syntax does not infer parameter types from sibling members.

```ds:main.ds
declare function callIt<T>(obj: {
    produce: (x: number) => T,
    consume: (y: T) => void,
}): void;

callIt({
    consume(y) { return y.toFixed(); },
    produce(x: number) { return x * 2; },
});
```

- contains: unknown

## nested object callbacks

### arrow properties infer across sibling ordering with nested object literals

Arrow properties contextually infer generic payloads regardless of sibling ordering.

```ds:main.ds
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

### object methods do not contextually infer sibling generic payloads

Method syntax does not gain arrow-style sibling contextual inference in object literals.

```ds:main.ds
declare function build<T>(spec: {
    payload: () => T,
    consume: (value: T) => string,
}): string;

build({
    consume(value) { return value.toUpperCase(); },
    payload() { return "ready"; },
});
```

- contains: unknown

### arrow callbacks preserve inference through renamed re-exports

Renamed re-exports do not affect arrow contextual inference.

```ds:api.ds
export declare function build<T>(spec: {
    payload: () => T,
    consume: (value: T) => string,
}): string;
```

```ds:index.ds
export { build as make } from "./api.ds";
```

```ds:main.ds
import { make } from "./index.ds";

const output = make({
    consume: value => value.toUpperCase(),
    payload: () => "ready",
});

output satisfies string;
```

## generic callback precision

### generic callback inference preserves const tuple literal element types

Generic callback inference preserves tuple literal precision from const tuple arguments.

```ds:main.ds
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

const tuple = [1, 2] as const;
const head = mapOne(tuple, input => input[0]);

head satisfies 1;
```

### generic callback inference widens mutable array element types

Generic callback inference widens mutable array element reads to their primitive element type.

```ds:main.ds
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

let values = [1, 2];
const head = mapOne(values, input => input[0]);

head satisfies number;
```

### generic callback inference does not keep mutable array literal elements

Generic callback inference does not preserve mutable array element literal types.

```ds:main.ds
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

let values = [1, 2];
const head = mapOne(values, input => input[0]);

head satisfies 1;
```

- contains: not assignable

### generic callback inference stays precise through renamed re-exports

Generic callback inference preserves const tuple precision across renamed re-export paths.

```ds:api.ds
export declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;
```

```ds:index.ds
export { mapOne as runOne } from "./api.ds";
```

```ds:main.ds
import { runOne } from "./index.ds";

const tuple = [1, 2] as const;
const head = runOne(tuple, input => input[0]);

head satisfies 1;
```

## nested contextual inference

### nested callback inference threads outer generic payloads

Nested callbacks preserve contextual generic payload types through outer callback positions.

```ds:main.ds
declare function withValue<T>(
    value: T,
    callback: (read: () => T) => string,
): string;

const output = withValue("ready", read => read());
output satisfies string;
```

### nested callback inference rejects mismatched payload usage

Nested callbacks reject payload usage that is incompatible with the inferred contextual generic type.

```ds:main.ds
declare function withValue<T>(
    value: T,
    callback: (read: () => T) => string,
): string;

withValue("ready", read => read().toFixed());
```

- contains: tofixed

### nested arrows remain order-insensitive in contextual object inference

Nested arrow properties remain order-insensitive for contextual generic object inference.

```ds:main.ds
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

### nested methods do not infer sibling payloads

Nested method syntax does not gain arrow-style sibling contextual inference.

```ds:main.ds
declare function wire<T>(spec: {
    make: () => { value: T },
    use: (input: { value: T }) => string,
}): string;

wire({
    use(input) { return input.value.toUpperCase(); },
    make() { return { value: "ready" }; },
});
```

- contains: unknown
