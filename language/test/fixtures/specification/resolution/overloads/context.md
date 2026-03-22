# Overload Contextual Typing

Overload selection should include contextual typing against each candidate.
When multiple candidates apply, declaration order should still win.

## Lambdas

### contextual typing follows overload declaration order

> Contextual lambda typing should be evaluated per overload candidate in order.

```ds
function apply(transform: (value: number) => number): "number" {
    return "number";
}

function apply(transform: (value: string) => string): "string" {
    return "string";
}

const selected = apply((value) => value);
selected satisfies "number";
```

### contextual typing does not select later overloads

> Later overloads should not win when earlier overloads are applicable.

```ds
function apply(transform: (value: number) => number): "number" {
    return "number";
}

function apply(transform: (value: string) => string): "string" {
    return "string";
}

const selected = apply((value) => value);
selected satisfies "string";
```

- not assignable

## Optional and rest parameters

### optional parameters do not override earlier overloads

> Earlier overloads should win even when later overloads are compatible via optional parameters.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(value: number, other?: number): "two" {
    return "two";
}

const selected = pick(1);
selected satisfies "one";
```

### optional parameters do not select later overloads

> Later optional overloads should not win when earlier overloads apply.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(value: number, other?: number): "two" {
    return "two";
}

const selected = pick(1);
selected satisfies "two";
```

- not assignable

### rest parameters do not override earlier overloads

> Rest parameters should not override earlier overloads when both can apply.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(...values: number[]): "many" {
    return "many";
}

const selected = pick(1);
selected satisfies "one";
```

### rest parameters do not select later overloads

> Later rest overloads should not win when earlier overloads apply.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(...values: number[]): "many" {
    return "many";
}

const selected = pick(1);
selected satisfies "many";
```

- not assignable

## Context-sensitive callbacks

### callback object contextual typing still follows overload declaration order

> Context-sensitive callback objects should be typed per candidate in declaration order.

```ts:main.ts
declare function choose<T>(spec: {
    produce: () => T,
    consume: (value: T) => void,
}): "generic-first";

declare function choose(spec: {
    produce: () => string,
    consume: (value: string) => void,
}): "string-second";

const selected = choose({
    consume: value => value.toUpperCase(),
    produce: () => "ok",
});

selected satisfies "generic-first";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### callback object contextual typing does not select later candidates

> Later callback candidates should not win when an earlier candidate applies.

```ts:main.ts
declare function choose<T>(spec: {
    produce: () => T,
    consume: (value: T) => void,
}): "generic-first";

declare function choose(spec: {
    produce: () => string,
    consume: (value: string) => void,
}): "string-second";

const selected = choose({
    consume: value => value.toUpperCase(),
    produce: () => "ok",
});

selected satisfies "string-second";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- not assignable

### rest callback candidates can shadow single-argument candidates when first

> Rest callback candidates declared first should shadow later single-argument callbacks.

```ds
function choose(callback: (...values: number[]) => number): "rest" {
    return "rest";
}

function choose(callback: (value: number) => number): "single" {
    return "single";
}

const selected = choose(_value => 1);
selected satisfies "rest";
```

### callback object overload order remains stable through renamed re-exports

> Renamed re-export paths should not perturb overload declaration-order contextual typing.

```ts:api.ts
export declare function choose<T>(spec: {
    produce: () => T,
    consume: (value: T) => void,
}): "generic-first";

export declare function choose(spec: {
    produce: () => string,
    consume: (value: string) => void,
}): "string-second";
```

```ts:index.ts
export { choose as pick } from "./api";
```

```ts:main.ts
import { pick } from "./index";

const selected = pick({
    consume: value => value.toUpperCase(),
    produce: () => "ok",
});

selected satisfies "generic-first";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

### callback object overload order through re-exports still rejects later candidates

> Later contextual callback candidates should not win through renamed re-export paths.

```ts:api.ts
export declare function choose<T>(spec: {
    produce: () => T,
    consume: (value: T) => void,
}): "generic-first";

export declare function choose(spec: {
    produce: () => string,
    consume: (value: string) => void,
}): "string-second";
```

```ts:index.ts
export { choose as pick } from "./api";
```

```ts:main.ts
import { pick } from "./index";

const selected = pick({
    consume: value => value.toUpperCase(),
    produce: () => "ok",
});

selected satisfies "string-second";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "lib": ["es5"] } }
```

- not assignable
