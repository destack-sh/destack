# Dynamic Parameters

Dynamic parameter passing and defaults.

## dynamic parameters

### positional arguments map to parameters

> Positional arguments flow into parameters by order.

```ds
function sum(a: number, b: number): number {
    return a + b;
}

const value = sum(1, 2);
value satisfies number;
```

### defaulted parameters are optional

> Parameters with defaults can be omitted at call sites.

```ds
function greet(name: string = "hi"): string {
    return name;
}

greet() satisfies string;
greet("hello") satisfies string;
```

### defaulted parameter types are enforced

> Arguments still satisfy the declared parameter type.

```ds
function repeat(value: string = "hi"): string {
    return value;
}

repeat("ok");
repeat(1);
```

- contains: is not assignable

### tuple rest parameters infer element types

> Tuple rest parameters preserve element types from call sites.

```ds
function pair<T, U>(...values: (T, U)): (T, U) {
    return (values[0], values[1]);
}

const result = pair(1, "hi");
result satisfies (int, string);
```

### comptime dynamic parameters accept literal arguments

> Dynamic parameters marked `comptime` accept compile-time-known literal arguments.

```ds
function sized(comptime width: int32): int32 {
    return width;
}

const width = sized(4);
width satisfies int32;
```

### comptime dynamic parameters accept const bindings with static initializers

> Dynamic `comptime` parameters accept const bindings when the initializer is static.

```ds
const WIDTH = 4;

function sized(comptime width: int32): int32 {
    return width;
}

const width = sized(WIDTH);
width satisfies int32;
```

### comptime dynamic parameters reject mutable runtime bindings

> Dynamic `comptime` parameters reject mutable runtime bindings.

```ds
let width = 4;

function sized(comptime value: int32): int32 {
    return value;
}

sized(width);
```

- contains: static expression

### tuple rest destructuring supports nested defaults

> Tuple rest destructuring supports nested object defaults and a tuple level fallback.

```ts:main.ts
type SpawnArguments = [string, { syncSnapshot?: boolean }?];

function spawnChild(...[src, { syncSnapshot = false } = {}]: SpawnArguments): boolean {
    return syncSnapshot;
}
```

- named fields are not allowed in array or tuple patterns


## invalid parameter properties

### function parameters cannot be parameter properties

> Parameter property modifiers are only allowed in constructors.

```ds
function build(public value: number) {
}
```

- contains: parameter property

## invalid optional parameters

### optional pattern parameters are rejected in `.ts` sources

> Optional parameters must use identifiers, not binding patterns.

```ts:main.ts
interface Payload {
    value: string;
}

function handle({ value }?: Payload) {
    let _ = value;
}
```

- optional parameters cannot use binding patterns

### optional rest parameters are rejected in `.ts` sources

> Rest parameters cannot be optional.

```ts:main.ts
function collect(...items?: string[]) {
    let _ = items;
}
```

- optional rest parameters are not allowed

## this parameters

### this parameters shape call contexts

> Explicit this parameters enforce call context types.

```ds
function log(this: { prefix: string }, value: string): string {
    return this.prefix + value;
}

log.call({ prefix: ">" }, "ok") satisfies string;
```
